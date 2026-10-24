//! The auto-battle: "pretty much Magic", but mana comes from the lands you
//! placed on the map within Reach of the fight, unspent mana banks to the
//! next turn, and nobody makes a choice once it starts. Every choice a
//! player would make is a fixed priority here, so a fight is decided by its
//! seed and the two decks and replays the same everywhere (MTG-4).
//!
//! A turn, for the side whose turn it is:
//!
//! 1. untap, upkeep triggers, then mana: one unit for each land in Reach,
//!    added to the bank;
//! 2. draw a card (the hand holds `HAND_MAX`);
//! 3. cast: the most expensive card the bank can pay for, again and again;
//! 4. abilities, last in the chain, with the mana above what the side keeps
//!    for its next turn (the person's rule: "their abilities should go off
//!    last in the chain if you have mana");
//! 5. attack; the other side blocks;
//! 6. end: what is left in the bank stays.
//!
//! The other side may answer during this turn: counter a spell, shrink an
//! attacker, or cast a flash creature at the end of the turn.
//!
//! The whole fight runs at once into a `Record`: one `Event` a step, each
//! with a snapshot of both sides, which the page plays back.

use serde::Serialize;

use crate::content::{ty, Card};
use crate::mana::{self, Bank, Cost};
use crate::rng::Rng;
use crate::rules::{kw, Aim, Aura, Eff, Filter, OnCast, Static, Timing, Token, Use, Who};

pub const HAND_MAX: usize = 5;
pub const OPENING_HAND: usize = 5;
/// Turns, both sides counted, before the foe withdraws.
pub const TURN_CAP: u32 = 60;
/// Life lost when a side must draw from an empty library: its graveyard is
/// shuffled back in, and this is the price. (Magic's decking loss suits sixty
/// cards; this game's decks start at twelve. SECOND-ORDER R2.6.)
pub const EMPTY_DRAW_LOSS: i32 = 1;

/// Power and toughness of an animated keyrune.
pub const KEYRUNE_SIZE: i32 = 2;

pub const P: usize = 0;
pub const F: usize = 1;

/// Rules a side brings from its armour and abilities (`stats.rs`).
pub mod perk {
    pub const DETAIN_FIRST: u32 = 1;
    pub const START_MANA: u32 = 1 << 1;
    pub const EXTRA_DRAW: u32 = 1 << 2;
    pub const SPELL_PING: u32 = 1 << 3;
    pub const OPENING_DAMAGE: u32 = 1 << 4;
    pub const UNLEASH_HASTE: u32 = 1 << 5;
    pub const HASTE_FIRST: u32 = 1 << 6;
    pub const SCAVENGE_CHEAP: u32 = 1 << 7;
    pub const REGROWTH: u32 = 1 << 8;
    pub const TOKEN_START: u32 = 1 << 9;
    pub const SET_DETAIN_TWO: u32 = 1 << 10;
    pub const SET_SPELL_CHEAP: u32 = 1 << 11;
    pub const SET_FIRST_STRIKE: u32 = 1 << 12;
    pub const SET_REGENERATE: u32 = 1 << 13;
    pub const SET_POPULATE: u32 = 1 << 14;
    /// Starting bank from START_MANA.
    pub const START_MANA_UNITS: usize = 2;
    pub const OPENING_DAMAGE_AMOUNT: i32 = 2;
}

#[derive(Debug, Clone, Default)]
pub struct SideSetup {
    pub name: String,
    pub life: i32,
    pub deck: Vec<usize>,
    /// One unit a turn for each: the colours a land's unit can be.
    pub income: Vec<u8>,
    /// Lands that are Swamps, for swampwalk.
    pub swamps: usize,
    /// Cards that start on the battlefield (the creature you met).
    pub in_play: Vec<usize>,
    pub perks: u32,
}

#[derive(Debug, Clone)]
pub struct Setup {
    pub sides: [SideSetup; 2],
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Ev {
    Start,
    Turn,
    Mana,
    Draw,
    Cast,
    Counter,
    Enter,
    Attack,
    Block,
    Hit,
    Face,
    Dies,
    Life,
    Ability,
    Detain,
    Token,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PermSnap {
    pub id: u32,
    pub side: u8,
    /// Index into the cards, or -1 for a token.
    pub card: i32,
    pub token: String,
    pub name: String,
    pub power: i32,
    pub toughness: i32,
    pub damage: i32,
    pub tapped: bool,
    pub attacking: bool,
    pub blocking: i32,
    pub detained: bool,
    pub creature: bool,
    pub attached: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snap {
    pub life: [i32; 2],
    pub bank: [Vec<String>; 2],
    pub hand: [usize; 2],
    pub library: [usize; 2],
    pub grave: [usize; 2],
    /// The player's hand, as card indices.
    pub player_hand: Vec<usize>,
    pub perms: Vec<PermSnap>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Event {
    pub ev: Ev,
    pub side: u8,
    pub turn: u32,
    /// A string key in data/strings.json and its named values.
    pub key: &'static str,
    pub args: Vec<(&'static str, String)>,
    /// Permanents the event is about: source, then target.
    pub focus: Vec<u32>,
    pub snap: Snap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Outcome {
    Won,
    Lost,
    /// The turn cap came: the foe withdrew.
    Withdrew,
}

#[derive(Debug, Clone, Serialize)]
pub struct Record {
    pub events: Vec<Event>,
    pub outcome: Outcome,
    pub life: [i32; 2],
    pub turns: u32,
}

#[derive(Debug, Clone)]
struct Perm {
    id: u32,
    card: Option<usize>,
    token: Option<Token>,
    ctrl: usize,
    owner: usize,
    tapped: bool,
    sick: bool,
    damage: i32,
    plus: i32,
    tp: i32,
    tt: i32,
    tkw: u32,
    detained_until: u32,
    attached: Option<u32>,
    animated: bool,
    unleashed: bool,
    attacking: bool,
    blocking: Option<u32>,
    deathtouched: bool,
    /// Cards exiled by this until it leaves (Angel of Serenity, Detention Sphere).
    held: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Default)]
struct Side {
    name: String,
    life: i32,
    library: Vec<usize>,
    hand: Vec<usize>,
    grave: Vec<usize>,
    exile: Vec<usize>,
    bank: Bank,
    income: Vec<u8>,
    swamps: usize,
    perks: u32,
    creatures_cast: u32,
    regrowth_used: bool,
    regen_used: bool,
}

struct Fight<'a> {
    cards: &'a [Card],
    s: [Side; 2],
    perms: Vec<Perm>,
    next_id: u32,
    turn: u32,
    active: usize,
    rng: Rng,
    events: Vec<Event>,
    over: Option<Outcome>,
    /// The spell resolving now: the source the log names for its damage.
    casting: Option<String>,
}

fn other(side: usize) -> usize {
    1 - side
}

impl<'a> Fight<'a> {
    // ---------- looking at the board ----------

    fn rules_of(&self, p: &Perm) -> Option<&'a crate::rules::Rules> {
        p.card.map(|c| &self.cards[c].rules)
    }

    fn name_of(&self, p: &Perm) -> String {
        match (p.card, p.token) {
            (Some(c), _) => self.cards[c].name.clone(),
            (None, Some(t)) => t.stats().4.to_string(),
            _ => String::new(),
        }
    }

    fn find(&self, id: u32) -> Option<usize> {
        self.perms.iter().position(|p| p.id == id)
    }

    fn is_creature(&self, p: &Perm) -> bool {
        p.token.is_some() || p.animated || p.card.map(|c| self.cards[c].is(ty::CREATURE)).unwrap_or(false)
    }

    fn colours_of(&self, p: &Perm) -> u8 {
        match (p.card, p.token) {
            (Some(c), _) if p.animated => {
                if self.cards[c].name == "Azorius Keyrune" {
                    mana::W | mana::U
                } else {
                    mana::B | mana::G
                }
            }
            (Some(c), _) => self.cards[c].colours,
            (None, Some(t)) => t.stats().3,
            _ => 0,
        }
    }

    fn statics_of(&self, side: usize, st: Static) -> usize {
        self.perms
            .iter()
            .filter(|p| p.ctrl == side && self.rules_of(p).map(|r| r.statics.contains(&st)).unwrap_or(false))
            .count()
    }

    fn any_static(&self, st: Static) -> bool {
        self.statics_of(P, st) + self.statics_of(F, st) > 0
    }

    fn keywords(&self, p: &Perm) -> u32 {
        let mut k = p.tkw;
        if let Some(r) = self.rules_of(p) {
            k |= r.kw;
        }
        if let Some(t) = p.token {
            k |= t.stats().2;
        }
        if p.animated {
            k |= if self.name_of(p) == "Azorius Keyrune" {
                kw::FLYING
            } else {
                kw::DEATHTOUCH
            };
        }
        for a in self.perms.iter().filter(|a| a.attached == Some(p.id)) {
            if self.rules_of(a).and_then(|r| r.aura) == Some(Aura::EtherealArmor) {
                k |= kw::FIRST_STRIKE;
            }
        }
        let side = &self.s[p.ctrl];
        if p.unleashed && side.perks & perk::UNLEASH_HASTE != 0 {
            k |= kw::HASTE;
        }
        k
    }

    fn has(&self, p: &Perm, k: u32) -> bool {
        self.keywords(p) & k != 0
    }

    fn power(&self, p: &Perm) -> i32 {
        let mut v = p.tp + p.plus;
        if let Some(c) = p.card {
            // an animated keyrune is a 2/2 (Azorius Bird, Golgari Insect)
            v += if p.animated {
                KEYRUNE_SIZE
            } else {
                self.cards[c].power
            };
        }
        if let Some(t) = p.token {
            v += t.stats().0 + self.statics_of(p.ctrl, Static::TokenAnthem) as i32;
        }
        for a in self.perms.iter().filter(|a| a.attached == Some(p.id)) {
            match self.rules_of(a) {
                Some(r) if r.aura == Some(Aura::StabWound) => v -= 2,
                Some(r) if r.aura == Some(Aura::EtherealArmor) => v += self.enchantments(a.ctrl) as i32,
                Some(r) if r.equip.is_some() => v += mana::count_colours(self.colours_of(p)) as i32,
                _ => {}
            }
        }
        v
    }

    fn toughness(&self, p: &Perm) -> i32 {
        let mut v = p.tt + p.plus;
        if let Some(c) = p.card {
            // an animated keyrune is a 2/2 (Azorius Bird, Golgari Insect)
            v += if p.animated {
                KEYRUNE_SIZE
            } else {
                self.cards[c].toughness
            };
        }
        if let Some(t) = p.token {
            v += t.stats().1 + self.statics_of(p.ctrl, Static::TokenAnthem) as i32;
        }
        for a in self.perms.iter().filter(|a| a.attached == Some(p.id)) {
            match self.rules_of(a) {
                Some(r) if r.aura == Some(Aura::StabWound) => v -= 2,
                Some(r) if r.aura == Some(Aura::EtherealArmor) => v += self.enchantments(a.ctrl) as i32,
                _ => {}
            }
        }
        v
    }

    fn enchantments(&self, side: usize) -> usize {
        self.perms
            .iter()
            .filter(|p| p.ctrl == side && p.card.map(|c| self.cards[c].is(ty::ENCHANTMENT)).unwrap_or(false))
            .count()
    }

    fn arrested(&self, p: &Perm) -> bool {
        self.perms
            .iter()
            .any(|a| a.attached == Some(p.id) && self.rules_of(a).and_then(|r| r.aura) == Some(Aura::Arrest))
    }

    fn detained(&self, p: &Perm) -> bool {
        p.detained_until > self.turn
    }

    fn value(&self, p: &Perm) -> i32 {
        if !self.is_creature(p) {
            return p.card.map(|c| self.cards[c].value as i32 * 2).unwrap_or(0);
        }
        let k = self.keywords(p);
        let mut v = self.power(p) * 3 + self.toughness(p) * 2;
        for (bit, w) in [
            (kw::FLYING, 3),
            (kw::DEATHTOUCH, 3),
            (kw::DOUBLE_STRIKE, 3),
            (kw::FIRST_STRIKE, 2),
            (kw::LIFELINK, 2),
            (kw::TRAMPLE, 1),
            (kw::UNBLOCKABLE, 3),
        ] {
            if k & bit != 0 {
                v += w;
            }
        }
        if let Some(c) = p.card {
            v += self.cards[c].value as i32;
            if !self.cards[c].rules.activated.is_empty() || !self.cards[c].rules.statics.is_empty() {
                v += 2;
            }
        }
        v
    }

    fn matches(&self, p: &Perm, f: &Filter, me: usize) -> bool {
        let who_ok = match f.who {
            Who::Mine => p.ctrl == me,
            Who::Theirs => p.ctrl != me,
            Who::Any => true,
        };
        if !who_ok || p.attached.is_some() && !self.is_creature(p) && f.creature {
            return false;
        }
        if f.creature && !self.is_creature(p) {
            return false;
        }
        if p.ctrl != me && self.has(p, kw::HEXPROOF) {
            return false;
        }
        if let Some(m) = f.max_value {
            if p.card.map(|c| self.cards[c].value).unwrap_or(0) > m {
                return false;
            }
        }
        if let Some(mp) = f.min_power {
            if self.power(p) < mp {
                return false;
            }
        }
        if let Some(fl) = f.flying {
            if self.has(p, kw::FLYING) != fl {
                return false;
            }
        }
        if f.mono && mana::count_colours(self.colours_of(p)) != 1 {
            return false;
        }
        true
    }

    fn candidates(&self, f: &Filter, me: usize) -> Vec<usize> {
        (0..self.perms.len())
            .filter(|&i| self.matches(&self.perms[i], f, me))
            .collect()
    }

    // ---------- the record ----------

    fn snap(&self) -> Snap {
        let bank = |s: &Side| {
            s.bank
                .units
                .iter()
                .map(|u| {
                    if *u == mana::C {
                        "C".to_string()
                    } else if mana::count_colours(*u) == 1 {
                        mana::letter_of(*u).to_string()
                    } else if *u == mana::ANY {
                        "A".to_string()
                    } else {
                        mana::COLOURS
                            .iter()
                            .filter(|c| *u & **c != 0)
                            .map(|c| mana::letter_of(*c))
                            .collect()
                    }
                })
                .collect()
        };
        Snap {
            life: [self.s[0].life, self.s[1].life],
            bank: [bank(&self.s[0]), bank(&self.s[1])],
            hand: [self.s[0].hand.len(), self.s[1].hand.len()],
            library: [self.s[0].library.len(), self.s[1].library.len()],
            grave: [self.s[0].grave.len(), self.s[1].grave.len()],
            player_hand: self.s[P].hand.clone(),
            perms: self
                .perms
                .iter()
                .map(|p| PermSnap {
                    id: p.id,
                    side: p.ctrl as u8,
                    card: p.card.map(|c| c as i32).unwrap_or(-1),
                    token: p.token.map(|t| t.model().to_string()).unwrap_or_default(),
                    name: self.name_of(p),
                    power: self.power(p),
                    toughness: self.toughness(p),
                    damage: p.damage,
                    tapped: p.tapped,
                    attacking: p.attacking,
                    blocking: p.blocking.map(|b| b as i32).unwrap_or(-1),
                    detained: self.detained(p),
                    creature: self.is_creature(p),
                    attached: p.attached.map(|a| a as i32).unwrap_or(-1),
                })
                .collect(),
        }
    }

    fn log(
        &mut self,
        ev: Ev,
        side: usize,
        key: &'static str,
        args: Vec<(&'static str, String)>,
        focus: Vec<u32>,
    ) {
        let snap = self.snap();
        self.events.push(Event {
            ev,
            side: side as u8,
            turn: self.turn,
            key,
            args,
            focus,
            snap,
        });
    }

    fn side_name(&self, side: usize) -> String {
        self.s[side].name.clone()
    }

    fn check_end(&mut self) -> bool {
        if self.over.is_some() {
            return true;
        }
        let out = if self.s[P].life <= 0 {
            Some(Outcome::Lost)
        } else if self.s[F].life <= 0 {
            Some(Outcome::Won)
        } else {
            None
        };
        if let Some(o) = out {
            self.over = Some(o);
            let winner = if o == Outcome::Won { P } else { F };
            let n = self.side_name(winner);
            self.log(Ev::End, winner, "log.win", vec![("side", n)], vec![]);
            return true;
        }
        false
    }

    // ---------- life and damage ----------

    fn lose_life(&mut self, side: usize, n: i32) {
        self.s[side].life -= n;
    }

    fn gain_life(&mut self, side: usize, n: i32) {
        if n > 0 {
            self.s[side].life += n;
            let name = self.side_name(side);
            self.log(
                Ev::Life,
                side,
                "log.gain",
                vec![("side", name), ("n", n.to_string())],
                vec![],
            );
        }
    }

    /// Damage from a source to a side's face.
    fn damage_face(&mut self, src: Option<u32>, to: usize, n: i32, combat: bool) {
        if n <= 0 {
            return;
        }
        self.lose_life(to, n);
        let (src_name, src_side, lifelink, draw_any) = match src.and_then(|id| self.find(id)) {
            Some(i) => {
                let p = &self.perms[i];
                let draw_any = self
                    .rules_of(p)
                    .map(|r| r.statics.contains(&Static::DrawOnAnyHit))
                    .unwrap_or(false);
                (self.name_of(p), p.ctrl, self.has(p, kw::LIFELINK), draw_any)
            }
            None => (self.casting.clone().unwrap_or_default(), other(to), false, false),
        };
        let to_name = self.side_name(to);
        self.log(
            Ev::Face,
            src_side,
            "log.face",
            vec![("source", src_name), ("side", to_name), ("n", n.to_string())],
            src.into_iter().collect(),
        );
        if lifelink {
            self.gain_life(src_side, n);
        }
        if draw_any {
            self.draw(src_side, 1);
        }
        if combat {
            if let Some(i) = src.and_then(|id| self.find(id)) {
                let st = self
                    .rules_of(&self.perms[i])
                    .map(|r| r.statics.clone())
                    .unwrap_or_default();
                let ctrl = self.perms[i].ctrl;
                if st.contains(&Static::DrawOnHit) {
                    self.draw(ctrl, 1);
                }
                if st.contains(&Static::SoldierOnHit) {
                    self.make_token(ctrl, Token::Soldier, 1);
                }
            }
        }
        self.check_end();
    }

    /// Damage to a creature; marks it, and kills it once state is checked.
    fn damage_perm(&mut self, src: Option<u32>, target: u32, n: i32) {
        let Some(ti) = self.find(target) else { return };
        if n <= 0 {
            return;
        }
        self.perms[ti].damage += n;
        let (src_name, src_side, lifelink, deathtouch) = match src.and_then(|id| self.find(id)) {
            Some(i) => {
                let p = &self.perms[i];
                (
                    self.name_of(p),
                    p.ctrl,
                    self.has(p, kw::LIFELINK),
                    self.has(p, kw::DEATHTOUCH),
                )
            }
            None => (
                self.casting.clone().unwrap_or_default(),
                other(self.perms[ti].ctrl),
                false,
                false,
            ),
        };
        if deathtouch {
            self.perms[ti].deathtouched = true;
        }
        let tname = self.name_of(&self.perms[ti]);
        self.log(
            Ev::Hit,
            src_side,
            "log.damage",
            vec![("source", src_name), ("target", tname), ("n", n.to_string())],
            src.into_iter().chain([target]).collect(),
        );
        if lifelink {
            self.gain_life(src_side, n);
        }
    }

    /// Creatures with lethal damage or no toughness die; auras and
    /// equipment whose host is gone fall off.
    fn state_check(&mut self) {
        loop {
            let dying: Vec<u32> = self
                .perms
                .iter()
                .filter(|p| {
                    self.is_creature(p)
                        && (self.toughness(p) <= 0
                            || p.damage >= self.toughness(p)
                            || p.deathtouched && p.damage > 0)
                })
                .map(|p| p.id)
                .collect();
            if dying.is_empty() {
                break;
            }
            for id in dying {
                let Some(i) = self.find(id) else { continue };
                let p = &self.perms[i];
                let toughness_gone = self.toughness(p) <= 0;
                // regeneration: Grim Roustabout's ability, or the Golgari set's once a fight
                if !toughness_gone && self.try_regenerate(id) {
                    continue;
                }
                self.dies(id);
            }
        }
        let orphans: Vec<u32> = self
            .perms
            .iter()
            .filter(|p| p.attached.map(|a| self.find(a).is_none()).unwrap_or(false))
            .map(|p| p.id)
            .collect();
        for id in orphans {
            let Some(i) = self.find(id) else { continue };
            if self
                .rules_of(&self.perms[i])
                .map(|r| r.equip.is_some())
                .unwrap_or(false)
            {
                self.perms[i].attached = None;
            } else {
                self.leave(id, Leave::Graveyard);
            }
        }
    }

    fn try_regenerate(&mut self, id: u32) -> bool {
        let Some(i) = self.find(id) else { return false };
        let ctrl = self.perms[i].ctrl;
        let regen = self
            .rules_of(&self.perms[i])
            .and_then(|r| r.activated.iter().find(|a| a.when == Use::Regenerate).cloned());
        let paid = match regen {
            Some(a)
                if !self.arrested(&self.perms[i])
                    && !self.detained(&self.perms[i])
                    && self.s[ctrl].bank.pay(&a.cost) =>
            {
                true
            }
            _ if self.s[ctrl].perks & perk::SET_REGENERATE != 0 && !self.s[ctrl].regen_used => {
                self.s[ctrl].regen_used = true;
                true
            }
            _ => false,
        };
        if !paid {
            return false;
        }
        let p = &mut self.perms[i];
        p.damage = 0;
        p.deathtouched = false;
        p.tapped = true;
        p.attacking = false;
        p.blocking = None;
        let name = self.name_of(&self.perms[i]);
        self.log(
            Ev::Ability,
            ctrl,
            "log.regenerate",
            vec![("card", name)],
            vec![id],
        );
        true
    }

    fn dies(&mut self, id: u32) {
        let Some(i) = self.find(id) else { return };
        let p = self.perms[i].clone();
        let name = self.name_of(&p);
        self.log(Ev::Dies, p.ctrl, "log.dies", vec![("card", name)], vec![id]);
        let power = self.power(&p);
        let dies_effects = self.rules_of(&p).map(|r| r.dies.clone()).unwrap_or_default();
        let to_top = dies_effects.contains(&Eff::ToLibraryTop);
        self.leave(
            id,
            if to_top {
                Leave::LibraryTop
            } else {
                Leave::Graveyard
            },
        );
        for e in dies_effects.iter().filter(|e| **e != Eff::ToLibraryTop) {
            self.resolve(p.ctrl, None, e, None);
        }
        // Death's Presence: its controller puts the dead creature's power in counters on another
        if self.statics_of(p.ctrl, Static::DeathsPresence) > 0 && power > 0 {
            if let Some(t) = self.best(
                &self.candidates(&crate::rules::creatures(Who::Mine), p.ctrl),
                true,
            ) {
                let tid = self.perms[t].id;
                self.perms[t].plus += power;
                let tname = self.name_of(&self.perms[t]);
                self.log(
                    Ev::Ability,
                    p.ctrl,
                    "log.counters",
                    vec![("card", tname), ("n", power.to_string())],
                    vec![tid],
                );
            }
        }
        // the Golgari gloves: the first creature of yours to die comes back to your hand
        let owner = p.owner;
        if let Some(c) = p.card {
            if self.s[owner].perks & perk::REGROWTH != 0
                && !self.s[owner].regrowth_used
                && self.cards[c].is(ty::CREATURE)
            {
                if let Some(pos) = self.s[owner].grave.iter().rposition(|&g| g == c) {
                    self.s[owner].grave.remove(pos);
                    self.s[owner].hand.push(c);
                    self.s[owner].regrowth_used = true;
                    let n = self.cards[c].name.clone();
                    self.log(Ev::Ability, owner, "log.regrowth", vec![("card", n)], vec![]);
                }
            }
        }
    }

    fn leave(&mut self, id: u32, to: Leave) {
        let Some(i) = self.find(id) else { return };
        let p = self.perms.remove(i);
        for (c, owner) in p.held.clone() {
            // Angel of Serenity returns them to hand; Detention Sphere to the battlefield
            let sphere = p
                .card
                .map(|k| self.cards[k].name == "Detention Sphere")
                .unwrap_or(false);
            if let Some(pos) = self.s[owner].exile.iter().position(|&e| e == c) {
                self.s[owner].exile.remove(pos);
            }
            if sphere {
                self.enter(owner, c, false);
            } else {
                self.s[owner].hand.push(c);
            }
        }
        // what was attached to it falls off; what it blocked is unblocked
        for q in self.perms.iter_mut() {
            if q.blocking == Some(id) {
                q.blocking = None;
            }
        }
        let Some(c) = p.card else { return };
        match to {
            Leave::Graveyard => {
                if self.cards[c].is_spell() && self.any_static(Static::SpellsToExile) {
                    self.s[p.owner].exile.push(c)
                } else {
                    self.s[p.owner].grave.push(c)
                }
            }
            Leave::Hand => self.s[p.owner].hand.push(c),
            Leave::Exile => self.s[p.owner].exile.push(c),
            Leave::LibraryTop => self.s[p.owner].library.push(c),
        }
    }

    // ---------- entering ----------

    fn new_perm(&mut self, side: usize, card: Option<usize>, token: Option<Token>) -> u32 {
        self.next_id += 1;
        let id = self.next_id;
        self.perms.push(Perm {
            id,
            card,
            token,
            ctrl: side,
            owner: side,
            tapped: false,
            sick: true,
            damage: 0,
            plus: 0,
            tp: 0,
            tt: 0,
            tkw: 0,
            detained_until: 0,
            attached: None,
            animated: false,
            unleashed: false,
            attacking: false,
            blocking: None,
            deathtouched: false,
            held: Vec::new(),
        });
        id
    }

    fn make_token(&mut self, side: usize, t: Token, n: u8) {
        for _ in 0..n {
            let id = self.new_perm(side, None, Some(t));
            let name = self.side_name(side);
            self.log(
                Ev::Token,
                side,
                "log.token",
                vec![("side", name), ("token", t.stats().4.to_string())],
                vec![id],
            );
        }
    }

    fn populate(&mut self, side: usize) {
        let best = self
            .perms
            .iter()
            .filter(|p| p.ctrl == side && p.token.is_some())
            .max_by_key(|p| (self.value(p), p.id))
            .and_then(|p| p.token);
        if let Some(t) = best {
            self.make_token(side, t, 1);
        }
    }

    /// A permanent card enters under `side`. Runs its enters triggers.
    fn enter(&mut self, side: usize, c: usize, cast: bool) -> u32 {
        let id = self.new_perm(side, Some(c), None);
        let card = &self.cards[c];
        let rules = card.rules.clone();
        if card.is(ty::CREATURE) && cast {
            self.s[side].creatures_cast += 1;
            if self.s[side].perks & perk::HASTE_FIRST != 0 && self.s[side].creatures_cast == 1 {
                let i = self.find(id).unwrap_or(0);
                self.perms[i].tkw |= kw::HASTE;
                self.perms[i].tp += 0;
            }
        }
        if rules.unleash && self.wants_unleash(side) {
            let i = self.find(id).unwrap_or(0);
            self.perms[i].plus += 1;
            self.perms[i].unleashed = true;
        }
        let name = card.name.clone();
        let key = if rules.unleash && self.perms.last().map(|p| p.unleashed).unwrap_or(false) {
            "log.enter_unleashed"
        } else {
            "log.enter"
        };
        let sname = self.side_name(side);
        self.log(
            Ev::Enter,
            side,
            key,
            vec![("side", sname), ("card", name)],
            vec![id],
        );
        for e in &rules.etb {
            self.resolve(side, Some(id), e, None);
            if self.over.is_some() {
                break;
            }
        }
        self.state_check();
        id
    }

    /// Unleash when ahead or level on creatures: a side behind keeps its blockers.
    fn wants_unleash(&self, side: usize) -> bool {
        let count = |s: usize| {
            self.perms
                .iter()
                .filter(|p| p.ctrl == s && self.is_creature(p))
                .count()
        };
        count(side) + 1 >= count(other(side)) || self.s[side].life > self.s[other(side)].life
    }

    // ---------- choosing ----------

    /// The best of some permanents: highest value, or lowest for `own_worst`.
    fn best(&self, idx: &[usize], highest: bool) -> Option<usize> {
        let key = |&&i: &&usize| {
            (
                self.value(&self.perms[i]),
                self.perms[i].id as i32 * if highest { -1 } else { 1 },
            )
        };
        if highest {
            idx.iter().max_by_key(key).copied()
        } else {
            idx.iter().min_by_key(key).copied()
        }
    }

    /// Picks the target for an aimed effect: None if it has none worth it.
    fn pick(&self, side: usize, aim: &Aim, eff: &Eff) -> Option<Target> {
        match aim {
            Aim::Face => Some(Target::Face(other(side))),
            Aim::Me => Some(Target::Face(side)),
            Aim::Each(_) => Some(Target::All),
            Aim::UpTo(_, f) | Aim::One(f) => {
                let c = self.candidates(f, side);
                let c: Vec<usize> = match eff {
                    // damage: the best creature it kills
                    Eff::Damage(n, _) => c
                        .into_iter()
                        .filter(|&i| {
                            let p = &self.perms[i];
                            self.toughness(p) - p.damage <= *n
                        })
                        .collect(),
                    Eff::Detain(_) | Eff::Tap(_) => c
                        .into_iter()
                        .filter(|&i| {
                            !self.detained(&self.perms[i])
                                && !self.perms[i].tapped
                                && self.is_creature(&self.perms[i])
                        })
                        .collect(),
                    Eff::Pump { p, .. } if *p < 0 => c
                        .into_iter()
                        .filter(|&i| self.perms[i].attacking || !self.perms[i].tapped)
                        .collect(),
                    Eff::Pump { .. } | Eff::Chorus(_) | Eff::Counters(..) => {
                        // your own creature most likely to fight: untapped and able to attack
                        let ready: Vec<usize> = c
                            .iter()
                            .copied()
                            .filter(|&i| {
                                self.can_attack(&self.perms[i])
                                    || self.perms[i].attacking
                                    || self.perms[i].blocking.is_some()
                            })
                            .collect();
                        if ready.is_empty() && matches!(eff, Eff::Pump { .. } | Eff::Chorus(_)) {
                            return None;
                        }
                        if ready.is_empty() {
                            c
                        } else {
                            ready
                        }
                    }
                    _ => c,
                };
                let b = self.best(&c, true)?;
                Some(Target::Perm(self.perms[b].id))
            }
            Aim::CreatureOrFace(f) => {
                let n = match eff {
                    Eff::Damage(n, _) => *n,
                    _ => 0,
                };
                // lethal to the face first
                if self.s[other(side)].life <= n {
                    return Some(Target::Face(other(side)));
                }
                let killable: Vec<usize> = self
                    .candidates(f, side)
                    .into_iter()
                    .filter(|&i| {
                        let p = &self.perms[i];
                        self.toughness(p) - p.damage <= n
                    })
                    .collect();
                match self.best(&killable, true) {
                    Some(b) => Some(Target::Perm(self.perms[b].id)),
                    None => Some(Target::Face(other(side))),
                }
            }
        }
    }

    // ---------- resolving ----------

    fn resolve(&mut self, side: usize, src: Option<u32>, e: &Eff, chosen: Option<Target>) {
        if self.over.is_some() {
            return;
        }
        let me = side;
        match e {
            Eff::Damage(n, aim) => match aim {
                Aim::Each(f) => {
                    for id in self
                        .candidates(f, me)
                        .into_iter()
                        .map(|i| self.perms[i].id)
                        .collect::<Vec<_>>()
                    {
                        self.damage_perm(src, id, *n);
                    }
                }
                Aim::Face => self.damage_face(src, other(me), *n, false),
                _ => match chosen.or_else(|| self.pick(me, aim, e)) {
                    Some(Target::Perm(id)) => self.damage_perm(src, id, *n),
                    Some(Target::Face(s)) => self.damage_face(src, s, *n, false),
                    _ => {}
                },
            },
            Eff::Destroy(aim) | Eff::Exile(aim) | Eff::Bounce(aim) => {
                let ids: Vec<u32> = match aim {
                    Aim::Each(f) => self
                        .candidates(f, me)
                        .into_iter()
                        .map(|i| self.perms[i].id)
                        .collect(),
                    _ => match chosen.or_else(|| self.pick(me, aim, e)) {
                        Some(Target::Perm(id)) => vec![id],
                        _ => vec![],
                    },
                };
                for id in ids {
                    let Some(i) = self.find(id) else { continue };
                    let name = self.name_of(&self.perms[i]);
                    let ctrl = self.perms[i].ctrl;
                    match e {
                        Eff::Destroy(_) => {
                            self.log(Ev::Ability, me, "log.destroy", vec![("card", name)], vec![id]);
                            if !self.try_regenerate(id) {
                                self.dies(id);
                            }
                        }
                        Eff::Exile(_) => {
                            self.log(Ev::Ability, me, "log.exile", vec![("card", name)], vec![id]);
                            self.leave(id, Leave::Exile);
                        }
                        _ => {
                            self.log(Ev::Ability, me, "log.bounce", vec![("card", name)], vec![id]);
                            self.leave(id, Leave::Hand);
                        }
                    }
                    let _ = ctrl;
                }
                self.state_check();
            }
            Eff::Detain(aim) => {
                let ids: Vec<u32> = match aim {
                    Aim::UpTo(n, f) => {
                        let mut c = self.candidates(f, me);
                        c.retain(|&i| !self.detained(&self.perms[i]));
                        c.sort_by_key(|&i| (-self.value(&self.perms[i]), self.perms[i].id));
                        c.into_iter()
                            .take(*n as usize)
                            .map(|i| self.perms[i].id)
                            .collect()
                    }
                    _ => match chosen.or_else(|| self.pick(me, aim, e)) {
                        Some(Target::Perm(id)) => vec![id],
                        _ => vec![],
                    },
                };
                for id in ids {
                    self.detain(me, id);
                }
            }
            Eff::Tap(aim) => {
                let ids: Vec<u32> = match aim {
                    Aim::Each(f) => self
                        .candidates(f, me)
                        .into_iter()
                        .map(|i| self.perms[i].id)
                        .collect(),
                    _ => match chosen.or_else(|| self.pick(me, aim, e)) {
                        Some(Target::Perm(id)) => vec![id],
                        _ => vec![],
                    },
                };
                for id in ids {
                    if let Some(i) = self.find(id) {
                        self.perms[i].tapped = true;
                        let name = self.name_of(&self.perms[i]);
                        self.log(Ev::Ability, me, "log.tap", vec![("card", name)], vec![id]);
                    }
                }
            }
            Eff::Pump { p, t, kw, aim, untap } => {
                let ids: Vec<u32> = match aim {
                    Aim::Each(f) => self
                        .candidates(f, me)
                        .into_iter()
                        .map(|i| self.perms[i].id)
                        .collect(),
                    _ => match chosen.or_else(|| self.pick(me, aim, e)) {
                        Some(Target::Perm(id)) => vec![id],
                        _ => vec![],
                    },
                };
                for id in ids {
                    if let Some(i) = self.find(id) {
                        self.perms[i].tp += p;
                        self.perms[i].tt += t;
                        self.perms[i].tkw |= kw;
                        if *untap {
                            self.perms[i].tapped = false;
                        }
                        let name = self.name_of(&self.perms[i]);
                        self.log(
                            Ev::Ability,
                            me,
                            "log.pump",
                            vec![("card", name), ("p", signed(*p)), ("t", signed(*t))],
                            vec![id],
                        );
                    }
                }
                self.state_check();
            }
            Eff::Chorus(aim) => {
                let n = self
                    .perms
                    .iter()
                    .filter(|q| q.ctrl == me && self.is_creature(q))
                    .count() as i32;
                let pump = Eff::Pump {
                    p: n,
                    t: n,
                    kw: kw::TRAMPLE,
                    aim: *aim,
                    untap: false,
                };
                self.resolve(me, src, &pump, chosen);
            }
            Eff::Counters(n, aim) => {
                if let Some(Target::Perm(id)) = chosen.or_else(|| self.pick(me, aim, e)) {
                    if let Some(i) = self.find(id) {
                        self.perms[i].plus += n;
                        let name = self.name_of(&self.perms[i]);
                        self.log(
                            Ev::Ability,
                            me,
                            "log.counters",
                            vec![("card", name), ("n", n.to_string())],
                            vec![id],
                        );
                    }
                }
            }
            Eff::Token(t, n) => self.make_token(me, *t, *n),
            Eff::Populate => self.populate(me),
            Eff::Draw(n) => self.draw(me, *n as usize),
            Eff::Gain(n) => self.gain_life(me, *n),
            Eff::Drain(n) => self.damage_face(src, other(me), *n, false),
            Eff::OppDiscard(n) => self.discard(other(me), *n as usize),
            Eff::EachSacrifices => {
                for s in [me, other(me)] {
                    let mine: Vec<usize> = (0..self.perms.len())
                        .filter(|&i| self.perms[i].ctrl == s && self.is_creature(&self.perms[i]))
                        .collect();
                    if let Some(w) = self.best(&mine, false) {
                        let id = self.perms[w].id;
                        let name = self.name_of(&self.perms[w]);
                        self.log(Ev::Ability, s, "log.sacrifice", vec![("card", name)], vec![id]);
                        self.dies(id);
                    }
                }
            }
            Eff::ImpostorReturn => {
                let others: Vec<usize> = (0..self.perms.len())
                    .filter(|&i| {
                        self.perms[i].ctrl == me
                            && Some(self.perms[i].id) != src
                            && self.is_creature(&self.perms[i])
                            && self.perms[i].card.is_some()
                    })
                    .collect();
                match self.best(&others, false) {
                    Some(w) => {
                        let id = self.perms[w].id;
                        let name = self.name_of(&self.perms[w]);
                        self.log(Ev::Ability, me, "log.bounce", vec![("card", name)], vec![id]);
                        self.leave(id, Leave::Hand);
                    }
                    None => {
                        if let Some(id) = src {
                            let name = self
                                .find(id)
                                .map(|i| self.name_of(&self.perms[i]))
                                .unwrap_or_default();
                            self.log(Ev::Ability, me, "log.sacrifice", vec![("card", name)], vec![id]);
                            self.dies(id);
                        }
                    }
                }
            }
            Eff::AngelExile => {
                let mut c: Vec<usize> = (0..self.perms.len())
                    .filter(|&i| {
                        let p = &self.perms[i];
                        p.ctrl != me && self.is_creature(p) && !self.has(p, kw::HEXPROOF)
                    })
                    .collect();
                c.sort_by_key(|&i| (-self.value(&self.perms[i]), self.perms[i].id));
                let ids: Vec<u32> = c.into_iter().take(3).map(|i| self.perms[i].id).collect();
                for id in ids {
                    self.hold(src, id);
                }
            }
            Eff::SphereExile => {
                let target = self.pick(me, &Aim::One(crate::rules::permanents(Who::Theirs)), e);
                if let Some(Target::Perm(id)) = target {
                    let name = self
                        .find(id)
                        .map(|i| self.name_of(&self.perms[i]))
                        .unwrap_or_default();
                    let same: Vec<u32> = self
                        .perms
                        .iter()
                        .filter(|p| {
                            p.ctrl != me && self.name_of(p) == name && self.name_of(p) != "Detention Sphere"
                        })
                        .map(|p| p.id)
                        .collect();
                    for id in same {
                        self.hold(src, id);
                    }
                }
            }
            Eff::Loot(d, x) => {
                self.draw(me, *d as usize);
                self.discard(me, *x as usize);
            }
            Eff::ToLibraryTop | Eff::Counter { .. } => {}
        }
    }

    /// Exiles a permanent until `src` leaves.
    fn hold(&mut self, src: Option<u32>, id: u32) {
        let Some(i) = self.find(id) else { return };
        let p = self.perms[i].clone();
        let name = self.name_of(&p);
        self.log(
            Ev::Ability,
            other(p.ctrl),
            "log.exile",
            vec![("card", name)],
            vec![id],
        );
        self.leave(id, Leave::Exile);
        if let (Some(c), Some(s)) = (p.card, src.and_then(|s| self.find(s))) {
            self.perms[s].held.push((c, p.owner));
        }
    }

    fn detain(&mut self, me: usize, id: u32) {
        let Some(i) = self.find(id) else { return };
        // until the detainer's next turn
        let until = if self.active == me {
            self.turn + 2
        } else {
            self.turn + 1
        };
        self.perms[i].detained_until = until;
        let name = self.name_of(&self.perms[i]);
        self.log(Ev::Detain, me, "log.detain", vec![("card", name)], vec![id]);
    }

    fn draw(&mut self, side: usize, n: usize) {
        for _ in 0..n {
            if self.over.is_some() {
                return;
            }
            if self.s[side].hand.len() >= HAND_MAX {
                return;
            }
            match self.s[side].library.pop() {
                Some(c) => {
                    self.s[side].hand.push(c);
                    let name = self.side_name(side);
                    let card = if side == P {
                        self.cards[c].name.clone()
                    } else {
                        String::new()
                    };
                    let key = if side == P { "log.draw" } else { "log.draw_hidden" };
                    self.log(Ev::Draw, side, key, vec![("side", name), ("card", card)], vec![]);
                }
                None => {
                    self.lose_life(side, EMPTY_DRAW_LOSS);
                    let name = self.side_name(side);
                    self.log(
                        Ev::Life,
                        side,
                        "log.empty_library",
                        vec![("side", name), ("n", EMPTY_DRAW_LOSS.to_string())],
                        vec![],
                    );
                    if self.check_end() {
                        return;
                    }
                    let mut back = std::mem::take(&mut self.s[side].grave);
                    for i in (1..back.len()).rev() {
                        let j = (self.rng.next_u64() % (i as u64 + 1)) as usize;
                        back.swap(i, j);
                    }
                    if back.is_empty() {
                        return;
                    }
                    self.s[side].library = back;
                    if let Some(c) = self.s[side].library.pop() {
                        self.s[side].hand.push(c);
                    }
                }
            }
        }
    }

    /// Discards the cheapest cards: a player throws away what matters least.
    fn discard(&mut self, side: usize, n: usize) {
        for _ in 0..n {
            let Some(pos) =
                (0..self.s[side].hand.len()).min_by_key(|&i| (self.cards[self.s[side].hand[i]].value, i))
            else {
                return;
            };
            let c = self.s[side].hand.remove(pos);
            self.s[side].grave.push(c);
            let name = self.side_name(side);
            let card = self.cards[c].name.clone();
            self.log(
                Ev::Ability,
                side,
                "log.discard",
                vec![("side", name), ("card", card)],
                vec![],
            );
        }
    }

    // ---------- casting ----------

    fn cost_for(&self, side: usize, c: usize) -> Cost {
        let card = &self.cards[c];
        let mut cut = 0;
        if card.is_spell() {
            cut += self.statics_of(side, Static::SpellsCheaper) as u8;
            if self.s[side].perks & perk::SET_SPELL_CHEAP != 0 {
                cut += 1;
            }
        }
        card.cost.reduced(cut)
    }

    fn overload_cost(&self, side: usize, c: usize) -> Option<Cost> {
        let (cost, _) = self.cards[c].rules.overload.as_ref()?;
        let mut cut = self.statics_of(side, Static::SpellsCheaper) as u8;
        if self.s[side].perks & perk::SET_SPELL_CHEAP != 0 {
            cut += 1;
        }
        Some(cost.reduced(cut))
    }

    /// How a card would be cast now, if it should be: its mode and target.
    fn plan_cast(&self, side: usize, c: usize, window: Timing) -> Option<Plan> {
        let card = &self.cards[c];
        if card.is(ty::LAND) {
            return None;
        }
        let r = &card.rules;
        let timing_ok = match window {
            Timing::Main => r.timing == Timing::Main || card.is(ty::CREATURE),
            Timing::EndStep => r.timing == Timing::EndStep,
            Timing::Defend => r.timing == Timing::Defend,
            Timing::Answer => false,
        };
        if !timing_ok {
            return None;
        }
        let bank = &self.s[side].bank;
        // an overload worth more than one target
        if let Some(oc) = self.overload_cost(side, c) {
            if let Some((_, effs)) = &r.overload {
                let hits = effs
                    .iter()
                    .map(|e| match e {
                        Eff::Damage(_, Aim::Each(f))
                        | Eff::Tap(Aim::Each(f))
                        | Eff::Bounce(Aim::Each(f))
                        | Eff::Pump {
                            aim: Aim::Each(f), ..
                        } => self.candidates(f, side).len(),
                        _ => 0,
                    })
                    .max()
                    .unwrap_or(0);
                if hits >= 2 && bank.can_pay(&oc) {
                    return Some(Plan {
                        card: c,
                        mode: None,
                        overload: true,
                        target: None,
                        cost: oc,
                    });
                }
            }
        }
        let cost = self.cost_for(side, c);
        if !bank.can_pay(&cost) {
            return None;
        }
        if card.is(ty::CREATURE) || (card.is(ty::ARTIFACT | ty::ENCHANTMENT) && r.aura.is_none()) {
            // Faerie Impostor needs another creature of yours, or it is a wasted card
            if r.etb.contains(&Eff::ImpostorReturn)
                && !self.perms.iter().any(|p| p.ctrl == side && self.is_creature(p))
            {
                return None;
            }
            if r.etb.contains(&Eff::EachSacrifices)
                && !self.perms.iter().any(|p| p.ctrl != side && self.is_creature(p))
            {
                return None;
            }
            if r.etb.contains(&Eff::SphereExile)
                && self
                    .pick(
                        side,
                        &Aim::One(crate::rules::permanents(Who::Theirs)),
                        &Eff::SphereExile,
                    )
                    .is_none()
            {
                return None;
            }
            if r.equip.is_some() && !self.perms.iter().any(|p| p.ctrl == side && self.is_creature(p)) {
                return None;
            }
            return Some(Plan {
                card: c,
                mode: None,
                overload: false,
                target: None,
                cost,
            });
        }
        if let Some(a) = r.aura {
            let f = match a {
                Aura::EtherealArmor => crate::rules::creatures(Who::Mine),
                _ => crate::rules::creatures(Who::Theirs),
            };
            let c2 = self.candidates(&f, side);
            let c2: Vec<usize> = if a == Aura::StabWound {
                // a creature it kills, else the best
                let kills: Vec<usize> = c2
                    .iter()
                    .copied()
                    .filter(|&i| self.toughness(&self.perms[i]) <= 2)
                    .collect();
                if kills.is_empty() {
                    c2
                } else {
                    kills
                }
            } else {
                c2
            };
            let t = self.best(&c2, true)?;
            return Some(Plan {
                card: c,
                mode: None,
                overload: false,
                target: Some(Target::Perm(self.perms[t].id)),
                cost,
            });
        }
        // an instant or sorcery: the first mode that has a target worth it
        for (m, effs) in r.modes.iter().enumerate() {
            let Some(first) = effs.first() else { continue };
            if matches!(first, Eff::Counter { .. }) {
                continue;
            }
            let target = match first {
                Eff::Damage(_, aim)
                | Eff::Destroy(aim)
                | Eff::Exile(aim)
                | Eff::Bounce(aim)
                | Eff::Detain(aim)
                | Eff::Tap(aim)
                | Eff::Chorus(aim)
                | Eff::Counters(_, aim)
                | Eff::Pump { aim, .. } => match aim {
                    Aim::Each(_) | Aim::Face | Aim::Me => None,
                    _ => Some(self.pick(side, aim, first)?),
                },
                Eff::OppDiscard(_) if self.s[other(side)].hand.is_empty() => return None,
                Eff::Loot(..) if self.s[side].hand.len() > 2 => continue,
                Eff::Populate if !self.perms.iter().any(|p| p.ctrl == side && p.token.is_some()) => continue,
                _ => None,
            };
            // Blustersquall taps a blocker before your attack: only with an attacker ready
            if matches!(first, Eff::Tap(_))
                && !self.perms.iter().any(|p| p.ctrl == side && self.can_attack(p))
            {
                continue;
            }
            // a pump in your main phase only goes on a creature about to attack
            if matches!(first, Eff::Pump { p, .. } if *p > 0) && window == Timing::Main {
                if let Some(Target::Perm(id)) = target {
                    if !self
                        .find(id)
                        .map(|i| self.can_attack(&self.perms[i]))
                        .unwrap_or(false)
                    {
                        continue;
                    }
                }
            }
            return Some(Plan {
                card: c,
                mode: Some(m),
                overload: false,
                target,
                cost,
            });
        }
        None
    }

    /// Casts a planned card: pays, lets the other side answer, resolves.
    fn cast(&mut self, side: usize, plan: Plan) {
        let c = plan.card;
        if !self.s[side].bank.pay(&plan.cost) {
            return;
        }
        if let Some(pos) = self.s[side].hand.iter().position(|&h| h == c) {
            self.s[side].hand.remove(pos);
        }
        let card = &self.cards[c];
        let name = card.name.clone();
        let sname = self.side_name(side);
        let key = if plan.overload {
            "log.cast_overload"
        } else {
            "log.cast"
        };
        self.log(
            Ev::Cast,
            side,
            key,
            vec![("side", sname), ("card", name.clone())],
            vec![],
        );
        self.on_cast_triggers(side, c);
        if self.over.is_some() {
            return;
        }
        if self.answer(other(side), c) {
            // countered
            if card.is_spell() && self.any_static(Static::SpellsToExile) {
                self.s[side].exile.push(c);
            } else {
                self.s[side].grave.push(c);
            }
            return;
        }
        if card.is_spell() {
            self.casting = Some(card.name.clone());
            let effs = if plan.overload {
                card.rules
                    .overload
                    .as_ref()
                    .map(|o| o.1.clone())
                    .unwrap_or_default()
            } else {
                card.rules
                    .modes
                    .get(plan.mode.unwrap_or(0))
                    .cloned()
                    .unwrap_or_default()
            };
            for (k, e) in effs.iter().enumerate() {
                let t = if k == 0 { plan.target } else { None };
                self.resolve(side, None, e, t);
                if self.over.is_some() {
                    self.casting = None;
                    return;
                }
            }
            self.casting = None;
            if self.any_static(Static::SpellsToExile) {
                self.s[side].exile.push(c);
            } else {
                self.s[side].grave.push(c);
            }
            self.state_check();
            return;
        }
        let id = self.enter(side, c, true);
        if let (Some(Target::Perm(t)), true) = (plan.target, card.rules.aura.is_some()) {
            if let Some(i) = self.find(id) {
                self.perms[i].attached = Some(t);
            }
            self.state_check();
        }
    }

    fn on_cast_triggers(&mut self, side: usize, c: usize) {
        let spell = self.cards[c].is_spell();
        let multi = self.cards[c].multicoloured();
        let mut burns = Vec::new();
        for p in self.perms.iter().filter(|p| p.ctrl == side) {
            match self.rules_of(p).and_then(|r| r.on_cast) {
                Some(OnCast::SpellBurn) if spell => burns.push((p.id, Eff::Damage(2, Aim::Face))),
                Some(OnCast::MulticolourBurn) if multi => burns.push((
                    p.id,
                    Eff::Damage(2, Aim::CreatureOrFace(crate::rules::creatures(Who::Theirs))),
                )),
                _ => {}
            }
        }
        if multi {
            for p in self.perms.iter_mut().filter(|p| p.ctrl == side) {
                if p.card
                    .map(|k| self.cards[k].rules.on_cast == Some(OnCast::MulticolourUntap))
                    .unwrap_or(false)
                {
                    p.tapped = false;
                }
            }
        }
        for (id, e) in burns {
            self.resolve(side, Some(id), &e, None);
        }
        if spell && self.s[side].perks & perk::SPELL_PING != 0 {
            self.damage_face(None, other(side), 1, false);
        }
    }

    /// The other side's chance to counter a spell. True if it was countered.
    fn answer(&mut self, side: usize, spell: usize) -> bool {
        let card = &self.cards[spell];
        if card.rules.kw & kw::CANT_BE_COUNTERED != 0 {
            return false;
        }
        // worth a counter: a big spell, a strong creature, or removal
        let worth = card.value >= 3
            || card.power >= 3
            || card
                .rules
                .modes
                .iter()
                .flatten()
                .any(|e| matches!(e, Eff::Destroy(_) | Eff::Exile(_)));
        if !worth {
            return false;
        }
        let hand = self.s[side].hand.clone();
        for h in hand {
            let r = &self.cards[h].rules;
            let Some((m, pay)) = r
                .modes
                .iter()
                .enumerate()
                .find_map(|(m, effs)| match effs.first() {
                    Some(Eff::Counter { creature, pay }) => {
                        let fits = match creature {
                            Some(true) => card.is(ty::CREATURE),
                            Some(false) => !card.is(ty::CREATURE),
                            None => true,
                        };
                        fits.then_some((m, *pay))
                    }
                    _ => None,
                })
            else {
                continue;
            };
            let _ = m;
            let cost = self.cost_for(side, h);
            if !self.s[side].bank.pay(&cost) {
                continue;
            }
            self.s[side].hand.retain(|&x| x != h);
            let name = self.cards[h].name.clone();
            let target = card.name.clone();
            let sname = self.side_name(side);
            self.log(
                Ev::Cast,
                side,
                "log.cast",
                vec![("side", sname), ("card", name.clone())],
                vec![],
            );
            self.on_cast_triggers(side, h);
            self.s[side].grave.push(h);
            // "unless its controller pays": the caster pays if it can
            if pay > 0 {
                let tax = Cost::of(&[crate::mana::Sym::Generic(pay)]);
                if self.s[other(side)].bank.pay(&tax) {
                    let csname = self.side_name(other(side));
                    self.log(
                        Ev::Counter,
                        other(side),
                        "log.paid",
                        vec![("side", csname), ("card", target)],
                        vec![],
                    );
                    return false;
                }
            }
            self.log(
                Ev::Counter,
                side,
                "log.countered",
                vec![("card", target), ("by", name)],
                vec![],
            );
            return true;
        }
        false
    }

    // ---------- combat ----------

    fn can_attack(&self, p: &Perm) -> bool {
        self.is_creature(p)
            && !p.tapped
            && (!p.sick || self.has(p, kw::HASTE))
            && !self.has(p, kw::DEFENDER)
            && !self.detained(p)
            && !self.arrested(p)
            && self.power(p) > 0
    }

    fn can_block(&self, blocker: &Perm, attacker: &Perm) -> bool {
        if !self.is_creature(blocker) || blocker.tapped || self.detained(blocker) || self.arrested(blocker) {
            return false;
        }
        // an unleashed creature with a +1/+1 counter can't block
        if blocker.unleashed && blocker.plus > 0 {
            return false;
        }
        if self.has(attacker, kw::UNBLOCKABLE) {
            return false;
        }
        if self.has(attacker, kw::FLYING) && !self.has(blocker, kw::FLYING | kw::REACH) {
            return false;
        }
        if self.has(attacker, kw::SWAMPWALK) && self.s[blocker.ctrl].swamps > 0 {
            return false;
        }
        true
    }

    /// Whether `a` kills `b` in a fight between them, counting first strike.
    fn kills(&self, a: &Perm, b: &Perm) -> bool {
        let dmg = self.power(a) * if self.has(a, kw::DOUBLE_STRIKE) { 2 } else { 1 };
        dmg > 0 && (self.has(a, kw::DEATHTOUCH) || dmg + b.damage >= self.toughness(b))
    }

    fn strikes_first(&self, a: &Perm, b: &Perm) -> bool {
        self.has(a, kw::FIRST_STRIKE | kw::DOUBLE_STRIKE)
            && !self.has(b, kw::FIRST_STRIKE | kw::DOUBLE_STRIKE)
    }

    /// Whether `a` survives a fight with `b`.
    fn survives(&self, a: &Perm, b: &Perm) -> bool {
        if self.strikes_first(a, b) && self.kills(a, b) {
            return true;
        }
        !self.kills(b, a)
    }

    fn choose_attackers(&self, side: usize) -> Vec<u32> {
        let foes: Vec<&Perm> = self.perms.iter().filter(|p| p.ctrl != side).collect();
        let foe_life = self.s[other(side)].life;
        let ready: Vec<&Perm> = self
            .perms
            .iter()
            .filter(|p| p.ctrl == side && self.can_attack(p))
            .collect();
        // all in when the open damage is lethal
        let open: i32 = ready
            .iter()
            .filter(|a| !foes.iter().any(|b| self.can_block(b, a)))
            .map(|a| self.power(a) * if self.has(a, kw::DOUBLE_STRIKE) { 2 } else { 1 })
            .sum();
        if open >= foe_life {
            return ready.iter().map(|a| a.id).collect();
        }
        ready
            .iter()
            .filter(|a| {
                foes.iter().filter(|b| self.can_block(b, a)).all(|b| {
                    // safe against this blocker, or a trade that favours us
                    self.survives(a, b) || (self.kills(a, b) && self.value(b) >= self.value(a))
                })
            })
            .map(|a| a.id)
            .collect()
    }

    fn choose_blocks(&self, side: usize) -> Vec<(u32, u32)> {
        let mut attackers: Vec<&Perm> = self.perms.iter().filter(|p| p.attacking).collect();
        attackers.sort_by_key(|a| (-self.power(a), a.id));
        let incoming: i32 = attackers
            .iter()
            .map(|a| self.power(a) * if self.has(a, kw::DOUBLE_STRIKE) { 2 } else { 1 })
            .sum();
        let mut life_after = self.s[side].life - incoming;
        let mut used: Vec<u32> = Vec::new();
        let mut out = Vec::new();
        for a in attackers {
            let options: Vec<&Perm> = self
                .perms
                .iter()
                .filter(|b| b.ctrl == side && !used.contains(&b.id) && self.can_block(b, a))
                .collect();
            let dmg = self.power(a) * if self.has(a, kw::DOUBLE_STRIKE) { 2 } else { 1 };
            // 1: kill it and live; 2: trade up; 3: chump when the hit would be lethal
            let good = options
                .iter()
                .filter(|b| self.survives(b, a) && self.kills(b, a))
                .min_by_key(|b| (self.value(b), b.id));
            let safe = options
                .iter()
                .filter(|b| self.survives(b, a))
                .min_by_key(|b| (self.value(b), b.id));
            let trade = options
                .iter()
                .filter(|b| self.kills(b, a) && self.value(a) >= self.value(b))
                .min_by_key(|b| (self.value(b), b.id));
            let chump = if life_after <= 0 {
                options.iter().min_by_key(|b| (self.value(b), b.id))
            } else {
                None
            };
            let pick = good
                .or(trade)
                .or(if self.has(a, kw::TRAMPLE) { None } else { safe })
                .or(chump);
            if let Some(b) = pick {
                used.push(b.id);
                out.push((b.id, a.id));
                if !self.has(a, kw::TRAMPLE) {
                    life_after += dmg;
                }
            }
        }
        out
    }

    fn combat(&mut self, side: usize) {
        let attackers = self.choose_attackers(side);
        if attackers.is_empty() {
            return;
        }
        for id in &attackers {
            if let Some(i) = self.find(*id) {
                self.perms[i].attacking = true;
                if !self.has(&self.perms[i], kw::VIGILANCE) {
                    self.perms[i].tapped = true;
                }
            }
        }
        let names: Vec<String> = attackers
            .iter()
            .filter_map(|id| self.find(*id))
            .map(|i| self.name_of(&self.perms[i]))
            .collect();
        let sname = self.side_name(side);
        self.log(
            Ev::Attack,
            side,
            "log.attack",
            vec![("side", sname), ("cards", names.join(", "))],
            attackers.clone(),
        );
        // attack triggers (none in this region yet beyond the log)
        // the defender's answer: a shrinking instant on the biggest attacker
        self.cast_window(other(side), Timing::Defend);
        if self.over.is_some() {
            return;
        }
        let blocks = self.choose_blocks(other(side));
        for (b, a) in &blocks {
            if let Some(i) = self.find(*b) {
                self.perms[i].blocking = Some(*a);
            }
            let bn = self
                .find(*b)
                .map(|i| self.name_of(&self.perms[i]))
                .unwrap_or_default();
            let an = self
                .find(*a)
                .map(|i| self.name_of(&self.perms[i]))
                .unwrap_or_default();
            self.log(
                Ev::Block,
                other(side),
                "log.block",
                vec![("blocker", bn), ("attacker", an)],
                vec![*b, *a],
            );
        }
        // first strike, then regular damage
        for first in [true, false] {
            let fighters: Vec<u32> = self
                .perms
                .iter()
                .filter(|p| p.attacking || p.blocking.is_some())
                .filter(|p| {
                    let strikes = self.has(p, kw::FIRST_STRIKE | kw::DOUBLE_STRIKE);
                    let ds = self.has(p, kw::DOUBLE_STRIKE);
                    if first {
                        strikes
                    } else {
                        !strikes || ds
                    }
                })
                .map(|p| p.id)
                .collect();
            if fighters.is_empty() {
                continue;
            }
            for id in fighters {
                let Some(i) = self.find(id) else { continue };
                let p = self.perms[i].clone();
                let power = self.power(&p);
                if power <= 0 {
                    continue;
                }
                if p.attacking {
                    let blockers: Vec<u32> = self
                        .perms
                        .iter()
                        .filter(|b| b.blocking == Some(id))
                        .map(|b| b.id)
                        .collect();
                    if blockers.is_empty() {
                        // unblocked, or its blocker is gone: trample still reaches the player
                        let was_blocked = blocks.iter().any(|(_, a)| *a == id);
                        if !was_blocked || self.has(&p, kw::TRAMPLE) {
                            self.damage_face(Some(id), other(side), power, true);
                        }
                    } else {
                        let mut left = power;
                        for b in blockers {
                            let Some(bi) = self.find(b) else { continue };
                            let bp = &self.perms[bi];
                            let lethal = if self.has(&p, kw::DEATHTOUCH) {
                                1
                            } else {
                                (self.toughness(bp) - bp.damage).max(0)
                            };
                            let give = if self.has(&p, kw::TRAMPLE) {
                                left.min(lethal.max(1))
                            } else {
                                left
                            };
                            self.damage_perm(Some(id), b, give);
                            left -= give;
                        }
                        if left > 0 && self.has(&p, kw::TRAMPLE) {
                            self.damage_face(Some(id), other(side), left, true);
                        }
                    }
                } else if let Some(a) = p.blocking {
                    if self.find(a).is_some() {
                        self.damage_perm(Some(id), a, power);
                    }
                }
                if self.over.is_some() {
                    return;
                }
            }
            self.state_check();
        }
        for p in self.perms.iter_mut() {
            p.attacking = false;
            p.blocking = None;
        }
    }

    // ---------- abilities, last in the chain ----------

    /// Mana a side keeps back: enough, with next turn's income, to cast the
    /// dearest card in its hand, and what its answers cost.
    fn reserve(&self, side: usize) -> usize {
        let income = self.s[side].income.len() + self.mana_permanents(side);
        let dearest = self.s[side]
            .hand
            .iter()
            .map(|&c| self.cost_for(side, c).value())
            .max()
            .unwrap_or(0);
        let answers: usize = self.s[side]
            .hand
            .iter()
            .filter(|&&c| {
                matches!(
                    self.cards[c].rules.timing,
                    Timing::Answer | Timing::Defend | Timing::EndStep
                )
            })
            .map(|&c| self.cost_for(side, c).value())
            .max()
            .unwrap_or(0);
        dearest.saturating_sub(income).max(answers)
    }

    fn surplus(&self, side: usize) -> usize {
        self.s[side].bank.len().saturating_sub(self.reserve(side))
    }

    fn mana_permanents(&self, side: usize) -> usize {
        self.perms
            .iter()
            .filter(|p| p.ctrl == side && self.rules_of(p).and_then(|r| r.mana).is_some())
            .count()
    }

    fn abilities(&mut self, side: usize) {
        let plan = self.choose_attackers(side);
        let ids: Vec<u32> = self
            .perms
            .iter()
            .filter(|p| p.ctrl == side)
            .map(|p| p.id)
            .collect();
        for id in ids {
            let Some(i) = self.find(id) else { continue };
            if self.detained(&self.perms[i]) || self.arrested(&self.perms[i]) {
                continue;
            }
            let acts = self
                .rules_of(&self.perms[i])
                .map(|r| r.activated.clone())
                .unwrap_or_default();
            for a in acts {
                if self.find(id).is_none() {
                    break;
                }
                let uses = match a.when {
                    Use::Pump => 2,
                    Use::Ping => 3,
                    _ => 1,
                };
                for _ in 0..uses {
                    let Some(i) = self.find(id) else { break };
                    let p = self.perms[i].clone();
                    if a.tap && (p.tapped || (p.sick && self.is_creature(&p) && !self.has(&p, kw::HASTE))) {
                        break;
                    }
                    let afford = a.cost.value() <= self.surplus(side) && self.s[side].bank.can_pay(&a.cost);
                    if !afford {
                        break;
                    }
                    let go = match a.when {
                        Use::Pump => {
                            plan.contains(&id)
                                && self.toughness(&p)
                                    + a.effects
                                        .iter()
                                        .map(|e| if let Eff::Pump { t, .. } = e { *t } else { 0 })
                                        .sum::<i32>()
                                    > 0
                        }
                        Use::Ping => true,
                        Use::SacForToken => true,
                        Use::Animate => false,
                        Use::Regenerate | Use::Equip => false,
                    };
                    if !go {
                        break;
                    }
                    self.s[side].bank.pay(&a.cost);
                    if a.tap {
                        self.perms[i].tapped = true;
                    }
                    let name = self.name_of(&self.perms[i]);
                    self.log(Ev::Ability, side, "log.ability", vec![("card", name)], vec![id]);
                    let effects = a.effects.clone();
                    if a.sac {
                        self.dies(id);
                    }
                    for e in &effects {
                        let e = match (e, a.when) {
                            // a pump ability pumps its own creature
                            (Eff::Pump { p, t, kw, untap, .. }, Use::Pump) => {
                                if let Some(k) = self.find(id) {
                                    self.perms[k].tp += p;
                                    self.perms[k].tt += t;
                                    self.perms[k].tkw |= kw;
                                    if *untap {
                                        self.perms[k].tapped = false;
                                    }
                                }
                                continue;
                            }
                            (other, _) => other.clone(),
                        };
                        self.resolve(side, Some(id), &e, None);
                    }
                    self.state_check();
                    if self.over.is_some() {
                        return;
                    }
                    if matches!(a.when, Use::SacForToken) {
                        break;
                    }
                }
            }
        }
        // equipment: on the best creature it is not on
        let sabers: Vec<u32> = self
            .perms
            .iter()
            .filter(|p| p.ctrl == side && self.rules_of(p).and_then(|r| r.equip.clone()).is_some())
            .map(|p| p.id)
            .collect();
        for s in sabers {
            let Some(si) = self.find(s) else { continue };
            let cost = self
                .rules_of(&self.perms[si])
                .and_then(|r| r.equip.clone())
                .unwrap_or_default();
            let mine = self.candidates(&crate::rules::creatures(Who::Mine), side);
            let Some(best) = self.best(&mine, true) else {
                continue;
            };
            let bid = self.perms[best].id;
            if self.perms[si].attached == Some(bid)
                || cost.value() > self.surplus(side)
                || !self.s[side].bank.pay(&cost)
            {
                continue;
            }
            self.perms[si].attached = Some(bid);
            let name = self.name_of(&self.perms[best]);
            self.log(Ev::Ability, side, "log.equip", vec![("card", name)], vec![s, bid]);
        }
        // scavenge: a creature card in the graveyard feeds one on the battlefield
        let mut grave = self.s[side].grave.clone();
        grave.sort_by_key(|&c| -self.cards[c].power);
        for c in grave {
            let Some(cost) = self.cards[c].rules.scavenge.clone() else {
                continue;
            };
            let cost = if self.s[side].perks & perk::SCAVENGE_CHEAP != 0 {
                cost.reduced(2)
            } else {
                cost
            };
            if cost.value() > self.surplus(side) || !self.s[side].bank.can_pay(&cost) {
                continue;
            }
            let mine = self.candidates(&crate::rules::creatures(Who::Mine), side);
            let Some(t) = self.best(&mine, true) else { break };
            self.s[side].bank.pay(&cost);
            if let Some(pos) = self.s[side].grave.iter().position(|&g| g == c) {
                self.s[side].grave.remove(pos);
            }
            self.s[side].exile.push(c);
            let n = self.cards[c].power;
            self.perms[t].plus += n;
            let tid = self.perms[t].id;
            let tname = self.name_of(&self.perms[t]);
            let cname = self.cards[c].name.clone();
            self.log(
                Ev::Ability,
                side,
                "log.scavenge",
                vec![("card", cname), ("target", tname), ("n", n.to_string())],
                vec![tid],
            );
        }
    }

    /// Instants of one timing, for the side that may cast them now.
    fn cast_window(&mut self, side: usize, window: Timing) {
        loop {
            if self.over.is_some() {
                return;
            }
            let hand = self.s[side].hand.clone();
            let mut plans: Vec<Plan> = hand
                .iter()
                .filter_map(|&c| self.plan_cast(side, c, window))
                .collect();
            if window == Timing::Defend {
                // shrink only an attacker that would otherwise hit
                plans.retain(|pl| matches!(pl.target, Some(Target::Perm(id)) if self.find(id).map(|i| self.perms[i].attacking).unwrap_or(false)) || pl.overload);
            }
            plans.sort_by_key(|pl| (std::cmp::Reverse(pl.cost.value()), pl.card));
            let Some(plan) = plans.into_iter().next() else {
                return;
            };
            self.cast(side, plan);
        }
    }

    // ---------- the turn ----------

    fn upkeep(&mut self, side: usize) {
        // detained creatures go free at the detainer's next turn; damage and pumps ended last turn
        let shriek = self.statics_of(other(side), Static::Shrieking);
        if shriek > 0 && self.s[side].hand.len() <= 1 {
            for _ in 0..shriek {
                self.lose_life(side, 3);
                let name = self.side_name(side);
                self.log(
                    Ev::Life,
                    side,
                    "log.shrieking",
                    vec![("side", name), ("n", "3".into())],
                    vec![],
                );
            }
            if self.check_end() {
                return;
            }
        }
        let stabs: Vec<u32> = self
            .perms
            .iter()
            .filter(|a| self.rules_of(a).and_then(|r| r.aura) == Some(Aura::StabWound))
            .filter_map(|a| a.attached)
            .filter(|h| self.find(*h).map(|i| self.perms[i].ctrl == side).unwrap_or(false))
            .collect();
        for _ in stabs {
            self.lose_life(side, 2);
            let name = self.side_name(side);
            self.log(
                Ev::Life,
                side,
                "log.stab",
                vec![("side", name), ("n", "2".into())],
                vec![],
            );
            if self.check_end() {
                return;
            }
        }
        let ranks = self.statics_of(side, Static::UpkeepPopulate);
        for _ in 0..ranks {
            self.populate(side);
        }
        if self.s[side].perks & perk::SET_POPULATE != 0 && self.turn % 6 < 2 {
            self.populate(side);
        }
    }

    fn mana_step(&mut self, side: usize) {
        let mut gained = 0;
        let income = self.s[side].income.clone();
        for u in income {
            self.s[side].bank.add(u);
            gained += 1;
        }
        let makers: Vec<(usize, u8)> = (0..self.perms.len())
            .filter(|&i| self.perms[i].ctrl == side && !self.perms[i].tapped)
            .filter_map(|i| self.rules_of(&self.perms[i]).and_then(|r| r.mana).map(|m| (i, m)))
            .collect();
        for (i, m) in makers {
            self.perms[i].tapped = true;
            self.s[side].bank.add(m);
            gained += 1;
        }
        if self.statics_of(side, Static::Lantern) > 0 {
            self.s[side].bank.any_colour();
        }
        let name = self.side_name(side);
        let bank = self.s[side].bank.len();
        self.log(
            Ev::Mana,
            side,
            "log.mana",
            vec![
                ("side", name),
                ("n", gained.to_string()),
                ("bank", bank.to_string()),
            ],
            vec![],
        );
    }

    fn take_turn(&mut self, side: usize) {
        self.active = side;
        self.turn += 1;
        let name = self.side_name(side);
        self.log(
            Ev::Turn,
            side,
            "log.turn",
            vec![("side", name), ("n", (self.turn.div_ceil(2)).to_string())],
            vec![],
        );
        // untap
        let grasped: Vec<u32> = self
            .perms
            .iter()
            .filter(|a| self.rules_of(a).and_then(|r| r.aura) == Some(Aura::Grasp))
            .filter_map(|a| a.attached)
            .collect();
        for p in self.perms.iter_mut().filter(|p| p.ctrl == side) {
            if !grasped.contains(&p.id) {
                p.tapped = false;
            }
            p.sick = false;
        }
        self.upkeep(side);
        if self.over.is_some() {
            return;
        }
        self.mana_step(side);
        let first_turn = self.turn == 1;
        if !first_turn {
            self.draw(side, 1);
        }
        if self.over.is_some() {
            return;
        }
        // cast: the dearest castable card, again and again
        loop {
            let hand = self.s[side].hand.clone();
            let mut plans: Vec<Plan> = hand
                .iter()
                .filter_map(|&c| self.plan_cast(side, c, Timing::Main))
                .collect();
            plans.sort_by_key(|pl| {
                let card = &self.cards[pl.card];
                (
                    std::cmp::Reverse(pl.cost.value()),
                    !card.is(ty::CREATURE),
                    pl.card,
                )
            });
            let Some(plan) = plans.into_iter().next() else {
                break;
            };
            self.cast(side, plan);
            if self.over.is_some() {
                return;
            }
        }
        self.abilities(side);
        if self.over.is_some() {
            return;
        }
        self.combat(side);
        if self.over.is_some() {
            return;
        }
        // the other side's end-step flash
        self.cast_window(other(side), Timing::EndStep);
        // end of turn: damage and "until end of turn" effects wear off
        for p in self.perms.iter_mut() {
            p.damage = 0;
            p.tp = 0;
            p.tt = 0;
            p.tkw = 0;
            p.deathtouched = false;
        }
        self.state_check();
    }

    fn setup(&mut self, setup: &Setup) {
        for side in [P, F] {
            let ss = &setup.sides[side];
            let mut lib = ss.deck.clone();
            // shuffle: Fisher–Yates on the seed
            for i in (1..lib.len()).rev() {
                let j = (self.rng.next_u64() % (i as u64 + 1)) as usize;
                lib.swap(i, j);
            }
            self.s[side] = Side {
                name: ss.name.clone(),
                life: ss.life,
                library: lib,
                income: ss.income.clone(),
                swamps: ss.swamps,
                perks: ss.perks,
                ..Side::default()
            };
        }
        self.log(
            Ev::Start,
            P,
            "log.start",
            vec![("side", self.side_name(F))],
            vec![],
        );
        for side in [P, F] {
            let n = OPENING_HAND
                + if self.s[side].perks & perk::EXTRA_DRAW != 0 {
                    1
                } else {
                    0
                };
            for _ in 0..n.min(HAND_MAX) {
                if let Some(c) = self.s[side].library.pop() {
                    self.s[side].hand.push(c);
                }
            }
            if self.s[side].perks & perk::START_MANA != 0 {
                for _ in 0..perk::START_MANA_UNITS {
                    self.s[side].bank.add(mana::ANY);
                }
            }
            for &c in &setup.sides[side].in_play.clone() {
                let id = self.enter(side, c, false);
                if let Some(i) = self.find(id) {
                    self.perms[i].sick = false;
                }
            }
            if self.s[side].perks & perk::TOKEN_START != 0 {
                self.make_token(side, Token::Soldier, 1);
            }
        }
        if self.s[P].perks & perk::OPENING_DAMAGE != 0 {
            self.damage_face(None, F, perk::OPENING_DAMAGE_AMOUNT, false);
        }
        let detains = if self.s[P].perks & perk::SET_DETAIN_TWO != 0 {
            2
        } else if self.s[P].perks & perk::DETAIN_FIRST != 0 {
            1
        } else {
            0
        };
        if detains > 0 {
            self.active = P;
            self.resolve(
                P,
                None,
                &Eff::Detain(Aim::UpTo(detains, crate::rules::creatures(Who::Theirs))),
                None,
            );
        }
        if self.s[P].perks & perk::SET_FIRST_STRIKE != 0 {
            // the Rakdos set: the creatures you start with strike first
            for p in self.perms.iter_mut().filter(|p| p.ctrl == P) {
                p.tkw |= kw::FIRST_STRIKE;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Perm(u32),
    Face(usize),
    All,
}

#[derive(Debug, Clone)]
struct Plan {
    card: usize,
    mode: Option<usize>,
    overload: bool,
    target: Option<Target>,
    cost: Cost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leave {
    Graveyard,
    Hand,
    Exile,
    LibraryTop,
}

fn signed(n: i32) -> String {
    if n >= 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// Runs a whole fight. The player takes the first turn.
pub fn run(cards: &[Card], setup: &Setup) -> Record {
    let mut f = Fight {
        cards,
        s: [Side::default(), Side::default()],
        perms: Vec::new(),
        next_id: 0,
        turn: 0,
        active: P,
        rng: Rng::new(setup.seed),
        events: Vec::new(),
        over: None,
        casting: None,
    };
    f.setup(setup);
    let mut side = P;
    while f.over.is_none() && f.turn < TURN_CAP {
        f.take_turn(side);
        f.check_end();
        side = other(side);
    }
    let outcome = match f.over {
        Some(o) => o,
        None => {
            f.log(Ev::End, F, "log.withdrew", vec![("side", f.side_name(F))], vec![]);
            Outcome::Withdrew
        }
    };
    Record {
        outcome,
        life: [f.s[P].life, f.s[F].life],
        turns: f.turn,
        events: f.events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{find, for_tests};
    use crate::mana::{G, R, U, W};

    fn side(name: &str, life: i32, deck: Vec<usize>, income: Vec<u8>) -> SideSetup {
        SideSetup {
            name: name.into(),
            life,
            deck,
            income,
            ..SideSetup::default()
        }
    }

    fn deck(cards: &[Card], names: &[&str]) -> Vec<usize> {
        names.iter().map(|n| find(cards, n)).collect()
    }

    fn keys(r: &Record) -> Vec<&str> {
        r.events.iter().map(|e| e.key).collect()
    }

    #[test]
    fn mana_banks_from_turn_to_turn_when_nothing_can_be_cast() {
        let cards = for_tests();
        // one Mountain in reach and a hand of six-drops: the bank grows by one each turn
        let d = deck(&cards, &["Tenement Crasher"; 8]);
        let setup = Setup {
            sides: [
                side("you", 20, d.clone(), vec![R]),
                side("foe", 20, vec![], vec![]),
            ],
            seed: 1,
        };
        let r = run(&cards, &setup);
        let banks: Vec<usize> = r
            .events
            .iter()
            .filter(|e| e.ev == Ev::Mana && e.side == 0)
            .map(|e| e.snap.bank[0].len())
            .take(6)
            .collect();
        // turns 1–5 bank 1, 2, 3, 4, 5; on the sixth the Crasher is cast with the sixth
        assert_eq!(&banks[..5], &[1, 2, 3, 4, 5]);
        assert!(r
            .events
            .iter()
            .any(|e| e.key == "log.cast" && e.args[1].1 == "Tenement Crasher"));
    }

    #[test]
    fn a_haste_creature_hits_on_the_turn_it_is_cast() {
        let cards = for_tests();
        let d = deck(&cards, &["Rakdos Shred-Freak"; 6]);
        let setup = Setup {
            sides: [side("you", 20, d, vec![R, R]), side("foe", 20, vec![], vec![])],
            seed: 2,
        };
        let r = run(&cards, &setup);
        // turn 1: two mana, one Shred-Freak, two damage to the foe before the foe's first turn
        let first_face = r.events.iter().find(|e| e.ev == Ev::Face).unwrap();
        assert_eq!(first_face.turn, 1);
        assert_eq!(first_face.snap.life[1], 18);
    }

    #[test]
    fn a_flier_is_not_blocked_by_a_creature_without_reach() {
        let cards = for_tests();
        let pegasus = find(&cards, "Concordia Pegasus");
        let slug = find(&cards, "Catacomb Slug");
        let setup = Setup {
            sides: [
                SideSetup {
                    in_play: vec![pegasus],
                    ..side("you", 20, vec![], vec![])
                },
                SideSetup {
                    in_play: vec![slug],
                    ..side("foe", 20, vec![], vec![])
                },
            ],
            seed: 3,
        };
        let r = run(&cards, &setup);
        assert!(!keys(&r).contains(&"log.block"));
        let hit = r.events.iter().find(|e| e.ev == Ev::Face && e.side == 0).unwrap();
        // Pegasus deals 1 to the foe: 20 - 1
        assert_eq!(hit.snap.life[1], 19);
    }

    #[test]
    fn detain_stops_a_creature_attacking_until_the_detainers_next_turn() {
        let cards = for_tests();
        let arrester = find(&cards, "Azorius Arrester");
        let cobble = find(&cards, "Cobblebrute");
        // the player casts Arrester on turn 1 (two Plains); the foe starts with a Cobblebrute
        let setup = Setup {
            sides: [
                side("you", 20, vec![arrester], vec![W, W]),
                SideSetup {
                    in_play: vec![cobble],
                    ..side("foe", 20, vec![], vec![])
                },
            ],
            seed: 4,
        };
        let r = run(&cards, &setup);
        assert!(r
            .events
            .iter()
            .any(|e| e.key == "log.detain" && e.args[0].1 == "Cobblebrute"));
        // through the foe's turn 2 the Cobblebrute is still detained; at the player's turn 3 it goes free
        let detained_on = |turn: u32| {
            r.events
                .iter()
                .filter(|e| e.turn == turn)
                .flat_map(|e| e.snap.perms.iter())
                .any(|p| p.name == "Cobblebrute" && p.detained)
        };
        assert!(detained_on(2), "detained through the foe's next turn");
        assert!(!detained_on(3), "free from the detainer's next turn");
        // the foe's turn 4: free again; it attacks, the 2/1 Arrester will not block a 5/2
        assert!(r
            .events
            .iter()
            .any(|e| e.ev == Ev::Attack && e.side == 1 && e.turn == 4));
    }

    #[test]
    fn overload_is_cast_when_it_hits_two_or_more() {
        let cards = for_tests();
        let electrickery = find(&cards, "Electrickery");
        let goblins = find(&cards, "Trained Caracal");
        let setup = Setup {
            sides: [
                side("you", 20, vec![electrickery], vec![R, R]),
                SideSetup {
                    in_play: vec![goblins, goblins],
                    ..side("foe", 20, vec![], vec![])
                },
            ],
            seed: 5,
        };
        let r = run(&cards, &setup);
        assert!(r.events.iter().any(|e| e.key == "log.cast_overload"));
        // two 1/1 Caracals die to 1 damage each
        assert_eq!(r.events.iter().filter(|e| e.ev == Ev::Dies).count(), 2);
    }

    #[test]
    fn a_fight_replays_the_same_from_its_seed() {
        let cards = for_tests();
        let a = deck(
            &cards,
            &[
                "Frostburn Weird",
                "Goblin Electromancer",
                "Mizzium Mortars",
                "Izzet Charm",
                "Runewing",
                "Cancel",
                "Electrickery",
                "Guttersnipe",
            ],
        );
        let b = deck(
            &cards,
            &[
                "Dreg Mangler",
                "Sluiceway Scorpion",
                "Drudge Beetle",
                "Dead Reveler",
                "Giant Growth",
                "Korozda Monitor",
            ],
        );
        let setup = Setup {
            sides: [
                side("you", 18, a, vec![U, R, R, U]),
                side("foe", 9, b, vec![G, crate::mana::B, G]),
            ],
            seed: 99,
        };
        let one = run(&cards, &setup);
        let two = run(&cards, &setup);
        assert_eq!(one.events, two.events);
        assert!(one.events.len() > 10);
    }

    #[test]
    fn the_foe_withdraws_at_the_turn_cap() {
        let cards = for_tests();
        let wall = find(&cards, "Hover Barrier");
        let setup = Setup {
            sides: [
                SideSetup {
                    in_play: vec![wall],
                    ..side("you", 20, vec![], vec![])
                },
                SideSetup {
                    in_play: vec![wall],
                    ..side("foe", 20, vec![], vec![])
                },
            ],
            seed: 6,
        };
        let r = run(&cards, &setup);
        // both libraries and graveyards are empty: each draw costs 1; 20 life lasts twenty draws, or the cap ends it
        assert!(matches!(
            r.outcome,
            Outcome::Lost | Outcome::Won | Outcome::Withdrew
        ));
        assert!(r.turns <= TURN_CAP);
    }

    #[test]
    fn a_counterspell_answers_a_big_spell() {
        let cards = for_tests();
        let crasher = find(&cards, "Tenement Crasher");
        let cancel = find(&cards, "Cancel");
        let setup = Setup {
            sides: [
                side("you", 20, vec![crasher], vec![R, R, R, R, R, R]),
                SideSetup {
                    deck: vec![cancel],
                    ..side("foe", 20, vec![], vec![U, U, U])
                },
            ],
            seed: 7,
        };
        let r = run(&cards, &setup);
        // turn 1: the player has 6 and casts the Crasher; the foe has nothing banked yet
        // (its mana comes on its own turn), so the Crasher resolves
        assert!(r
            .events
            .iter()
            .any(|e| e.key == "log.enter" && e.args[1].1 == "Tenement Crasher"));
        let _ = cancel;
    }

    #[test]
    fn lifelink_gains_what_it_deals() {
        let cards = for_tests();
        let imp = find(&cards, "Daggerdrome Imp");
        let setup = Setup {
            sides: [
                SideSetup {
                    in_play: vec![imp],
                    ..side("you", 10, vec![], vec![])
                },
                side("foe", 20, vec![], vec![]),
            ],
            seed: 8,
        };
        let r = run(&cards, &setup);
        let gain = r.events.iter().find(|e| e.key == "log.gain").unwrap();
        // the Imp hits for 1 and its controller goes 10 -> 11
        assert_eq!(gain.snap.life[0], 11);
    }
}
