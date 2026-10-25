//! A game of Magic between the player and a foe: the fight on a creature's
//! tile, and the fight with the guardian. The rules follow Magic's
//! Comprehensive Rules for what the region's cards need: turns and steps,
//! priority and the stack, casting and targets, triggered and activated
//! abilities, combat with first strike, double strike, trample, deathtouch
//! and lifelink, state-based actions, and the guild mechanics.
//!
//! Two changes guard against mana screw and flood (DECISIONS.md, MVP 2):
//!
//! - lands are not in the deck. Each side has a land pile: for the player,
//!   the lands placed on the map within Reach of the fight; for a foe, lands
//!   of its colours. Once a turn, in a main phase, a side may play any land
//!   from its pile. The deck is all spells;
//! - an opening hand is `OPENING_HAND` spells, with no mulligan.
//!
//! And one keeps small decks playable: drawing from an empty library
//! shuffles the graveyard back in and costs half the side's life
//! (`empty_draw_loss`).
//!
//! The engine waits on decisions. Every decision either side makes is a
//! `Pending` ask; the computer answers its own (`duel_ai.rs`) and the
//! player's come through `answer()`. Given its seed, both decks and the
//! player's answers, a game plays out the same everywhere (MTG-4).
//!
//! Everything that happens is logged as an `Event` with a snapshot of the
//! table, which the page plays back.

use std::rc::Rc;

use serde::Serialize;

use crate::content::{ty, Card};
use crate::mana::{self, Bank, Cost};
use crate::rng::Rng;

#[path = "duel_more.rs"]
mod more;
use crate::rules::{
    kw, Ability, Aura, Eff, Entry, Extra, Filter, GraveKind, OnCast, Release, Rules, SpellKind, Static, Tgt,
    Token, When, Who,
};

pub const OPENING_HAND: usize = 6;
pub const MAX_HAND: usize = 6;
/// Turns, both sides counted, before the foe withdraws.
pub const TURN_CAP: u32 = 50;
/// What drawing from an empty library costs: the graveyard is shuffled back
/// in, and the side loses half its life, rounded up (the person's rule, T2).
pub fn empty_draw_loss(life: i32) -> i32 {
    (life.max(0) + 1) / 2
}
/// Power and toughness of an animated keyrune.
pub const KEYRUNE_SIZE: i32 = 2;
/// Steps the engine takes before it gives up on a game that does not move.
const STEP_GUARD: usize = 200_000;

pub const P: usize = 0;
pub const F: usize = 1;

/// Rules a side brings from its armour and abilities (`player.rs`).
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
    pub const OPENING_DAMAGE_AMOUNT: i32 = 2;
    /// Generic mana scavenge costs less with SCAVENGE_CHEAP.
    pub const SCAVENGE_DISCOUNT: u8 = 2;
    /// SET_POPULATE: populate at the upkeep of every this-many of your turns.
    pub const POPULATE_EVERY: u32 = 3;
}

#[derive(Debug, Clone, Default)]
pub struct SideSetup {
    pub name: String,
    pub life: i32,
    /// The spells: no lands.
    pub deck: Vec<usize>,
    /// The land pile.
    pub lands: Vec<usize>,
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
pub enum Step {
    Upkeep,
    Draw,
    Main1,
    Combat,
    Attackers,
    Blockers,
    FirstDamage,
    Damage,
    EndCombat,
    Main2,
    End,
    Cleanup,
}

impl Step {
    pub fn key(self) -> &'static str {
        match self {
            Step::Upkeep => "step.upkeep",
            Step::Draw => "step.draw",
            Step::Main1 => "step.main1",
            Step::Combat => "step.combat",
            Step::Attackers => "step.attackers",
            Step::Blockers => "step.blockers",
            Step::FirstDamage => "step.first_damage",
            Step::Damage => "step.damage",
            Step::EndCombat => "step.end_combat",
            Step::Main2 => "step.main2",
            Step::End => "step.end",
            Step::Cleanup => "step.cleanup",
        }
    }

    fn next(self) -> Option<Step> {
        Some(match self {
            Step::Upkeep => Step::Draw,
            Step::Draw => Step::Main1,
            Step::Main1 => Step::Combat,
            Step::Combat => Step::Attackers,
            Step::Attackers => Step::Blockers,
            Step::Blockers => Step::FirstDamage,
            Step::FirstDamage => Step::Damage,
            Step::Damage => Step::EndCombat,
            Step::EndCombat => Step::Main2,
            Step::Main2 => Step::End,
            Step::End => Step::Cleanup,
            Step::Cleanup => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Ev {
    Start,
    Turn,
    Step,
    Land,
    Draw,
    Cast,
    Resolve,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Outcome {
    Won,
    Lost,
    /// The turn cap came: the foe withdrew.
    Withdrew,
}

/// A permanent on the battlefield.
#[derive(Debug, Clone)]
pub struct Perm {
    pub id: u32,
    pub card: Option<usize>,
    pub token: Option<Token>,
    pub owner: usize,
    pub ctrl: usize,
    pub tapped: bool,
    /// Came under its controller's control since their turn began.
    pub fresh: bool,
    pub damage: i32,
    pub counters: i32,
    /// Until end of turn: +p/+t and keywords.
    pub mods: Vec<(i32, i32, u32)>,
    /// Keywords for the whole game, from perks.
    pub perk_kw: u32,
    pub deathtouched: bool,
    pub exile_if_dies: bool,
    pub regen: u8,
    pub attacking: bool,
    pub blocked: bool,
    pub blocking: Option<u32>,
    pub attached: Option<u32>,
    /// Detained by this player, until their next turn.
    pub detained_by: Option<usize>,
    /// A keyrune that is a creature until end of turn.
    pub animated: bool,
    pub unleashed: bool,
    /// Cards it exiled, returned when it leaves: (owner, card).
    pub held: Vec<(usize, usize)>,
    pub dealt_first: bool,
    /// A planeswalker's loyalty.
    pub loyalty: i32,
    /// Charge counters (Mana Bloom), filibuster counters (Azor's Elocutors).
    pub charge: i32,
    /// An Ooze token's size.
    pub size: i32,
    /// Controlled until end of turn: the side it goes back to (Traitorous Instinct).
    pub borrowed: Option<usize>,
    /// It dealt damage this turn (Avenging Arrow).
    pub dealt: bool,
    /// The planeswalker it attacks, when not the player.
    pub aim: Option<u32>,
    /// Vraska's ward, until her controller's next turn.
    pub ward: bool,
    /// A loyalty ability or a once-a-turn ability used this turn.
    pub used: bool,
    /// Colours it has gained (a Zombie returned by Grave Betrayal).
    pub extra_colours: u8,
    /// The card name chosen as it entered (Pithing Needle).
    pub named: String,
    /// The two colours chosen as it entered (Tablet of the Guilds).
    pub chosen_colours: u8,
}

/// Where a target is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Target {
    Perm(u32),
    Player(usize),
    /// A spell on the stack, by its id.
    Item(u32),
    /// A card in a graveyard: (owner, card).
    Grave(usize, usize),
    /// A card in the chooser's hand, by its place (a cost: discard it).
    Hand(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Spell {
        card: usize,
        mode: usize,
        overload: bool,
    },
    Ability {
        perm: u32,
        card: Option<usize>,
    },
    /// A copy of a spell (Nivix Guildmage): it resolves like the spell, and leaves no card.
    Copy {
        card: usize,
    },
}

/// A spell or ability on the stack.
#[derive(Debug, Clone)]
pub struct Item {
    pub id: u32,
    pub ctrl: usize,
    pub src: Source,
    pub ability: Ability,
    pub targets: Vec<Target>,
    /// Overloaded: "target" became "each".
    pub each: bool,
    /// The power of the creature that died (Death's Presence), or a scavenged card's power.
    pub x: i32,
    pub name: String,
    /// The controllers of permanent targets, read as it begins to resolve.
    pub tctrl: Vec<usize>,
}

/// A triggered ability waiting to go on the stack.
#[derive(Debug, Clone)]
pub struct Trig {
    pub ctrl: usize,
    pub perm: u32,
    pub card: Option<usize>,
    pub ability: Ability,
    pub x: i32,
    pub name: String,
    /// Targets fixed by what triggered it (the creature that dealt the damage).
    pub fixed: Vec<Target>,
}

/// What a side may do when it holds priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Play the land at this place in the land pile.
    Land(usize),
    /// Cast the card at this place in hand, in this mode, overloaded or not.
    Cast {
        hand: usize,
        mode: usize,
        overload: bool,
    },
    /// Activate a permanent's ability.
    Activate {
        perm: u32,
        n: usize,
    },
    /// Scavenge the card at this place in the graveyard.
    Scavenge {
        grave: usize,
    },
    /// Activate an ability of the card at this place in the graveyard (Jarad).
    GraveAbility {
        grave: usize,
        n: usize,
    },
    Pass,
}

/// What happens once targets are chosen.
#[derive(Debug, Clone)]
pub enum Then {
    Cast {
        hand: usize,
        card: usize,
        mode: usize,
        overload: bool,
        /// X, once chosen.
        x: Option<i32>,
        /// The creature sacrificed to cast it, once chosen (Launch Party).
        pick: Option<Target>,
        /// Cast without paying, from exile (Epic Experiment, Jace).
        free: bool,
    },
    Activate {
        perm: u32,
        n: usize,
        x: Option<i32>,
        pick: Option<Target>,
    },
    Scavenge {
        grave: usize,
        card: usize,
    },
    Trigger(Trig),
}

#[derive(Debug, Clone)]
pub enum Ask {
    Priority,
    Targets {
        spec: Tgt,
        legal: Vec<Target>,
        min: usize,
        max: usize,
        chosen: Vec<Target>,
        then: Box<Then>,
    },
    Attackers {
        legal: Vec<u32>,
        chosen: Vec<u32>,
    },
    Blockers {
        legal: Vec<u32>,
        attackers: Vec<u32>,
        chosen: Vec<(u32, u32)>,
    },
    /// Enter with a +1/+1 counter?
    Unleash {
        perm: u32,
    },
    /// Discard this many more cards.
    Discard {
        n: usize,
    },
    Sacrifice {
        legal: Vec<u32>,
    },
    /// Return one of these, or sacrifice the Impostor.
    Impostor {
        legal: Vec<u32>,
        perm: u32,
    },
    /// Pay this much, or the spell is countered.
    Pay {
        n: u8,
        item: u32,
    },
    /// Choose X, up to this much.
    ChooseX {
        max: i32,
        then: Box<Then>,
    },
    /// Choose what pays a cost beyond mana: a creature to sacrifice, a card to discard.
    CostPick {
        legal: Vec<Target>,
        then: Box<Then>,
    },
    /// A "you may" with no target.
    May {
        what: May,
    },
    /// Name a card.
    Name {
        options: Vec<String>,
        perm: u32,
    },
    /// Take one of two piles (Jace).
    Piles {
        a: Vec<usize>,
        b: Vec<usize>,
    },
    /// Sacrifice a creature to tap Desecration Demon, or don't.
    Offer {
        legal: Vec<u32>,
        demon: u32,
    },
}

/// What a "you may" offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum May {
    /// Pay 2 life, or this shock land enters tapped.
    Shock(u32),
    /// Pay {1}, or sacrifice Transguild Promenade.
    Promenade(u32),
    /// Discard a card, to draw one (Viashino Racketeer).
    Rummage,
    /// Pay {B}: the target player discards (Drainpipe Vermin).
    Vermin(usize),
    /// Pay the enchanted permanent's mana value, or sacrifice it (Soul Tithe).
    Tithe(u32, u8),
}

#[derive(Debug, Clone)]
pub struct Pending {
    pub who: usize,
    pub ask: Ask,
}

/// An answer to a `Pending` ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Act(Action),
    Targets(Vec<Target>),
    Attack(Vec<u32>),
    /// Attackers, and those of them that attack a planeswalker: (attacker, planeswalker).
    AttackAt(Vec<u32>, Vec<(u32, u32)>),
    Block(Vec<(u32, u32)>),
    Yes(bool),
    /// A card in hand, or one of the listed permanents; None declines.
    Pick(Option<usize>),
}

#[derive(Debug, Clone)]
enum Resume {
    /// Go on resolving this item from its effect `next`.
    Effects { item: Box<Item>, next: usize },
    /// Finish a permanent's entering: its triggers.
    Enter { perm: u32 },
    /// Cleanup: discard down to the hand size, then end the turn.
    Cleanup,
    /// Draw a card (Viashino Racketeer, after its discard).
    Draw { who: usize },
}

#[derive(Debug, Clone, Default)]
pub struct Side {
    pub name: String,
    pub life: i32,
    pub library: Vec<usize>,
    pub hand: Vec<usize>,
    pub grave: Vec<usize>,
    pub exile: Vec<usize>,
    pub pile: Vec<usize>,
    pub land_played: bool,
    pub perks: u32,
    pub creatures_cast: u32,
    pub regrowth_used: bool,
    pub regen_used: bool,
    pub turns: u32,
    /// Life lost this turn.
    pub lost: i32,
    /// No combat damage to this side this turn (Druid's Deliverance).
    pub no_combat_damage: bool,
    /// Damage still to be prevented this turn (Security Blockade).
    pub shield: i32,
    /// Jace's +1: attackers against this side get -1/-0, until its next turn.
    pub jace: bool,
    /// Extra turns to take after this one.
    pub extra_turns: u32,
    /// Creature cards to return under this side's control at the next end step (Grave Betrayal): (owner, card).
    pub betrayal: Vec<(usize, usize)>,
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
    pub land: bool,
    pub attached: i32,
    pub counters: i32,
    pub fresh: bool,
    /// A planeswalker's loyalty; 0 for anything else.
    pub loyalty: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snap {
    pub life: [i32; 2],
    pub hand: [usize; 2],
    pub library: [usize; 2],
    pub grave: [usize; 2],
    pub pile: [usize; 2],
    /// The player's hand, as card indices.
    pub player_hand: Vec<usize>,
    pub perms: Vec<PermSnap>,
    /// The stack, bottom first: (side, name).
    /// Each item on the stack: its controller, its name, and the card it
    /// comes from (-1 for none), bottom first.
    pub stack: Vec<(u8, String, i32)>,
    /// What each item on the stack aims at, bottom first: the table draws
    /// an arc from the top item's source to each (T35, after MTG Arena).
    pub aims: Vec<Aims>,
    pub step: Step,
    pub active: u8,
}

/// Where a stack item comes from and what it aims at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Aims {
    /// The permanent whose ability it is; none for a spell.
    pub from: Option<u32>,
    pub perms: Vec<u32>,
    /// Players aimed at, by side.
    pub players: Vec<u8>,
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

#[derive(Debug, Clone)]
pub struct Duel {
    pub cards: Rc<Vec<Card>>,
    pub s: [Side; 2],
    pub perms: Vec<Perm>,
    pub stack: Vec<Item>,
    /// Turns begun, both sides counted.
    pub turn: u32,
    pub active: usize,
    pub step: Step,
    pub priority: usize,
    passes: u8,
    pub pending: Option<Pending>,
    resume: Vec<Resume>,
    triggers: Vec<Trig>,
    next_id: u32,
    rng: Rng,
    /// Which sides answer through `answer()` rather than the computer.
    pub human: [bool; 2],
    pub events: Vec<Event>,
    pub outcome: Option<Outcome>,
    /// The player's answers, in order: with the seed, the game replays.
    pub answers: u32,
    /// Cards to cast without paying, in order: (caster, card).
    free_casts: Vec<(usize, usize)>,
    /// The player's draft of attackers aimed at planeswalkers.
    pub draft_aims: Vec<(u32, u32)>,
    /// Combat damage is being dealt (Druid's Deliverance prevents only that).
    in_combat_damage: bool,
    /// Who made the pending discard happen (Loxodon Smiter cares).
    forcer: usize,
}

fn other(side: usize) -> usize {
    1 - side
}

fn signed(n: i32) -> String {
    if n >= 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

impl Duel {
    pub fn new(cards: Rc<Vec<Card>>, setup: &Setup, human: [bool; 2]) -> Duel {
        let mut d = Duel {
            cards,
            s: [Side::default(), Side::default()],
            perms: Vec::new(),
            stack: Vec::new(),
            turn: 0,
            active: P,
            step: Step::Upkeep,
            priority: P,
            passes: 0,
            pending: None,
            resume: Vec::new(),
            triggers: Vec::new(),
            next_id: 1,
            rng: Rng::new(setup.seed),
            human,
            events: Vec::new(),
            outcome: None,
            answers: 0,
            free_casts: Vec::new(),
            draft_aims: Vec::new(),
            in_combat_damage: false,
            forcer: P,
        };
        d.start(setup);
        d
    }

    pub fn card(&self, c: usize) -> &Card {
        &self.cards[c]
    }

    pub fn rules(&self, c: usize) -> &Rules {
        &self.cards[c].rules
    }

    fn id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ---------- setup ----------

    fn start(&mut self, setup: &Setup) {
        for side in [P, F] {
            let st = &setup.sides[side];
            let mut library = st.deck.clone();
            self.shuffle(&mut library);
            self.s[side] = Side {
                name: st.name.clone(),
                life: st.life,
                library,
                pile: st.lands.clone(),
                perks: st.perks,
                ..Side::default()
            };
        }
        self.log(
            Ev::Start,
            P as u8,
            "log.start",
            vec![("side", self.s[F].name.clone())],
            vec![],
        );
        for side in [P, F] {
            let n = OPENING_HAND + usize::from(self.s[side].perks & perk::EXTRA_DRAW != 0);
            for _ in 0..n {
                if let Some(c) = self.s[side].library.pop() {
                    self.s[side].hand.push(c);
                }
            }
            for &c in &setup.sides[side].in_play {
                let id = self.make_perm(side, c);
                self.set(id, |p| p.fresh = false);
                if let Some(l) = self.rules(c).loyalty {
                    self.set(id, |p| p.loyalty = l);
                }
                self.log(
                    Ev::Enter,
                    side as u8,
                    "log.enter",
                    vec![
                        ("card", self.card(c).name.clone()),
                        ("side", self.s[side].name.clone()),
                    ],
                    vec![id],
                );
            }
            if self.s[side].perks & perk::START_MANA != 0 && !self.s[side].pile.is_empty() {
                let land = self.best_pile_land(side);
                self.play_land(side, land);
            }
            if self.s[side].perks & perk::TOKEN_START != 0 {
                self.make_token(side, Token::Soldier, 1);
            }
            if self.s[side].perks & perk::SET_FIRST_STRIKE != 0 {
                for p in self.perms.iter_mut().filter(|p| p.ctrl == side) {
                    p.perk_kw |= kw::FIRST_STRIKE;
                }
            }
        }
        if self.s[P].perks & perk::OPENING_DAMAGE != 0 {
            self.damage_player(None, "", P, F, perk::OPENING_DAMAGE_AMOUNT);
        }
        let detains = if self.s[P].perks & perk::SET_DETAIN_TWO != 0 {
            2
        } else {
            usize::from(self.s[P].perks & perk::DETAIN_FIRST != 0)
        };
        if detains > 0 {
            let mut theirs: Vec<u32> = self
                .perms
                .iter()
                .filter(|p| p.ctrl == F && self.is_creature(p))
                .map(|p| p.id)
                .collect();
            theirs.sort_by_key(|&id| std::cmp::Reverse(self.value_of(id)));
            for id in theirs.into_iter().take(detains) {
                self.detain(id, P);
            }
        }
        self.turn = 0;
        self.active = F;
        self.begin_turn();
    }

    fn shuffle(&mut self, v: &mut [usize]) {
        for i in (1..v.len()).rev() {
            let j = self.rng.below(i as u32 + 1) as usize;
            v.swap(i, j);
        }
    }

    /// The land the computer would play: the colour its hand needs most.
    pub fn best_pile_land(&self, side: usize) -> usize {
        let have: u8 = self
            .perms
            .iter()
            .filter(|p| p.ctrl == side)
            .filter_map(|p| p.card.and_then(|c| self.rules(c).mana))
            .fold(0, |m, u| m | u);
        let want: u8 = self.s[side]
            .hand
            .iter()
            .fold(0, |m, &c| m | self.card(c).cost.colours());
        let pile = &self.s[side].pile;
        (0..pile.len())
            .max_by_key(|&i| {
                let u = self.rules(pile[i]).mana.unwrap_or(0);
                let new = u & want & !have;
                let useful = u & want;
                (
                    mana::count_colours(new),
                    mana::count_colours(useful),
                    mana::count_colours(u),
                    std::cmp::Reverse(i),
                )
            })
            .unwrap_or(0)
    }

    // ---------- the log ----------

    pub fn snap(&self) -> Snap {
        let mut perms: Vec<PermSnap> = self
            .perms
            .iter()
            .map(|p| PermSnap {
                id: p.id,
                side: p.ctrl as u8,
                card: p.card.map(|c| c as i32).unwrap_or(-1),
                token: p.token.map(|t| t.model().to_string()).unwrap_or_default(),
                name: self.perm_name(p),
                power: self.power(p),
                toughness: self.toughness(p),
                damage: p.damage,
                tapped: p.tapped,
                attacking: p.attacking,
                blocking: p.blocking.map(|b| b as i32).unwrap_or(-1),
                detained: p.detained_by.is_some(),
                creature: self.is_creature(p),
                land: self.is_land(p),
                attached: p.attached.map(|a| a as i32).unwrap_or(-1),
                counters: p.counters,
                fresh: p.fresh,
                loyalty: if self.is_walker(p) { p.loyalty } else { 0 },
            })
            .collect();
        perms.sort_by_key(|p| p.id);
        Snap {
            life: [self.s[P].life, self.s[F].life],
            hand: [self.s[P].hand.len(), self.s[F].hand.len()],
            library: [self.s[P].library.len(), self.s[F].library.len()],
            grave: [self.s[P].grave.len(), self.s[F].grave.len()],
            pile: [self.s[P].pile.len(), self.s[F].pile.len()],
            player_hand: self.s[P].hand.clone(),
            perms,
            stack: self
                .stack
                .iter()
                .map(|i| {
                    let card = match i.src {
                        Source::Spell { card, .. } | Source::Copy { card } => Some(card),
                        Source::Ability { card, .. } => card,
                    };
                    (i.ctrl as u8, i.name.clone(), card.map(|c| c as i32).unwrap_or(-1))
                })
                .collect(),
            aims: self
                .stack
                .iter()
                .map(|i| Aims {
                    from: match i.src {
                        Source::Ability { perm, .. } => Some(perm),
                        _ => None,
                    },
                    perms: i
                        .targets
                        .iter()
                        .filter_map(|t| match t {
                            Target::Perm(id) => Some(*id),
                            _ => None,
                        })
                        .collect(),
                    players: i
                        .targets
                        .iter()
                        .filter_map(|t| match t {
                            Target::Player(side) => Some(*side as u8),
                            _ => None,
                        })
                        .collect(),
                })
                .collect(),
            step: self.step,
            active: self.active as u8,
        }
    }

    fn log(
        &mut self,
        ev: Ev,
        side: u8,
        key: &'static str,
        args: Vec<(&'static str, String)>,
        focus: Vec<u32>,
    ) {
        let snap = self.snap();
        self.events.push(Event {
            ev,
            side,
            turn: self.turn,
            key,
            args,
            focus,
            snap,
        });
    }

    pub fn side_name(&self, side: usize) -> String {
        self.s[side].name.clone()
    }

    pub fn perm_name(&self, p: &Perm) -> String {
        match (p.card, p.token) {
            (Some(c), _) => self.card(c).name.clone(),
            (None, Some(t)) => t.stats().4.to_string(),
            _ => String::new(),
        }
    }

    pub fn name_of(&self, id: u32) -> String {
        self.perm(id).map(|p| self.perm_name(p)).unwrap_or_default()
    }

    // ---------- permanents ----------

    pub fn perm(&self, id: u32) -> Option<&Perm> {
        self.perms.iter().find(|p| p.id == id)
    }

    fn set(&mut self, id: u32, f: impl FnOnce(&mut Perm)) {
        if let Some(p) = self.perms.iter_mut().find(|p| p.id == id) {
            f(p);
        }
    }

    fn blank(&mut self, side: usize, card: Option<usize>, token: Option<Token>) -> Perm {
        Perm {
            id: self.id(),
            card,
            token,
            owner: side,
            ctrl: side,
            tapped: false,
            fresh: true,
            damage: 0,
            counters: 0,
            mods: Vec::new(),
            perk_kw: 0,
            deathtouched: false,
            exile_if_dies: false,
            regen: 0,
            attacking: false,
            blocked: false,
            blocking: None,
            attached: None,
            detained_by: None,
            animated: false,
            unleashed: false,
            held: Vec::new(),
            dealt_first: false,
            loyalty: 0,
            charge: 0,
            size: 0,
            borrowed: None,
            dealt: false,
            aim: None,
            ward: false,
            used: false,
            extra_colours: 0,
            named: String::new(),
            chosen_colours: 0,
        }
    }

    fn make_perm(&mut self, side: usize, card: usize) -> u32 {
        let p = self.blank(side, Some(card), None);
        let id = p.id;
        self.perms.push(p);
        id
    }

    fn make_token(&mut self, side: usize, t: Token, n: u8) {
        for _ in 0..n {
            let p = self.blank(side, None, Some(t));
            let id = p.id;
            self.perms.push(p);
            self.log(
                Ev::Token,
                side as u8,
                "log.token",
                vec![("side", self.side_name(side)), ("token", t.stats().4.to_string())],
                vec![id],
            );
        }
    }

    pub fn is_creature(&self, p: &Perm) -> bool {
        p.token.is_some() || p.animated || p.card.map(|c| self.card(c).is(ty::CREATURE)).unwrap_or(false)
    }

    pub fn is_land(&self, p: &Perm) -> bool {
        p.card.map(|c| self.card(c).is(ty::LAND)).unwrap_or(false) && p.token.is_none()
    }

    pub fn is_walker(&self, p: &Perm) -> bool {
        p.card.map(|c| self.card(c).is(ty::PLANESWALKER)).unwrap_or(false)
    }

    fn is_artifact(&self, p: &Perm) -> bool {
        p.card.map(|c| self.card(c).is(ty::ARTIFACT)).unwrap_or(false)
    }

    fn is_enchantment(&self, p: &Perm) -> bool {
        p.card.map(|c| self.card(c).is(ty::ENCHANTMENT)).unwrap_or(false)
    }

    fn has_subtype(&self, p: &Perm, sub: &str) -> bool {
        p.card
            .map(|c| self.card(c).subtypes.iter().any(|t| t == sub))
            .unwrap_or(false)
    }

    /// Whether a side controls a Gate (Armory Guard, Ogre Jailbreaker).
    fn has_gate(&self, side: usize) -> bool {
        self.perms
            .iter()
            .any(|p| p.ctrl == side && self.has_subtype(p, "Gate"))
    }

    fn aura_of(&self, p: &Perm) -> Option<Aura> {
        p.card.and_then(|c| self.rules(c).aura)
    }

    /// Auras and equipment attached to this permanent.
    fn attachments(&self, id: u32) -> impl Iterator<Item = &Perm> {
        self.perms.iter().filter(move |a| a.attached == Some(id))
    }

    pub fn colours(&self, p: &Perm) -> u8 {
        let base = if p.animated {
            p.card
                .and_then(|c| self.rules(c).animate)
                .map(|a| a.1)
                .unwrap_or(0)
        } else {
            match (p.card, p.token) {
                (Some(c), _) => self.card(c).colours,
                (None, Some(t)) => t.stats().3,
                _ => 0,
            }
        };
        base | p.extra_colours
    }

    fn base_pt(&self, p: &Perm) -> (i32, i32) {
        if p.animated {
            return p
                .card
                .map(|c| self.rules(c).animate_pt)
                .unwrap_or((KEYRUNE_SIZE, KEYRUNE_SIZE));
        }
        if p.token == Some(Token::Ooze) {
            return (p.size, p.size);
        }
        let Some(c) = p.card else {
            return p.token.map(|t| (t.stats().0, t.stats().1)).unwrap_or((0, 0));
        };
        let r = self.rules(c);
        if r.statics.contains(&Static::RatCount) {
            let n = self
                .perms
                .iter()
                .filter(|q| q.ctrl == p.ctrl && self.has_subtype(q, "Rat"))
                .count() as i32;
            return (n, n);
        }
        if r.statics.contains(&Static::CreatureCount) {
            let n = self
                .perms
                .iter()
                .filter(|q| q.ctrl == p.ctrl && self.is_creature(q))
                .count() as i32;
            return (n, n);
        }
        (self.card(c).power, self.card(c).toughness)
    }

    fn statics_count(&self, side: usize, s: Static) -> i32 {
        self.perms
            .iter()
            .filter(|p| {
                p.ctrl == side
                    && p.card
                        .map(|c| self.rules(c).statics.contains(&s))
                        .unwrap_or(false)
            })
            .count() as i32
    }

    fn any_static(&self, s: Static) -> bool {
        self.statics_count(P, s) + self.statics_count(F, s) > 0
    }

    fn pt_mods(&self, p: &Perm) -> (i32, i32) {
        let mut dp = p.counters;
        let mut dt = p.counters;
        for (a, b, _) in &p.mods {
            dp += a;
            dt += b;
        }
        for a in self.attachments(p.id) {
            let (x, y) = match self.aura_of(a) {
                Some(Aura::StabWound) => (-2, -2),
                Some(Aura::EtherealArmor) => {
                    let n = self
                        .perms
                        .iter()
                        .filter(|e| e.ctrl == a.ctrl && self.is_enchantment(e))
                        .count() as i32;
                    (n, n)
                }
                Some(Aura::KnightlyValor) | Some(Aura::PursuitOfFlight) => (2, 2),
                Some(Aura::DeviantGlee) => (2, 1),
                Some(Aura::RighteousAuthority) => {
                    let n = self.s[p.ctrl].hand.len() as i32;
                    (n, n)
                }
                _ => (0, 0),
            };
            dp += x;
            dt += y;
            if a.card.map(|c| self.rules(c).equip.is_some()).unwrap_or(false) {
                dp += mana::count_colours(self.colours(p)) as i32;
            }
        }
        if p.token.is_some() {
            let n = self.statics_count(p.ctrl, Static::TokenAnthem);
            dp += n;
            dt += n;
        }
        if self.is_creature(p) {
            let n = self.statics_count(p.ctrl, Static::Blessing) * 3;
            dp += n;
            dt += n;
        }
        if p.card
            .map(|c| self.rules(c).statics.contains(&Static::GraveCreatures))
            .unwrap_or(false)
        {
            let n = self.s[p.ctrl]
                .grave
                .iter()
                .filter(|&&c| self.card(c).is(ty::CREATURE))
                .count() as i32;
            dp += n;
            dt += n;
        }
        (dp, dt)
    }

    pub fn power(&self, p: &Perm) -> i32 {
        self.base_pt(p).0 + self.pt_mods(p).0
    }

    pub fn toughness(&self, p: &Perm) -> i32 {
        self.base_pt(p).1 + self.pt_mods(p).1
    }

    pub fn kws(&self, p: &Perm) -> u32 {
        let mut k = p.perk_kw;
        if p.animated {
            k |= p
                .card
                .and_then(|c| self.rules(c).animate)
                .map(|a| a.0)
                .unwrap_or(0);
        } else {
            k |= match (p.card, p.token) {
                (Some(c), _) => self.rules(c).kw,
                (None, Some(t)) => t.stats().2,
                _ => 0,
            };
        }
        for (_, _, m) in &p.mods {
            k |= m;
        }
        for a in self.attachments(p.id) {
            match self.aura_of(a) {
                Some(Aura::EtherealArmor) => k |= kw::FIRST_STRIKE,
                Some(Aura::KnightlyValor) => k |= kw::VIGILANCE,
                _ => {}
            }
        }
        if let Some(c) = p.card {
            let r = self.rules(c);
            if r.statics.contains(&Static::VigilanceWithGate) && self.has_gate(p.ctrl) {
                k |= kw::VIGILANCE;
            }
            if r.statics.contains(&Static::TrampleWithCounter) && p.counters > 0 {
                k |= kw::TRAMPLE;
            }
        }
        if p.unleashed && p.counters > 0 && self.s[p.ctrl].perks & perk::UNLEASH_HASTE != 0 {
            k |= kw::HASTE;
        }
        k
    }

    fn has(&self, p: &Perm, k: u32) -> bool {
        self.kws(p) & k != 0
    }

    fn arrested(&self, p: &Perm) -> bool {
        self.attachments(p.id)
            .any(|a| self.aura_of(a) == Some(Aura::Arrest))
    }

    /// Can't use activated abilities: detained or arrested.
    fn silenced(&self, p: &Perm) -> bool {
        p.detained_by.is_some() || self.arrested(p)
    }

    /// A Pithing Needle names this permanent's card.
    fn needled(&self, p: &Perm) -> bool {
        let name = self.perm_name(p);
        self.perms.iter().any(|q| !q.named.is_empty() && q.named == name)
    }

    /// A rough worth, for the computer's choices and "best" picks.
    pub fn value_of(&self, id: u32) -> i32 {
        let Some(p) = self.perm(id) else { return 0 };
        let mv = p.card.map(|c| self.card(c).value as i32).unwrap_or(1);
        if self.is_walker(p) {
            return mv * 3 + p.loyalty * 2;
        }
        if !self.is_creature(p) {
            return mv * 2;
        }
        let k = self.kws(p);
        let mut v = self.power(p) * 3 + self.toughness(p) * 2 + mv;
        for (flag, w) in [
            (kw::FLYING, 3),
            (kw::FIRST_STRIKE, 2),
            (kw::DOUBLE_STRIKE, 4),
            (kw::DEATHTOUCH, 3),
            (kw::LIFELINK, 2),
            (kw::TRAMPLE, 1),
            (kw::UNBLOCKABLE, 3),
            (kw::HEXPROOF, 2),
        ] {
            if k & flag != 0 {
                v += w;
            }
        }
        if k & kw::DEFENDER != 0 {
            v -= 4;
        }
        v
    }

    // ---------- mana ----------

    /// The mana a side could make now: one entry a unit, (permanent, the
    /// colours the unit can be). Axebane Guardian makes several; Mana Bloom
    /// one a turn while it has charge counters.
    pub fn sources(&self, side: usize, except: Option<u32>) -> Vec<(u32, u8)> {
        let lantern = self.statics_count(side, Static::Lantern) > 0;
        let mut out = Vec::new();
        for p in self
            .perms
            .iter()
            .filter(|p| p.ctrl == side && Some(p.id) != except && !self.silenced(p))
        {
            let Some(c) = p.card else { continue };
            let r = self.rules(c);
            let Some(unit) = r.mana else { continue };
            let name = self.card(c).name.as_str();
            if name == "Mana Bloom" {
                if p.charge > 0 && !p.used {
                    out.push((p.id, mana::ANY));
                }
                continue;
            }
            if p.tapped || (self.is_creature(p) && p.fresh && !self.has(p, kw::HASTE)) {
                continue;
            }
            if name == "Axebane Guardian" {
                let n = self
                    .perms
                    .iter()
                    .filter(|q| q.ctrl == side && self.is_creature(q) && self.has(q, kw::DEFENDER))
                    .count();
                out.extend(std::iter::repeat_n((p.id, mana::ANY), n));
                continue;
            }
            let unit = if lantern && self.is_land(p) {
                mana::ANY
            } else {
                unit
            };
            out.push((p.id, unit));
        }
        out
    }

    pub fn can_pay(&self, side: usize, cost: &Cost, except: Option<u32>) -> bool {
        let src = self.sources(side, except);
        Bank {
            units: src.iter().map(|s| s.1).collect(),
        }
        .can_pay(&cost.with_x(0))
    }

    /// Taps sources to pay `cost`; false, with nothing tapped, if it can't.
    fn pay(&mut self, side: usize, cost: &Cost, except: Option<u32>) -> bool {
        let src = self.sources(side, except);
        let bank = Bank {
            units: src.iter().map(|s| s.1).collect(),
        };
        let Some(picks) = bank.plan(&cost.with_x(0)) else {
            return false;
        };
        let mut done: Vec<u32> = Vec::new();
        for i in picks {
            let id = src[i].0;
            if done.contains(&id) {
                continue;
            }
            done.push(id);
            let bloom = self
                .perm(id)
                .and_then(|p| p.card)
                .map(|c| self.card(c).name == "Mana Bloom")
                .unwrap_or(false);
            if bloom {
                self.set(id, |p| {
                    p.charge -= 1;
                    p.used = true;
                });
            } else {
                self.set(id, |p| p.tapped = true);
                self.tapped_for_mana(id);
            }
        }
        true
    }

    pub fn mana_available(&self, side: usize) -> usize {
        self.sources(side, None).len()
    }

    /// Mana sources a side has on the battlefield, tapped or not.
    pub fn mana_in_play(&self, side: usize) -> usize {
        self.perms
            .iter()
            .filter(|p| p.ctrl == side && p.card.and_then(|c| self.rules(c).mana).is_some())
            .count()
    }

    /// What a card costs `side` to cast now, overloaded or not, X still X.
    pub fn cast_cost(&self, side: usize, card: usize, overload: bool) -> Cost {
        let c = self.card(card);
        let base = if overload {
            c.rules.overload.clone().unwrap_or_default()
        } else {
            c.cost.clone()
        };
        if c.is_spell() {
            let off = self.statics_count(side, Static::SpellsCheaper)
                + i32::from(self.s[side].perks & perk::SET_SPELL_CHEAP != 0);
            base.reduced(off.max(0) as u8)
        } else if c.is(ty::CREATURE) && self.statics_count(side, Static::RakdosLord) > 0 {
            let lost = self.s[other(side)].lost.max(0);
            base.reduced((lost * self.statics_count(side, Static::RakdosLord)).min(255) as u8)
        } else {
            base
        }
    }

    /// The most X `side` could pay for this cost now.
    pub fn max_x(&self, side: usize, cost: &Cost) -> i32 {
        let xs = cost.xs() as i32;
        if xs == 0 {
            return 0;
        }
        let mut x = 0;
        while x < 20 && self.can_pay(side, &cost.with_x(x + 1), None) {
            x += 1;
        }
        x
    }

    fn scavenge_cost(&self, side: usize, card: usize) -> Option<Cost> {
        let c = self.rules(card).scavenge.clone()?;
        Some(if self.s[side].perks & perk::SCAVENGE_CHEAP != 0 {
            c.reduced(perk::SCAVENGE_DISCOUNT)
        } else {
            c
        })
    }

    // ---------- targets ----------

    fn matches(&self, f: &Filter, p: &Perm, side: usize, source: Option<u32>) -> bool {
        let who_ok = match f.who {
            Who::Mine => p.ctrl == side,
            Who::Theirs => p.ctrl != side,
            Who::Any => true,
        };
        if !who_ok {
            return false;
        }
        if f.land {
            return self.is_land(p);
        }
        if self.is_land(p) && !self.is_creature(p) {
            return false;
        }
        let walker = self.is_walker(p);
        if f.creature && !self.is_creature(p) && !(f.or_walker && walker) {
            return false;
        }
        if f.artifact && !self.is_artifact(p) {
            return false;
        }
        if f.enchantment && !self.is_enchantment(p) {
            return false;
        }
        if f.art_or_ench && !(self.is_artifact(p) || self.is_enchantment(p)) {
            return false;
        }
        if let Some(mv) = f.max_value {
            if p.card.map(|c| self.card(c).value).unwrap_or(0) > mv {
                return false;
            }
        }
        if let Some(mp) = f.min_power {
            if self.power(p) < mp {
                return false;
            }
        }
        if let Some(mt) = f.max_toughness {
            if self.toughness(p) > mt {
                return false;
            }
        }
        if let Some(fl) = f.flying {
            if self.has(p, kw::FLYING) != fl {
                return false;
            }
        }
        if f.mono && mana::count_colours(self.colours(p)) != 1 {
            return false;
        }
        if f.not_sphere
            && p.card
                .map(|c| self.card(c).name == "Detention Sphere")
                .unwrap_or(false)
        {
            return false;
        }
        if f.other && Some(p.id) == source {
            return false;
        }
        if f.combat && !(p.attacking || p.blocking.is_some()) {
            return false;
        }
        if f.attacking && !p.attacking {
            return false;
        }
        if f.blocking && p.blocking.is_none() {
            return false;
        }
        if f.damaged && !p.dealt {
            return false;
        }
        if f.nontoken && p.token.is_some() {
            return false;
        }
        true
    }

    /// The permanents an "each" with this filter reaches.
    fn each_of(&self, f: &Filter, side: usize) -> Vec<u32> {
        self.perms
            .iter()
            .filter(|p| self.matches(f, p, side, None))
            .map(|p| p.id)
            .collect()
    }

    /// Whether `side` may target this permanent at all: hexproof, and Oak
    /// Street Innkeeper's tapped creatures in their controller's opponent's turn.
    fn targetable(&self, side: usize, p: &Perm) -> bool {
        if p.ctrl == side {
            return true;
        }
        if self.has(p, kw::HEXPROOF) {
            return false;
        }
        !(p.tapped
            && self.is_creature(p)
            && self.active != p.ctrl
            && self.statics_count(p.ctrl, Static::TappedHexproof) > 0)
    }

    fn spell_fits(&self, side: usize, kind: SpellKind, i: &Item) -> bool {
        let card = match i.src {
            Source::Spell { card, .. } | Source::Copy { card } => card,
            _ => return false,
        };
        let c = self.card(card);
        match kind {
            SpellKind::Any => true,
            SpellKind::OnlyInstants => c.is(ty::INSTANT),
            SpellKind::InstantOrSorcery => c.is_spell(),
            SpellKind::Creature => c.is(ty::CREATURE),
            SpellKind::Noncreature => !c.is(ty::CREATURE),
            SpellKind::NotYours => i.ctrl != side,
        }
    }

    fn grave_fits(&self, kind: GraveKind, c: usize) -> bool {
        let k = self.card(c);
        match kind {
            GraveKind::Any => true,
            GraveKind::Creature => k.is(ty::CREATURE),
            GraveKind::Land => k.is(ty::LAND),
            GraveKind::InstantOrSorcery => k.is_spell(),
        }
    }

    /// What `side` may target with `spec`, from `source` (a permanent, for abilities).
    pub fn legal_targets(&self, side: usize, spec: &Tgt, source: Option<u32>) -> Vec<Target> {
        let perms = |f: &Filter| -> Vec<Target> {
            self.perms
                .iter()
                .filter(|p| self.matches(f, p, side, source) && self.targetable(side, p))
                .map(|p| Target::Perm(p.id))
                .collect()
        };
        let walkers = || -> Vec<Target> {
            self.perms
                .iter()
                .filter(|p| self.is_walker(p) && self.targetable(side, p))
                .map(|p| Target::Perm(p.id))
                .collect()
        };
        match spec {
            Tgt::One(f) | Tgt::UpTo(_, f) | Tgt::Twice(f) => perms(f),
            Tgt::Two(a, b) => {
                let mut v = perms(a);
                for t in perms(b) {
                    if !v.contains(&t) {
                        v.push(t);
                    }
                }
                v
            }
            Tgt::Any(f) => {
                let mut v = perms(f);
                for t in walkers() {
                    if !v.contains(&t) {
                        v.push(t);
                    }
                }
                v.push(Target::Player(other(side)));
                v.push(Target::Player(side));
                v
            }
            Tgt::Player => vec![Target::Player(other(side)), Target::Player(side)],
            Tgt::Opponent => vec![Target::Player(other(side))],
            Tgt::PlayerOrWalker => {
                let mut v = vec![Target::Player(other(side)), Target::Player(side)];
                v.extend(walkers());
                v
            }
            Tgt::Spell(kind) => self
                .stack
                .iter()
                .filter(|i| self.spell_fits(side, *kind, i))
                .map(|i| Target::Item(i.id))
                .collect(),
            Tgt::Grave(kind, who) => {
                let mut v = Vec::new();
                for owner in [P, F] {
                    let ok = match who {
                        Who::Mine => owner == side,
                        Who::Theirs => owner != side,
                        Who::Any => true,
                    };
                    if !ok {
                        continue;
                    }
                    let mut seen: Vec<usize> = Vec::new();
                    for &c in &self.s[owner].grave {
                        if self.grave_fits(*kind, c) && !seen.contains(&c) {
                            seen.push(c);
                            v.push(Target::Grave(owner, c));
                        }
                    }
                }
                v
            }
            Tgt::Angel => {
                let mut v: Vec<Target> = self
                    .perms
                    .iter()
                    .filter(|p| self.is_creature(p) && Some(p.id) != source && self.targetable(side, p))
                    .map(|p| Target::Perm(p.id))
                    .collect();
                for owner in [P, F] {
                    let mut seen: Vec<usize> = Vec::new();
                    for &c in &self.s[owner].grave {
                        if self.card(c).is(ty::CREATURE) && !seen.contains(&c) {
                            seen.push(c);
                            v.push(Target::Grave(owner, c));
                        }
                    }
                }
                v
            }
        }
    }

    fn spec_bounds(spec: &Tgt) -> (usize, usize) {
        match spec {
            Tgt::UpTo(n, _) => (0, *n as usize),
            Tgt::Angel => (0, 3),
            Tgt::Two(_, _) | Tgt::Twice(_) => (2, 2),
            _ => (1, 1),
        }
    }

    fn still_legal(&self, side: usize, spec: &Tgt, t: &Target, source: Option<u32>) -> bool {
        match t {
            Target::Player(_) => true,
            _ => self.legal_targets(side, spec, source).contains(t),
        }
    }

    // ---------- what a side may do ----------

    fn sorcery_time(&self, side: usize) -> bool {
        self.active == side && matches!(self.step, Step::Main1 | Step::Main2) && self.stack.is_empty()
    }

    fn mode_castable(&self, side: usize, card: usize, mode: usize, overload: bool) -> bool {
        let r = self.rules(card);
        if overload {
            return true;
        }
        if r.sac_to_cast && !self.perms.iter().any(|p| p.ctrl == side && self.is_creature(p)) {
            return false;
        }
        if r.statics.contains(&Static::RakdosLord) && self.s[other(side)].lost <= 0 {
            return false;
        }
        match r.modes.get(mode).and_then(|m| m.target) {
            Some(Tgt::Two(a, b)) => {
                let la = self.legal_targets(side, &Tgt::One(a), None);
                let lb = self.legal_targets(side, &Tgt::One(b), None);
                la.iter().any(|x| lb.iter().any(|y| x != y))
            }
            Some(spec) => {
                let (min, _) = Duel::spec_bounds(&spec);
                min == 0 || !self.legal_targets(side, &spec, None).is_empty()
            }
            None => true,
        }
    }

    /// Whether `side` may cast this card now, by its timing.
    fn in_time(&self, side: usize, card: usize, sorcery: bool) -> bool {
        let c = self.card(card);
        let fast = c.is(ty::INSTANT)
            || c.rules.kw & kw::FLASH != 0
            || (c.is(ty::SORCERY) && self.statics_count(side, Static::SorceryFlash) > 0);
        fast || sorcery
    }

    /// Everything `side` may do now with priority, Pass last.
    pub fn actions(&self, side: usize) -> Vec<Action> {
        let mut out = Vec::new();
        if self.outcome.is_some() {
            return vec![Action::Pass];
        }
        let sorcery = self.sorcery_time(side);
        if sorcery && !self.s[side].land_played {
            let mut names: Vec<usize> = Vec::new();
            for (i, &c) in self.s[side].pile.iter().enumerate() {
                if !names.contains(&c) {
                    names.push(c);
                    out.push(Action::Land(i));
                }
            }
        }
        for (i, &c) in self.s[side].hand.iter().enumerate() {
            let card = self.card(c);
            if card.is(ty::LAND) || !self.in_time(side, c, sorcery) {
                continue;
            }
            let modes = card.rules.modes.len().max(1);
            for m in 0..modes {
                if self.mode_castable(side, c, m, false)
                    && self.can_pay(side, &self.cast_cost(side, c, false), None)
                {
                    out.push(Action::Cast {
                        hand: i,
                        mode: m,
                        overload: false,
                    });
                }
            }
            if card.rules.overload.is_some() && self.can_pay(side, &self.cast_cost(side, c, true), None) {
                out.push(Action::Cast {
                    hand: i,
                    mode: 0,
                    overload: true,
                });
            }
        }
        for p in self.perms.iter().filter(|p| p.ctrl == side) {
            let Some(c) = p.card else { continue };
            for (n, a) in self.rules(c).activated.iter().enumerate() {
                if !a.from_grave && self.can_activate(side, p, a, sorcery) {
                    out.push(Action::Activate { perm: p.id, n });
                }
            }
        }
        // abilities that work from the graveyard (Jarad)
        for (g, &c) in self.s[side].grave.iter().enumerate() {
            if self.s[side].grave[..g].contains(&c) {
                continue;
            }
            for (n, a) in self.rules(c).activated.iter().enumerate() {
                if a.from_grave
                    && self.extra_payable(side, None, a.extra)
                    && self.can_pay(side, &a.cost, None)
                {
                    out.push(Action::GraveAbility { grave: g, n });
                }
            }
        }
        if sorcery {
            for (g, &c) in self.s[side].grave.iter().enumerate() {
                if let Some(cost) = self.scavenge_cost(side, c) {
                    let any_creature = self
                        .perms
                        .iter()
                        .any(|p| self.is_creature(p) && self.targetable(side, p));
                    if any_creature
                        && self.can_pay(side, &cost, None)
                        && !self.s[side].grave[..g].contains(&c)
                    {
                        out.push(Action::Scavenge { grave: g });
                    }
                }
            }
        }
        out.push(Action::Pass);
        out
    }

    /// What could pay a cost beyond mana: the creatures to sacrifice, the cards to discard.
    pub fn extra_options(&self, side: usize, source: Option<u32>, extra: Extra) -> Vec<Target> {
        let mine = |pred: &dyn Fn(&Perm) -> bool| -> Vec<Target> {
            self.perms
                .iter()
                .filter(|p| p.ctrl == side && Some(p.id) != source && pred(p))
                .map(|p| Target::Perm(p.id))
                .collect()
        };
        let hand = |pred: &dyn Fn(usize) -> bool| -> Vec<Target> {
            (0..self.s[side].hand.len())
                .filter(|&i| pred(self.s[side].hand[i]))
                .map(Target::Hand)
                .collect()
        };
        match extra {
            Extra::SacAnother => mine(&|p| self.is_creature(p)),
            Extra::SacNontoken => mine(&|p| self.is_creature(p) && p.token.is_none()),
            Extra::Discard => hand(&|_| true),
            Extra::DiscardCreature => hand(&|c| self.card(c).is(ty::CREATURE)),
            Extra::DiscardPair => hand(&|c| {
                !self.card(c).is(ty::LAND)
                    && self.s[side]
                        .hand
                        .iter()
                        .filter(|&&d| self.card(d).name == self.card(c).name)
                        .count()
                        >= 2
            }),
            _ => Vec::new(),
        }
    }

    /// Whether a cost beyond mana can be paid now.
    fn extra_payable(&self, side: usize, source: Option<&Perm>, extra: Extra) -> bool {
        let host = source.and_then(|p| p.attached).and_then(|h| self.perm(h));
        match extra {
            Extra::None => true,
            Extra::PayLife(n) => self.s[side].life >= n,
            Extra::TapTwo => {
                self.perms
                    .iter()
                    .filter(|p| {
                        p.ctrl == side
                            && self.is_creature(p)
                            && !p.tapped
                            && Some(p.id) != source.map(|s| s.id)
                    })
                    .count()
                    >= 2
            }
            Extra::SwampAndForest => {
                let has = |n: &str| {
                    self.perms.iter().any(|p| {
                        p.ctrl == side
                            && p.card
                                .map(|c| self.card(c).subtypes.iter().any(|t| t == n))
                                .unwrap_or(false)
                    })
                };
                has("Swamp") && has("Forest")
            }
            Extra::ExileOwnSpell => self.stack.iter().any(|i| {
                i.ctrl == side && matches!(i.src, Source::Spell { card, .. } if self.card(card).is_spell())
            }),
            Extra::TapHost => host.map(|h| !h.tapped).unwrap_or(false),
            Extra::TapHostPayLife => host.map(|h| !h.tapped).unwrap_or(false) && self.s[side].life >= 1,
            other => !self.extra_options(side, source.map(|s| s.id), other).is_empty(),
        }
    }

    fn can_activate(&self, side: usize, p: &Perm, a: &crate::rules::Activated, sorcery: bool) -> bool {
        if self.silenced(p) || (a.sorcery && !sorcery) {
            return false;
        }
        if a.loyalty.is_none() && self.needled(p) {
            return false;
        }
        if a.tap && (p.tapped || (self.is_creature(p) && p.fresh && !self.has(p, kw::HASTE))) {
            return false;
        }
        if a.ability.effects.contains(&Eff::Animate) && p.animated {
            return false;
        }
        if let Some(l) = a.loyalty {
            if p.used || p.loyalty + l < 0 {
                return false;
            }
        }
        if a.once && p.used {
            return false;
        }
        if !self.extra_payable(side, Some(p), a.extra) {
            return false;
        }
        if let Some(spec) = a.ability.target {
            let (min, _) = Duel::spec_bounds(&spec);
            if min > 0 && self.legal_targets(side, &spec, Some(p.id)).is_empty() {
                return false;
            }
        }
        let except = if a.tap { Some(p.id) } else { None };
        self.can_pay(side, &a.cost, except)
    }

    // ---------- the turn ----------

    fn begin_turn(&mut self) {
        self.turn += 1;
        // an extra turn: the same side again (Search the City)
        if self.s[self.active].extra_turns > 0 && self.turn > 1 {
            self.s[self.active].extra_turns -= 1;
        } else {
            self.active = other(self.active);
        }
        if self.turn > TURN_CAP {
            self.outcome = Some(Outcome::Withdrew);
            self.log(
                Ev::End,
                F as u8,
                "log.withdrew",
                vec![("side", self.side_name(F))],
                vec![],
            );
            return;
        }
        let a = self.active;
        self.s[a].land_played = false;
        self.s[a].turns += 1;
        self.s[a].jace = false;
        for side in [P, F] {
            self.s[side].lost = 0;
            self.s[side].no_combat_damage = false;
            self.s[side].shield = 0;
        }
        // untap; Urban Burgeoning's land untaps in every untap step
        let grasped: Vec<u32> = self
            .perms
            .iter()
            .filter(|p| p.attached.is_some() && self.aura_of(p) == Some(Aura::Grasp))
            .filter_map(|p| p.attached)
            .collect();
        let burgeoning: Vec<u32> = self
            .perms
            .iter()
            .filter(|p| p.attached.is_some() && self.aura_of(p) == Some(Aura::Burgeoning))
            .filter_map(|p| p.attached)
            .collect();
        for p in self.perms.iter_mut() {
            if p.ctrl == a {
                if !grasped.contains(&p.id) {
                    p.tapped = false;
                }
                p.fresh = false;
                p.used = false;
                p.ward = false;
            } else if burgeoning.contains(&p.id) {
                p.tapped = false;
            }
            if p.detained_by == Some(a) {
                p.detained_by = None;
            }
        }
        self.log(
            Ev::Turn,
            a as u8,
            "log.turn",
            vec![("side", self.side_name(a)), ("n", self.s[a].turns.to_string())],
            vec![],
        );
        self.enter_step(Step::Upkeep);
    }

    fn enter_step(&mut self, step: Step) {
        self.step = step;
        self.passes = 0;
        self.priority = self.active;
        let a = self.active;
        match step {
            Step::Upkeep => self.upkeep_triggers(),
            Step::Draw => {
                if self.turn > 1 {
                    self.draw(a, 1);
                }
                // Righteous Authority: an extra card for the enchanted creature's controller
                let extra = self
                    .perms
                    .iter()
                    .filter(|p| self.aura_of(p) == Some(Aura::RighteousAuthority))
                    .filter_map(|p| p.attached.and_then(|h| self.perm(h)))
                    .filter(|h| h.ctrl == a)
                    .count();
                self.draw(a, extra);
            }
            Step::Combat => self.combat_begins(),
            Step::Attackers => {
                let legal: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|p| self.can_attack(p))
                    .map(|p| p.id)
                    .collect();
                if legal.is_empty() {
                    self.enter_step(Step::EndCombat);
                } else {
                    // creatures that attack each combat if able are chosen already
                    let chosen: Vec<u32> = legal
                        .iter()
                        .copied()
                        .filter(|&id| self.perm(id).map(|p| self.must_attack(p)).unwrap_or(false))
                        .collect();
                    self.pending = Some(Pending {
                        who: a,
                        ask: Ask::Attackers { legal, chosen },
                    });
                }
            }
            Step::Blockers => {
                let attackers: Vec<u32> = self.perms.iter().filter(|p| p.attacking).map(|p| p.id).collect();
                if attackers.is_empty() {
                    self.enter_step(Step::EndCombat);
                    return;
                }
                let d = other(a);
                let legal: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|p| p.ctrl == d && attackers.iter().any(|&at| self.can_block(p, at)))
                    .map(|p| p.id)
                    .collect();
                if !legal.is_empty() {
                    self.pending = Some(Pending {
                        who: d,
                        ask: Ask::Blockers {
                            legal,
                            attackers,
                            chosen: Vec::new(),
                        },
                    });
                }
            }
            Step::FirstDamage => {
                let any = self.perms.iter().any(|p| {
                    (p.attacking || p.blocking.is_some())
                        && self.kws(p) & (kw::FIRST_STRIKE | kw::DOUBLE_STRIKE) != 0
                });
                if any {
                    self.combat_damage(true);
                } else {
                    self.enter_step(Step::Damage);
                }
            }
            Step::Damage => {
                if self.perms.iter().any(|p| p.attacking) {
                    self.combat_damage(false);
                } else {
                    self.enter_step(Step::EndCombat);
                }
            }
            Step::End => self.end_step_begins(),
            Step::EndCombat => {}
            Step::Cleanup => self.resume.push(Resume::Cleanup),
            _ => {}
        }
    }

    fn leave_step(&mut self) {
        if self.step == Step::EndCombat {
            for p in self.perms.iter_mut() {
                p.attacking = false;
                p.blocked = false;
                p.blocking = None;
                p.dealt_first = false;
                p.aim = None;
            }
        }
        match self.step.next() {
            Some(n) => {
                if self.step == Step::Main1 || n == Step::Main2 || n == Step::End {
                    self.log(Ev::Step, self.active as u8, n.key(), vec![], vec![]);
                }
                self.enter_step(n)
            }
            None => self.begin_turn(),
        }
    }

    fn upkeep_triggers(&mut self) {
        let a = self.active;
        let mut trigs = Vec::new();
        let empty = || Ability {
            target: None,
            effects: Vec::new(),
        };
        for p in &self.perms {
            let Some(c) = p.card else { continue };
            let r = self.rules(c);
            if p.ctrl == a {
                if let Some(ab) = &r.upkeep {
                    trigs.push(self.trig_of(p, ab.clone(), 0));
                }
            }
            for (w, ab) in &r.when {
                if *w == When::EachUpkeep {
                    let mut t = self.trig_of(p, ab.clone(), 0);
                    t.fixed = vec![Target::Player(a)];
                    trigs.push(t);
                }
            }
            if p.ctrl != a {
                if let Some((n, loss)) = r.opponent_upkeep {
                    if self.s[a].hand.len() <= n {
                        trigs.push(self.trig_of(p, empty(), loss));
                    }
                }
            }
            let host = p.attached.and_then(|h| self.perm(h));
            if r.aura == Some(Aura::StabWound) && host.map(|h| h.ctrl == a).unwrap_or(false) {
                trigs.push(self.trig_of(p, empty(), 2));
            }
            if r.aura == Some(Aura::SoulTithe) {
                if let Some(h) = host.filter(|h| h.ctrl == a) {
                    let mv = h.card.map(|c| self.card(c).value).unwrap_or(0) as u8;
                    let mut t = self.trig_of(
                        p,
                        Ability {
                            target: None,
                            effects: vec![Eff::TitheCheck],
                        },
                        mv as i32,
                    );
                    t.fixed = vec![Target::Perm(h.id)];
                    trigs.push(t);
                }
            }
        }
        if self.s[a].perks & perk::SET_POPULATE != 0 && self.s[a].turns.is_multiple_of(perk::POPULATE_EVERY) {
            trigs.push(Trig {
                ctrl: a,
                perm: 0,
                card: None,
                ability: Ability {
                    target: None,
                    effects: vec![Eff::Populate],
                },
                x: 0,
                name: String::new(),
                fixed: Vec::new(),
            });
        }
        self.triggers.extend(trigs);
    }

    fn trig_of(&self, p: &Perm, ability: Ability, x: i32) -> Trig {
        Trig {
            ctrl: p.ctrl,
            perm: p.id,
            card: p.card,
            ability,
            x,
            name: self.perm_name(p),
            fixed: Vec::new(),
        }
    }

    /// Attacks each combat if able: Volatile Rig, or a creature under Chemister's Trick.
    fn must_attack(&self, p: &Perm) -> bool {
        self.has(p, kw::MUST_ATTACK)
            || p.card
                .map(|c| self.rules(c).statics.contains(&Static::MustAttack))
                .unwrap_or(false)
    }

    pub fn can_attack(&self, p: &Perm) -> bool {
        let defender = self.has(p, kw::DEFENDER)
            && !(p
                .card
                .map(|c| self.rules(c).statics.contains(&Static::AttacksWithGate))
                .unwrap_or(false)
                && self.has_gate(p.ctrl));
        p.ctrl == self.active
            && self.is_creature(p)
            && !p.tapped
            && (!p.fresh || self.has(p, kw::HASTE))
            && !defender
            && p.detained_by.is_none()
            && !self.arrested(p)
    }

    /// Sphere of Safety: what each creature attacking `side` costs its controller.
    pub fn attack_tax(&self, side: usize) -> i32 {
        if self.statics_count(side, Static::AttackTax) == 0 {
            return 0;
        }
        self.perms
            .iter()
            .filter(|p| p.ctrl == side && self.is_enchantment(p))
            .count() as i32
            * self.statics_count(side, Static::AttackTax)
    }

    pub fn can_block(&self, b: &Perm, attacker: u32) -> bool {
        let Some(a) = self.perm(attacker) else {
            return false;
        };
        if !a.attacking
            || b.ctrl == a.ctrl
            || !self.is_creature(b)
            || b.tapped
            || b.detained_by.is_some()
            || self.arrested(b)
        {
            return false;
        }
        if b.unleashed && b.counters > 0 {
            return false;
        }
        if self.has(a, kw::UNBLOCKABLE) {
            return false;
        }
        if self.has(a, kw::INTIMIDATE) && !self.is_artifact(b) && self.colours(a) & self.colours(b) == 0 {
            return false;
        }
        if self.has(a, kw::FLYING) && !self.has(b, kw::FLYING) && !self.has(b, kw::REACH) {
            return false;
        }
        if self.has(a, kw::SWAMPWALK)
            && self
                .perms
                .iter()
                .any(|l| l.ctrl == b.ctrl && self.is_land(l) && self.has_subtype(l, "Swamp"))
        {
            return false;
        }
        true
    }

    fn draw(&mut self, side: usize, n: usize) {
        for _ in 0..n {
            if self.s[side].library.is_empty() {
                if self.s[side].grave.is_empty() {
                    continue;
                }
                let mut g = std::mem::take(&mut self.s[side].grave);
                self.shuffle(&mut g);
                self.s[side].library = g;
                let loss = empty_draw_loss(self.s[side].life);
                self.s[side].life -= loss;
                self.log(
                    Ev::Life,
                    side as u8,
                    "log.empty_library",
                    vec![("side", self.side_name(side)), ("n", loss.to_string())],
                    vec![],
                );
            }
            if let Some(c) = self.s[side].library.pop() {
                self.s[side].hand.push(c);
                if side == P {
                    self.log(
                        Ev::Draw,
                        side as u8,
                        "log.draw",
                        vec![
                            ("side", self.side_name(side)),
                            ("card", self.card(c).name.clone()),
                        ],
                        vec![],
                    );
                } else {
                    self.log(
                        Ev::Draw,
                        side as u8,
                        "log.draw_hidden",
                        vec![("side", self.side_name(side))],
                        vec![],
                    );
                }
            }
        }
    }

    fn play_land(&mut self, side: usize, at: usize) {
        if at >= self.s[side].pile.len() {
            return;
        }
        let c = self.s[side].pile.remove(at);
        let id = self.make_perm(side, c);
        self.s[side].land_played = true;
        self.log(
            Ev::Land,
            side as u8,
            "log.land",
            vec![
                ("side", self.side_name(side)),
                ("card", self.card(c).name.clone()),
            ],
            vec![id],
        );
        self.land_entered(side, id);
        let name = self.card(c).name.clone();
        self.played_name(side, &name);
    }

    // ---------- the engine loop ----------

    /// Runs until the game ends or waits on a human answer.
    pub fn run(&mut self) {
        for _ in 0..STEP_GUARD {
            if self.outcome.is_some() {
                self.pending = None;
                return;
            }
            if let Some(p) = &self.pending {
                if self.human[p.who] {
                    if matches!(p.ask, Ask::Priority) && !self.wants_stop(p.who) {
                        let who = p.who;
                        self.pending = None;
                        self.pass(who);
                        continue;
                    }
                    return;
                }
                let who = p.who;
                let a = crate::duel_ai::answer(self, who);
                if self.apply(who, a).is_err() {
                    // a computer answer the rules refuse: it passes or declines instead
                    let fallback = self.decline(who);
                    if self.apply(who, fallback).is_err() {
                        self.pending = None;
                    }
                }
                continue;
            }
            self.advance();
        }
        // a game that does not move ends as a withdrawal
        self.outcome = Some(Outcome::Withdrew);
        self.pending = None;
    }

    /// The answer that does nothing: pass, decline, the fewest targets.
    pub fn decline(&self, who: usize) -> Answer {
        let _ = who;
        match &self.pending {
            Some(Pending { ask, .. }) => match ask {
                Ask::Priority => Answer::Act(Action::Pass),
                Ask::Targets { legal, min, .. } => {
                    Answer::Targets(legal.iter().take(*min).copied().collect())
                }
                Ask::Attackers { .. } => Answer::Attack(Vec::new()),
                Ask::Blockers { .. } => Answer::Block(Vec::new()),
                Ask::Unleash { .. } => Answer::Yes(false),
                Ask::Discard { .. } => Answer::Pick(Some(0)),
                Ask::Sacrifice { .. } => Answer::Pick(Some(0)),
                Ask::Impostor { .. } => Answer::Pick(None),
                Ask::Pay { .. } => Answer::Yes(false),
                Ask::ChooseX { .. } => Answer::Pick(Some(0)),
                Ask::CostPick { .. } => Answer::Pick(Some(0)),
                Ask::May { .. } => Answer::Yes(false),
                Ask::Name { .. } => Answer::Pick(Some(0)),
                Ask::Piles { .. } => Answer::Pick(Some(0)),
                Ask::Offer { .. } => Answer::Pick(None),
            },
            None => Answer::Act(Action::Pass),
        }
    }

    /// Whether the player should be asked, rather than passed for: they
    /// have something to do, at a moment it can matter.
    pub fn wants_stop(&self, who: usize) -> bool {
        let acts = self.actions(who);
        if acts.len() <= 1 {
            return false;
        }
        if self.sorcery_time(who) {
            return true;
        }
        let fast = acts
            .iter()
            .any(|a| matches!(a, Action::Cast { .. } | Action::Activate { .. }));
        if !fast {
            return false;
        }
        if !self.stack.is_empty() {
            return self.stack.last().map(|i| i.ctrl != who).unwrap_or(false);
        }
        match self.step {
            Step::Combat | Step::Attackers | Step::Blockers => true,
            Step::End => self.active != who,
            _ => false,
        }
    }

    fn advance(&mut self) {
        if let Some(r) = self.resume.pop() {
            self.continue_resume(r);
            return;
        }
        if self.next_free_cast() {
            return;
        }
        if self.state_based() {
            return;
        }
        if let Some(i) = self.next_trigger() {
            let t = self.triggers.remove(i);
            self.put_trigger(t);
            return;
        }
        if self.step == Step::Cleanup {
            return;
        }
        self.pending = Some(Pending {
            who: self.priority,
            ask: Ask::Priority,
        });
    }

    /// The trigger to put on the stack next: the active player's first.
    fn next_trigger(&self) -> Option<usize> {
        if self.triggers.is_empty() {
            return None;
        }
        self.triggers
            .iter()
            .position(|t| t.ctrl == self.active)
            .or(Some(0))
    }

    fn pass(&mut self, who: usize) {
        self.passes += 1;
        if self.passes >= 2 {
            self.passes = 0;
            if self.stack.is_empty() {
                self.leave_step();
            } else {
                self.resolve_top();
                self.priority = self.active;
            }
        } else {
            self.priority = other(who);
        }
    }

    /// Takes an answer from `who` for the pending ask.
    pub fn answer(&mut self, who: usize, a: Answer) -> Result<(), String> {
        let r = self.apply(who, a);
        if r.is_ok() && self.human[who] {
            self.answers += 1;
        }
        self.run();
        r
    }

    fn apply(&mut self, who: usize, a: Answer) -> Result<(), String> {
        let Some(p) = self.pending.clone() else {
            return Err("Nothing is waiting on an answer.".to_string());
        };
        if p.who != who {
            return Err("It is not your decision.".to_string());
        }
        match (p.ask, a) {
            (Ask::Priority, Answer::Act(act)) => {
                if !self.actions(who).contains(&act) {
                    return Err("That can't be done now.".to_string());
                }
                self.pending = None;
                self.take_action(who, act);
            }
            (
                Ask::Targets {
                    spec,
                    legal,
                    min,
                    max,
                    then,
                    ..
                },
                Answer::Targets(ts),
            ) => {
                if ts.len() < min || ts.len() > max || ts.iter().any(|t| !legal.contains(t)) {
                    return Err("Those targets can't be chosen.".to_string());
                }
                // Common Bond may name the same creature twice; nothing else may
                if !matches!(spec, Tgt::Twice(_)) {
                    let mut seen = Vec::new();
                    for t in &ts {
                        if seen.contains(t) {
                            return Err("A target is chosen twice.".to_string());
                        }
                        seen.push(*t);
                    }
                }
                if let Tgt::Two(a, b) = spec {
                    let fits =
                        |t: &Target, f: Filter| self.legal_targets(who, &Tgt::One(f), None).contains(t);
                    if ts.len() == 2 && !(fits(&ts[0], a) && fits(&ts[1], b)) {
                        return Err("The first target and the second are of different kinds.".to_string());
                    }
                }
                self.pending = None;
                self.after_targets(who, *then, ts);
            }
            (Ask::Attackers { legal, .. }, Answer::Attack(ids)) => {
                let pairs: Vec<(u32, u32)> = Vec::new();
                self.check_attack(who, &legal, &ids, &pairs)?;
                self.pending = None;
                self.declare_attackers(ids);
            }
            (Ask::Attackers { legal, .. }, Answer::AttackAt(ids, pairs)) => {
                self.check_attack(who, &legal, &ids, &pairs)?;
                self.pending = None;
                for (a, w) in &pairs {
                    let w = *w;
                    self.set(*a, |p| p.aim = Some(w));
                }
                self.declare_attackers(ids);
            }
            (Ask::ChooseX { max, then }, Answer::Pick(Some(x))) => {
                if x as i32 > max {
                    return Err("That X can't be paid.".to_string());
                }
                self.pending = None;
                let then = match *then {
                    Then::Cast {
                        hand,
                        card,
                        mode,
                        overload,
                        pick,
                        free,
                        ..
                    } => Then::Cast {
                        hand,
                        card,
                        mode,
                        overload,
                        x: Some(x as i32),
                        pick,
                        free,
                    },
                    Then::Activate { perm, n, pick, .. } => Then::Activate {
                        perm,
                        n,
                        x: Some(x as i32),
                        pick,
                    },
                    other => other,
                };
                self.prepare(who, then);
            }
            (Ask::CostPick { legal, then }, Answer::Pick(Some(i))) => {
                let Some(&t) = legal.get(i) else {
                    return Err("That can't pay the cost.".to_string());
                };
                self.pending = None;
                let then = match *then {
                    Then::Cast {
                        hand,
                        card,
                        mode,
                        overload,
                        x,
                        free,
                        ..
                    } => Then::Cast {
                        hand,
                        card,
                        mode,
                        overload,
                        x,
                        pick: Some(t),
                        free,
                    },
                    Then::Activate { perm, n, x, .. } => Then::Activate {
                        perm,
                        n,
                        x,
                        pick: Some(t),
                    },
                    other => other,
                };
                self.prepare(who, then);
            }
            (Ask::May { what }, Answer::Yes(y)) => {
                self.pending = None;
                self.answer_may(who, what, y);
            }
            (Ask::Name { options, perm }, Answer::Pick(Some(i))) => {
                let Some(n) = options.get(i).cloned() else {
                    return Err("There is no such name.".to_string());
                };
                self.pending = None;
                self.named(who, perm, n);
            }
            (Ask::Piles { a, b }, Answer::Pick(Some(i))) => {
                self.pending = None;
                let (take, rest) = if i == 0 { (a, b) } else { (b, a) };
                self.take_piles(who, take, rest);
            }
            (Ask::Offer { legal, demon }, Answer::Pick(choice)) => {
                self.pending = None;
                if let Some(id) = choice.and_then(|i| legal.get(i).copied()) {
                    self.sacrifice(id);
                    self.demon_fed(demon);
                }
            }
            (Ask::Blockers { legal, .. }, Answer::Block(pairs)) => {
                let mut used = Vec::new();
                for (b, at) in &pairs {
                    let ok = legal.contains(b)
                        && !used.contains(b)
                        && self.perm(*b).map(|bp| self.can_block(bp, *at)).unwrap_or(false);
                    if !ok {
                        return Err("That block can't be made.".to_string());
                    }
                    used.push(*b);
                }
                self.pending = None;
                self.declare_blockers(pairs);
            }
            (Ask::Unleash { perm }, Answer::Yes(y)) => {
                self.pending = None;
                if y {
                    self.set(perm, |p| p.unleashed = true);
                    self.add_counters(perm, 1);
                }
                self.resume.push(Resume::Enter { perm });
                let name = self.name_of(perm);
                let key = if y { "log.enter_unleashed" } else { "log.enter" };
                self.log(
                    Ev::Enter,
                    who as u8,
                    key,
                    vec![("card", name), ("side", self.side_name(who))],
                    vec![perm],
                );
            }
            (Ask::Discard { n }, Answer::Pick(Some(i))) => {
                if i >= self.s[who].hand.len() {
                    return Err("There is no such card in hand.".to_string());
                }
                self.pending = None;
                let by = self.forcer;
                self.discard_by(who, i, by);
                if n > 1 && !self.s[who].hand.is_empty() {
                    self.pending = Some(Pending {
                        who,
                        ask: Ask::Discard { n: n - 1 },
                    });
                }
            }
            (Ask::Sacrifice { legal }, Answer::Pick(Some(i))) => {
                let Some(&id) = legal.get(i) else {
                    return Err("There is no such creature.".to_string());
                };
                self.pending = None;
                self.sacrifice(id);
            }
            (Ask::Impostor { legal, perm }, Answer::Pick(choice)) => {
                self.pending = None;
                match choice.and_then(|i| legal.get(i).copied()) {
                    Some(id) => self.bounce(id),
                    None => self.sacrifice(perm),
                }
            }
            (Ask::Pay { n, item }, Answer::Yes(y)) => {
                self.pending = None;
                let cost = Cost {
                    syms: vec![crate::mana::Sym::Generic(n)],
                };
                if y && self.pay(who, &cost, None) {
                    let name = self
                        .stack
                        .iter()
                        .find(|i| i.id == item)
                        .map(|i| i.name.clone())
                        .unwrap_or_default();
                    self.log(
                        Ev::Counter,
                        who as u8,
                        "log.paid",
                        vec![("side", self.side_name(who)), ("card", name)],
                        vec![],
                    );
                } else {
                    self.counter_item(item, "Izzet Charm");
                }
            }
            _ => return Err("That answer does not fit the question.".to_string()),
        }
        Ok(())
    }

    // ---------- the player's drafts ----------

    /// Toggles the `i`th listed choice in a draft: a target or an attacker.
    /// True when the draft is now full and can be sent at once.
    pub fn toggle(&mut self, who: usize, i: usize) -> Result<bool, String> {
        let Some(Pending { who: w, ask }) = &mut self.pending else {
            return Err("Nothing is waiting on an answer.".to_string());
        };
        if *w != who {
            return Err("It is not your decision.".to_string());
        }
        match ask {
            Ask::Targets {
                spec,
                legal,
                chosen,
                max,
                ..
            } => {
                let t = *legal.get(i).ok_or("There is no such target.")?;
                let twice = matches!(spec, Tgt::Twice(_));
                if let Some(k) = chosen
                    .iter()
                    .position(|c| *c == t)
                    .filter(|_| !twice || chosen.len() >= *max)
                {
                    chosen.remove(k);
                } else if chosen.len() < *max {
                    chosen.push(t);
                }
                Ok(chosen.len() == *max && *max == 1)
            }
            Ask::Attackers { legal, chosen } => {
                let id = *legal.get(i).ok_or("There is no such creature.")?;
                // each click: attack the player, then each of the opponent's planeswalkers, then not at all
                let walkers: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|p| {
                        p.ctrl != who
                            && p.card
                                .map(|c| self.cards[c].is(ty::PLANESWALKER))
                                .unwrap_or(false)
                    })
                    .map(|p| p.id)
                    .collect();
                if let Some(k) = chosen.iter().position(|c| *c == id) {
                    let now = self.draft_aims.iter().position(|(a, _)| *a == id);
                    let next = match now {
                        None => walkers.first().copied(),
                        Some(j) => {
                            let w = self.draft_aims[j].1;
                            self.draft_aims.remove(j);
                            walkers
                                .iter()
                                .position(|&x| x == w)
                                .and_then(|q| walkers.get(q + 1))
                                .copied()
                        }
                    };
                    match next {
                        Some(w) => self.draft_aims.push((id, w)),
                        None if now.is_some() || walkers.is_empty() => {
                            chosen.remove(k);
                        }
                        None => {}
                    }
                } else {
                    chosen.push(id);
                }
                Ok(false)
            }
            _ => Err("There is nothing to choose from a list now.".to_string()),
        }
    }

    /// Attack with everything that can, or nothing if all are chosen.
    pub fn toggle_all(&mut self, who: usize) -> Result<(), String> {
        match &mut self.pending {
            Some(Pending {
                who: w,
                ask: Ask::Attackers { legal, chosen },
            }) if *w == who => {
                if chosen.len() == legal.len() {
                    chosen.clear();
                } else {
                    *chosen = legal.clone();
                }
                Ok(())
            }
            _ => Err("No attack is being declared.".to_string()),
        }
    }

    /// Moves a blocker to the next attacker it can block, then to none.
    pub fn cycle_block(&mut self, who: usize, blocker: u32) -> Result<(), String> {
        let can: Vec<u32> = match &self.pending {
            Some(Pending {
                who: w,
                ask: Ask::Blockers { legal, attackers, .. },
            }) if *w == who && legal.contains(&blocker) => {
                let Some(b) = self.perm(blocker) else {
                    return Err("There is no such creature.".to_string());
                };
                attackers
                    .iter()
                    .copied()
                    .filter(|&a| self.can_block(b, a))
                    .collect()
            }
            _ => return Err("That creature can't block now.".to_string()),
        };
        if let Some(Pending {
            ask: Ask::Blockers { chosen, .. },
            ..
        }) = &mut self.pending
        {
            let now = chosen.iter().position(|(b, _)| *b == blocker);
            let next = match now {
                None => can.first().copied(),
                Some(k) => {
                    let at = chosen[k].1;
                    chosen.remove(k);
                    can.iter()
                        .position(|&a| a == at)
                        .and_then(|j| can.get(j + 1))
                        .copied()
                }
            };
            if let Some(a) = next {
                chosen.push((blocker, a));
            }
        }
        Ok(())
    }

    /// A click on a permanent: a target, an attacker or a blocker.
    pub fn click(&mut self, who: usize, id: u32) -> Result<bool, String> {
        let i = match &self.pending {
            Some(Pending {
                ask: Ask::Targets { legal, .. },
                ..
            }) => legal.iter().position(|t| *t == Target::Perm(id)),
            Some(Pending {
                ask: Ask::Attackers { legal, .. },
                ..
            }) => legal.iter().position(|t| *t == id),
            Some(Pending {
                ask: Ask::Blockers { .. },
                ..
            }) => {
                self.cycle_block(who, id)?;
                return Ok(false);
            }
            _ => None,
        };
        match i {
            Some(i) => self.toggle(who, i),
            None => Ok(false),
        }
    }

    /// Sends the draft: the targets, attackers or blocks chosen; or passes.
    pub fn confirm(&mut self, who: usize) -> Result<(), String> {
        let a = match &self.pending {
            Some(Pending { ask, .. }) => match ask {
                Ask::Priority => Answer::Act(Action::Pass),
                Ask::Targets { chosen, .. } => Answer::Targets(chosen.clone()),
                Ask::Attackers { chosen, .. } => {
                    let aims: Vec<(u32, u32)> = self
                        .draft_aims
                        .iter()
                        .copied()
                        .filter(|(a, _)| chosen.contains(a))
                        .collect();
                    self.draft_aims.clear();
                    if aims.is_empty() {
                        Answer::Attack(chosen.clone())
                    } else {
                        Answer::AttackAt(chosen.clone(), aims)
                    }
                }
                Ask::Blockers { chosen, .. } => Answer::Block(chosen.clone()),
                _ => return Err("Choose one of the answers.".to_string()),
            },
            None => return Err("Nothing is waiting on an answer.".to_string()),
        };
        self.answer(who, a)
    }

    /// Takes back a spell or ability whose targets are being chosen.
    pub fn cancel(&mut self, who: usize) -> Result<(), String> {
        match &self.pending {
            Some(Pending {
                who: w,
                ask: Ask::Targets { then, .. },
            }) if *w == who && !matches!(**then, Then::Trigger(_)) => {
                self.pending = Some(Pending {
                    who,
                    ask: Ask::Priority,
                });
                Ok(())
            }
            _ => Err("This can't be taken back.".to_string()),
        }
    }

    fn take_action(&mut self, who: usize, act: Action) {
        match act {
            Action::Pass => self.pass(who),
            Action::Land(i) => {
                self.play_land(who, i);
                self.passes = 0;
            }
            Action::Cast { hand, mode, overload } => {
                let card = self.s[who].hand[hand];
                self.prepare(
                    who,
                    Then::Cast {
                        hand,
                        card,
                        mode,
                        overload,
                        x: None,
                        pick: None,
                        free: false,
                    },
                );
            }
            Action::Activate { perm, n } => self.prepare(
                who,
                Then::Activate {
                    perm,
                    n,
                    x: None,
                    pick: None,
                },
            ),
            Action::GraveAbility { grave, n } => self.grave_ability(who, grave, n),
            Action::Scavenge { grave } => {
                let card = self.s[who].grave[grave];
                self.ask_targets(
                    who,
                    Some(Tgt::One(crate::rules::creatures(Who::Any))),
                    None,
                    Then::Scavenge { grave, card },
                );
            }
        }
    }

    /// Readies a spell or an ability: X, then a cost beyond mana, then targets.
    fn prepare(&mut self, who: usize, then: Then) {
        match then {
            Then::Cast {
                card,
                overload,
                mode,
                x,
                pick,
                free,
                ..
            } => {
                let cost = self.cast_cost(who, card, overload);
                if cost.xs() > 0 && x.is_none() && !free {
                    let max = self.max_x(who, &cost);
                    self.pending = Some(Pending {
                        who,
                        ask: Ask::ChooseX {
                            max,
                            then: Box::new(then),
                        },
                    });
                    return;
                }
                if self.rules(card).sac_to_cast && pick.is_none() {
                    let legal = self.extra_options(who, None, Extra::SacAnother);
                    if legal.is_empty() {
                        return;
                    }
                    self.pending = Some(Pending {
                        who,
                        ask: Ask::CostPick {
                            legal,
                            then: Box::new(then),
                        },
                    });
                    return;
                }
                let spec = if overload {
                    None
                } else {
                    self.rules(card).modes.get(mode).and_then(|m| m.target)
                };
                self.ask_targets(who, spec, None, then);
            }
            Then::Activate { perm, n, pick, .. } => {
                let Some(c) = self.perm(perm).and_then(|p| p.card) else {
                    return;
                };
                let a = self.rules(c).activated[n].clone();
                let needs_pick = matches!(
                    a.extra,
                    Extra::SacAnother
                        | Extra::SacNontoken
                        | Extra::Discard
                        | Extra::DiscardCreature
                        | Extra::DiscardPair
                );
                if needs_pick && pick.is_none() {
                    let legal = self.extra_options(who, Some(perm), a.extra);
                    if legal.is_empty() {
                        return;
                    }
                    self.pending = Some(Pending {
                        who,
                        ask: Ask::CostPick {
                            legal,
                            then: Box::new(then),
                        },
                    });
                    return;
                }
                self.ask_targets(who, a.ability.target, Some(perm), then);
            }
            other => self.after_targets(who, other, Vec::new()),
        }
    }

    fn ask_targets(&mut self, who: usize, spec: Option<Tgt>, source: Option<u32>, then: Then) {
        let Some(spec) = spec else {
            self.after_targets(who, then, Vec::new());
            return;
        };
        let legal = self.legal_targets(who, &spec, source);
        let (min, max) = Duel::spec_bounds(&spec);
        let max = if matches!(spec, Tgt::Twice(_)) {
            max
        } else {
            max.min(legal.len())
        };
        if legal.len() < min.min(if matches!(spec, Tgt::Twice(_)) { 1 } else { min })
            || legal.is_empty() && min > 0
        {
            // nothing to target: a trigger is removed; a spell or ability is not begun
            return;
        }
        // a land target that cannot matter is picked for the player (Street Sweeper)
        if matches!(spec, Tgt::One(f) if f.land && !matches!(then, Then::Cast { .. })) {
            let t = vec![legal[0]];
            self.after_targets(who, then, t);
            return;
        }
        self.pending = Some(Pending {
            who,
            ask: Ask::Targets {
                spec,
                legal,
                min,
                max,
                chosen: Vec::new(),
                then: Box::new(then),
            },
        });
    }

    fn after_targets(&mut self, who: usize, then: Then, targets: Vec<Target>) {
        match then {
            Then::Cast {
                hand,
                card,
                mode,
                overload,
                x,
                pick,
                free,
            } => self.cast(
                who,
                hand,
                card,
                mode,
                overload,
                x.unwrap_or(0),
                pick,
                free,
                targets,
            ),
            Then::Activate { perm, n, x, pick } => self.activate(who, perm, n, x, pick, targets),
            Then::Scavenge { grave, card } => {
                let Some(cost) = self.scavenge_cost(who, card) else {
                    return;
                };
                if self.s[who].grave.get(grave) != Some(&card) || !self.pay(who, &cost, None) {
                    return;
                }
                self.s[who].grave.remove(grave);
                self.s[who].exile.push(card);
                let id = self.id();
                let name = self.card(card).name.clone();
                self.stack.push(Item {
                    id,
                    ctrl: who,
                    src: Source::Ability {
                        perm: 0,
                        card: Some(card),
                    },
                    ability: Ability {
                        target: Some(Tgt::One(crate::rules::creatures(Who::Any))),
                        effects: vec![Eff::Counters(self.card(card).power)],
                    },
                    targets,
                    each: false,
                    x: self.card(card).power,
                    name: name.clone(),
                    tctrl: Vec::new(),
                });
                self.log(
                    Ev::Ability,
                    who as u8,
                    "log.scavenge_cast",
                    vec![("side", self.side_name(who)), ("card", name)],
                    vec![],
                );
                self.passes = 0;
                self.priority = who;
            }
            Then::Trigger(t) => {
                let id = self.id();
                let targets = if targets.is_empty() {
                    t.fixed.clone()
                } else {
                    targets
                };
                self.stack.push(Item {
                    id,
                    ctrl: t.ctrl,
                    src: Source::Ability {
                        perm: t.perm,
                        card: t.card,
                    },
                    ability: t.ability,
                    targets,
                    each: false,
                    x: t.x,
                    name: t.name,
                    tctrl: Vec::new(),
                });
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn cast(
        &mut self,
        who: usize,
        hand: usize,
        card: usize,
        mode: usize,
        overload: bool,
        x: i32,
        pick: Option<Target>,
        free: bool,
        targets: Vec<Target>,
    ) {
        if !free && self.s[who].hand.get(hand) != Some(&card) {
            return;
        }
        if !free {
            let cost = self.cast_cost(who, card, overload).with_x(x);
            if !self.pay(who, &cost, None) {
                return;
            }
        }
        if let Some(Target::Perm(id)) = pick {
            self.sacrifice(id);
        }
        if !free {
            self.s[who].hand.remove(hand);
        }
        let c = self.card(card).clone();
        let ability = if c.rules.modes.is_empty() {
            Ability {
                target: None,
                effects: Vec::new(),
            }
        } else {
            c.rules.modes[mode.min(c.rules.modes.len() - 1)].clone()
        };
        let id = self.id();
        self.stack.push(Item {
            id,
            ctrl: who,
            src: Source::Spell { card, mode, overload },
            ability,
            targets: targets.clone(),
            each: overload,
            x,
            name: c.name.clone(),
            tctrl: Vec::new(),
        });
        let focus: Vec<u32> = targets
            .iter()
            .filter_map(|t| match t {
                Target::Perm(p) => Some(*p),
                _ => None,
            })
            .collect();
        let mut args = vec![("side", self.side_name(who)), ("card", c.name.clone())];
        if let Some(t) = targets.first() {
            args.push(("target", self.target_name(t)));
        }
        let key = if free {
            "log.cast_free"
        } else if overload {
            "log.cast_overload"
        } else if !targets.is_empty() {
            "log.cast_at"
        } else {
            "log.cast"
        };
        self.log(Ev::Cast, who as u8, key, args, focus);
        if c.is(ty::CREATURE) {
            self.s[who].creatures_cast += 1;
        }
        self.cast_triggers(who, &c);
        self.played_name(who, &c.name);
        self.passes = 0;
        self.priority = who;
    }

    pub fn target_name(&self, t: &Target) -> String {
        match t {
            Target::Perm(id) => self.name_of(*id),
            Target::Player(p) => self.side_name(*p),
            Target::Item(i) => self
                .stack
                .iter()
                .find(|x| x.id == *i)
                .map(|x| x.name.clone())
                .unwrap_or_default(),
            Target::Grave(_, c) => self.card(*c).name.clone(),
            Target::Hand(_) => String::new(),
        }
    }

    fn cast_triggers(&mut self, who: usize, c: &Card) {
        let multi = c.multicoloured();
        let spell = c.is_spell();
        let mut trigs = Vec::new();
        let ab = |target: Option<Tgt>, effects: Vec<Eff>| Ability { target, effects };
        for p in self.perms.iter().filter(|p| p.ctrl == who) {
            let Some(pc) = p.card else { continue };
            match self.rules(pc).on_cast {
                Some(OnCast::SpellBurn) if spell => {
                    trigs.push(self.trig_of(p, ab(None, vec![Eff::DamageEachOpponent(2)]), 0))
                }
                Some(OnCast::MulticolourBurn) if multi => trigs.push(self.trig_of(
                    p,
                    ab(
                        Some(Tgt::Any(crate::rules::creatures(Who::Any))),
                        vec![Eff::Damage(2)],
                    ),
                    0,
                )),
                Some(OnCast::MulticolourUntap) if multi => {
                    let mut t = self.trig_of(p, ab(None, vec![Eff::Untap]), 0);
                    t.x = p.id as i32;
                    trigs.push(t)
                }
                Some(OnCast::SpellGrow) if spell => {
                    let mut t = self.trig_of(p, ab(None, vec![Eff::PumpSelf(1, 1), Eff::Untap]), 0);
                    t.x = p.id as i32;
                    trigs.push(t)
                }
                Some(OnCast::Tablet) => {
                    let n = mana::count_colours(c.colours & p.chosen_colours) as i32;
                    if n > 0 {
                        trigs.push(self.trig_of(p, ab(None, vec![Eff::Gain(n)]), 0));
                    }
                }
                _ => {}
            }
        }
        if spell && self.s[who].perks & perk::SPELL_PING != 0 {
            trigs.push(Trig {
                ctrl: who,
                perm: 0,
                card: None,
                ability: ab(None, vec![Eff::DamageEachOpponent(1)]),
                x: 0,
                name: self.side_name(who),
                fixed: Vec::new(),
            });
        }
        self.triggers.extend(trigs);
    }

    fn activate(
        &mut self,
        who: usize,
        perm: u32,
        n: usize,
        x: Option<i32>,
        pick: Option<Target>,
        targets: Vec<Target>,
    ) {
        let Some(p) = self.perm(perm).cloned() else { return };
        let Some(c) = p.card else { return };
        let a = self.rules(c).activated[n].clone();
        let except = if a.tap { Some(perm) } else { None };
        if !self.pay(who, &a.cost, except) {
            return;
        }
        if a.tap {
            self.set(perm, |q| q.tapped = true);
            self.tapped_for_mana(perm);
        }
        if let Some(l) = a.loyalty {
            self.set(perm, |q| {
                q.loyalty += l;
                q.used = true;
            });
        } else if a.once {
            self.set(perm, |q| q.used = true);
        }
        let name = self.perm_name(&p);
        // what the ability counts: the source's power, or what paid its cost
        let mut x = x.unwrap_or(self.power(&p));
        x = self.pay_extra(who, &p, a.extra, pick).unwrap_or(x);
        if a.sac {
            self.sacrifice(perm);
        }
        let id = self.id();
        self.stack.push(Item {
            id,
            ctrl: who,
            src: Source::Ability { perm, card: Some(c) },
            ability: a.ability.clone(),
            targets: targets.clone(),
            each: false,
            x,
            name: name.clone(),
            tctrl: Vec::new(),
        });
        let mut focus = vec![perm];
        for t in &targets {
            if let Target::Perm(x) = t {
                focus.push(*x);
            }
        }
        let key = match a.loyalty {
            Some(l) if l >= 0 => "log.loyalty_up",
            Some(_) => "log.loyalty_down",
            None => "log.ability",
        };
        let change = a
            .loyalty
            .map(|l| if l >= 0 { format!("+{l}") } else { l.to_string() })
            .unwrap_or_default();
        self.log(
            Ev::Ability,
            who as u8,
            key,
            vec![("card", name), ("n", change)],
            focus,
        );
        self.passes = 0;
        self.priority = who;
    }

    fn put_trigger(&mut self, t: Trig) {
        let spec = t.ability.target;
        let who = t.ctrl;
        let source = if t.perm > 0 { Some(t.perm) } else { None };
        if !t.fixed.is_empty() {
            let fixed = t.fixed.clone();
            self.after_targets(who, Then::Trigger(t), fixed);
            return;
        }
        match spec {
            Some(s) => {
                if self.legal_targets(who, &s, source).is_empty() {
                    return;
                }
                self.ask_targets(who, Some(s), source, Then::Trigger(t));
            }
            None => self.after_targets(who, Then::Trigger(t), Vec::new()),
        }
    }

    // ---------- resolving ----------

    fn resolve_top(&mut self) {
        let Some(mut item) = self.stack.pop() else { return };
        // targets that are no longer legal are not affected; with none left, it does nothing
        if let Some(spec) = item.ability.target {
            if !item.each && !item.targets.is_empty() {
                let source = match item.src {
                    Source::Ability { perm, .. } if perm > 0 => Some(perm),
                    _ => None,
                };
                let ctrl = item.ctrl;
                let legal: Vec<Target> = item
                    .targets
                    .iter()
                    .copied()
                    .filter(|t| self.still_legal(ctrl, &spec, t, source))
                    .collect();
                if legal.is_empty() {
                    self.log(
                        Ev::Resolve,
                        item.ctrl as u8,
                        "log.fizzle",
                        vec![("card", item.name.clone())],
                        vec![],
                    );
                    self.finish_spell(&item);
                    return;
                }
                item.targets = legal;
            }
        }
        if item.each {
            // Counterflux overloaded: each spell you don't control
            if let Some(Tgt::Spell(kind)) = item.ability.target {
                let ctrl = item.ctrl;
                item.targets = self
                    .stack
                    .iter()
                    .filter(|i| self.spell_fits(ctrl, kind, i))
                    .map(|i| Target::Item(i.id))
                    .collect();
            }
            let filter = match item.ability.target {
                Some(Tgt::One(f)) | Some(Tgt::UpTo(_, f)) | Some(Tgt::Any(f)) => Some(f),
                _ => None,
            };
            if let Some(f) = filter {
                item.targets = self
                    .each_of(&f, item.ctrl)
                    .into_iter()
                    .map(Target::Perm)
                    .collect();
            }
        }
        item.tctrl = item
            .targets
            .iter()
            .map(|t| match t {
                Target::Perm(id) => self.perm(*id).map(|p| p.ctrl).unwrap_or(item.ctrl),
                Target::Player(p) => *p,
                Target::Item(i) => self
                    .stack
                    .iter()
                    .find(|x| x.id == *i)
                    .map(|x| x.ctrl)
                    .unwrap_or(item.ctrl),
                Target::Grave(o, _) => *o,
                Target::Hand(_) => item.ctrl,
            })
            .collect();
        match item.src {
            Source::Spell { card, .. } if !self.card(card).is_spell() => {
                // a permanent spell enters the battlefield
                let ctrl = item.ctrl;
                let id = self.make_perm(ctrl, card);
                if self.rules(card).x_counters {
                    let x = item.x;
                    self.set(id, |p| p.charge = x);
                }
                if self.rules(card).aura.is_some() {
                    match item.targets.first() {
                        Some(Target::Perm(host)) if self.perm(*host).is_some() => {
                            let host = *host;
                            self.set(id, |p| p.attached = Some(host));
                        }
                        _ => {
                            self.perms.retain(|p| p.id != id);
                            self.put_in_grave(ctrl, card);
                            return;
                        }
                    }
                }
                self.entering(ctrl, id);
            }
            _ => {
                if let Source::Spell { .. } = item.src {
                    self.log(
                        Ev::Resolve,
                        item.ctrl as u8,
                        "log.resolve",
                        vec![("card", item.name.clone())],
                        vec![],
                    );
                }
                self.run_effects(Box::new(item), 0);
            }
        }
    }

    /// A permanent has entered: unleash, then its enter triggers.
    fn entering(&mut self, ctrl: usize, id: u32) {
        let card = self.perm(id).and_then(|p| p.card);
        let Some(c) = card else { return };
        self.as_it_enters(ctrl, id, c);
        if self.card(c).is(ty::CREATURE)
            && self.s[ctrl].perks & perk::HASTE_FIRST != 0
            && self.s[ctrl].creatures_cast == 1
        {
            self.set(id, |p| p.perk_kw |= kw::HASTE);
            self.s[ctrl].creatures_cast += 1;
        }
        if self.rules(c).unleash {
            self.pending = Some(Pending {
                who: ctrl,
                ask: Ask::Unleash { perm: id },
            });
            return;
        }
        let name = self.card(c).name.clone();
        self.log(
            Ev::Enter,
            ctrl as u8,
            "log.enter",
            vec![("card", name), ("side", self.side_name(ctrl))],
            vec![id],
        );
        self.enter_triggers(id);
    }

    fn enter_triggers(&mut self, id: u32) {
        let Some(p) = self.perm(id).cloned() else { return };
        let Some(c) = p.card else { return };
        if let Some(ab) = self.rules(c).etb.clone() {
            let t = self.trig_of(&p, ab, 0);
            self.triggers.push(t);
        }
        self.ally_entered(&p);
    }

    fn finish_spell(&mut self, item: &Item) {
        // a spell's card goes to the graveyard; a permanent spell that could not enter does too
        if let Source::Spell { card, .. } = item.src {
            if self.rules(card).exile_after && self.card(card).is_spell() {
                self.s[item.ctrl].exile.push(card);
            } else {
                self.put_in_grave(item.ctrl, card);
            }
        }
    }

    fn continue_resume(&mut self, r: Resume) {
        match r {
            Resume::Effects { item, next } => self.run_effects(item, next),
            Resume::Enter { perm } => self.enter_triggers(perm),
            Resume::Cleanup => self.cleanup(),
            Resume::Draw { who } => self.draw(who, 1),
        }
    }

    fn run_effects(&mut self, item: Box<Item>, from: usize) {
        let effects = item.ability.effects.clone();
        for (i, e) in effects.iter().enumerate().skip(from) {
            let waits = self.effect(&item, e);
            if self.outcome.is_some() {
                return;
            }
            if waits {
                self.resume.push(Resume::Effects { item, next: i + 1 });
                return;
            }
        }
        // the special upkeep losses
        if item.ability.effects.is_empty() && item.x > 0 {
            let a = self.active;
            let card = match item.src {
                Source::Ability { card, .. } => card,
                _ => None,
            };
            if let Some(c) = card {
                if let Some((n, _)) = self.rules(c).opponent_upkeep {
                    // the intervening "if" is checked again as it resolves
                    if self.s[a].hand.len() <= n {
                        self.lose_life(a, item.x, "log.shrieking");
                    }
                } else if self.rules(c).aura == Some(Aura::StabWound) {
                    self.lose_life(a, item.x, "log.stab");
                }
            }
        }
        self.finish_spell(&item);
    }

    fn lose_life(&mut self, side: usize, n: i32, key: &'static str) {
        self.s[side].life -= n;
        self.log(
            Ev::Life,
            side as u8,
            key,
            vec![("side", self.side_name(side)), ("n", n.to_string())],
            vec![],
        );
    }

    fn source_perm(item: &Item) -> Option<u32> {
        match item.src {
            Source::Ability { perm, .. } if perm > 0 => Some(perm),
            _ => None,
        }
    }

    /// Applies one effect. True when it waits on an answer.
    fn effect(&mut self, item: &Item, e: &Eff) -> bool {
        let ctrl = item.ctrl;
        let src = Duel::source_perm(item);
        let perm_targets: Vec<u32> = item
            .targets
            .iter()
            .filter_map(|t| match t {
                Target::Perm(id) => Some(*id),
                _ => None,
            })
            .filter(|id| self.perm(*id).is_some())
            .collect();
        match e {
            Eff::Damage(n) => {
                for t in item.targets.clone() {
                    match t {
                        Target::Perm(id) => self.damage_perm(ctrl, src, &item.name, id, *n),
                        Target::Player(pl) => self.damage_player(src, &item.name, ctrl, pl, *n),
                        _ => {}
                    }
                }
            }
            Eff::Destroy => {
                for id in perm_targets {
                    self.destroy(id);
                }
            }
            Eff::Exile => {
                for id in perm_targets {
                    self.exile(id);
                }
            }
            Eff::Bounce => {
                for id in perm_targets {
                    self.bounce(id);
                }
            }
            Eff::Detain => {
                for id in perm_targets {
                    self.detain(id, ctrl);
                }
            }
            Eff::Tap => {
                for id in perm_targets {
                    self.set(id, |p| p.tapped = true);
                    let n = self.name_of(id);
                    self.log(Ev::Ability, ctrl as u8, "log.tap", vec![("card", n)], vec![id]);
                }
            }
            Eff::Untap => {
                let ids = if item.ability.target.is_none() && item.x > 0 {
                    vec![item.x as u32]
                } else {
                    perm_targets
                };
                for id in ids {
                    self.set(id, |p| p.tapped = false);
                }
            }
            Eff::Pump(p, t, k) => {
                for id in perm_targets {
                    self.pump(id, *p, *t, *k);
                }
            }
            Eff::PumpSelf(p, t) => {
                if let Some(id) = src {
                    self.pump(id, *p, *t, 0);
                }
            }
            Eff::Counters(n) => {
                // with no target, the source (Gobbling Ooze, Lotleth Troll, Nivmagus Elemental)
                let ids = if perm_targets.is_empty() && item.ability.target.is_none() {
                    src.into_iter().collect()
                } else {
                    perm_targets
                };
                for id in ids {
                    self.add_counters(id, *n);
                }
            }
            Eff::CountersFromDied => {
                let n = item.x;
                for id in perm_targets {
                    self.add_counters(id, n);
                }
            }
            Eff::Chorus => {
                let n = self
                    .perms
                    .iter()
                    .filter(|p| p.ctrl == ctrl && self.is_creature(p))
                    .count() as i32;
                for id in perm_targets {
                    self.pump(id, n, n, kw::TRAMPLE);
                }
            }
            Eff::ExileIfDies => {
                for id in perm_targets {
                    self.set(id, |p| p.exile_if_dies = true);
                }
            }
            Eff::Token(t, n) => self.make_token(ctrl, *t, *n),
            Eff::Populate => self.populate(ctrl),
            Eff::Draw(n) => self.draw(ctrl, *n as usize),
            Eff::TargetDraws(n) => {
                for t in &item.targets {
                    if let Target::Player(pl) = t {
                        self.draw(*pl, *n as usize);
                    }
                }
            }
            Eff::Gain(n) => self.gain(ctrl, *n),
            Eff::DamageEachOpponent(n) => self.damage_player(src, &item.name, ctrl, other(ctrl), *n),
            Eff::TargetDiscards(n) => {
                for t in &item.targets {
                    if let Target::Player(pl) = t {
                        return self.discard_or_ask(*pl, *n as usize, ctrl);
                    }
                }
            }
            Eff::ItsControllerDiscards(n) => {
                if let Some(&pl) = item.tctrl.first() {
                    return self.discard_or_ask(pl, *n as usize, ctrl);
                }
            }
            Eff::Counter => {
                for t in &item.targets {
                    if let Target::Item(i) = t {
                        self.counter_item(*i, &item.name);
                    }
                }
            }
            Eff::CounterUnlessPays(n) => {
                for t in &item.targets {
                    if let Target::Item(i) = t {
                        let Some(victim) = self.stack.iter().find(|x| x.id == *i).map(|x| x.ctrl) else {
                            continue;
                        };
                        let uncounterable = self.stack.iter().any(|x| x.id == *i && matches!(x.src, Source::Spell { card, .. } if self.rules(card).kw & kw::CANT_BE_COUNTERED != 0));
                        if uncounterable {
                            continue;
                        }
                        let cost = Cost {
                            syms: vec![crate::mana::Sym::Generic(*n)],
                        };
                        if self.can_pay(victim, &cost, None) {
                            self.pending = Some(Pending {
                                who: victim,
                                ask: Ask::Pay { n: *n, item: *i },
                            });
                            return true;
                        }
                        self.counter_item(*i, &item.name);
                    }
                }
            }
            Eff::Loot(d, k) => {
                self.draw(ctrl, *d as usize);
                return self.discard(ctrl, *k as usize);
            }
            Eff::EachSacrifices => {
                for pl in [ctrl, other(ctrl)] {
                    let legal: Vec<u32> = self
                        .perms
                        .iter()
                        .filter(|p| p.ctrl == pl && self.is_creature(p))
                        .map(|p| p.id)
                        .collect();
                    if legal.is_empty() {
                        continue;
                    }
                    if self.human[pl] {
                        // the computer's side has already chosen; the player chooses last
                        if pl == ctrl {
                            let o = other(ctrl);
                            self.computer_sacrifice(o);
                        }
                        self.pending = Some(Pending {
                            who: pl,
                            ask: Ask::Sacrifice { legal },
                        });
                        return true;
                    }
                    self.computer_sacrifice(pl);
                }
            }
            Eff::ImpostorReturn => {
                let Some(me) = src.or_else(|| {
                    self.perms
                        .iter()
                        .rev()
                        .find(|p| {
                            p.card
                                .map(|c| self.card(c).name == "Faerie Impostor")
                                .unwrap_or(false)
                                && p.ctrl == ctrl
                        })
                        .map(|p| p.id)
                }) else {
                    return false;
                };
                if self.perm(me).is_none() {
                    return false;
                }
                let legal: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|p| p.ctrl == ctrl && p.id != me && self.is_creature(p))
                    .map(|p| p.id)
                    .collect();
                if legal.is_empty() {
                    self.sacrifice(me);
                    return false;
                }
                self.pending = Some(Pending {
                    who: ctrl,
                    ask: Ask::Impostor { legal, perm: me },
                });
                return true;
            }
            Eff::ExileHeld | Eff::ExileHeldSameName => {
                let holder = src.filter(|id| self.perm(*id).is_some());
                let mut victims: Vec<u32> = perm_targets.clone();
                if *e == Eff::ExileHeldSameName {
                    for id in &perm_targets {
                        let n = self.name_of(*id);
                        for p in &self.perms {
                            if p.id != *id && self.perm_name(p) == n && !victims.contains(&p.id) {
                                victims.push(p.id);
                            }
                        }
                    }
                }
                for id in victims {
                    let Some(p) = self.perm(id).cloned() else { continue };
                    let name = self.perm_name(&p);
                    self.leave(id);
                    if let Some(c) = p.card {
                        match holder {
                            Some(h) => self.set(h, |hp| hp.held.push((p.owner, c))),
                            None => self.s[p.owner].exile.push(c),
                        }
                    }
                    self.log(Ev::Dies, ctrl as u8, "log.exile", vec![("card", name)], vec![id]);
                }
                for t in &item.targets {
                    if let Target::Grave(o, c) = t {
                        if let Some(i) = self.s[*o].grave.iter().position(|g| g == c) {
                            self.s[*o].grave.remove(i);
                            match holder {
                                Some(h) => self.set(h, |hp| hp.held.push((*o, *c))),
                                None => self.s[*o].exile.push(*c),
                            }
                            let name = self.card(*c).name.clone();
                            self.log(Ev::Dies, ctrl as u8, "log.exile", vec![("card", name)], vec![]);
                        }
                    }
                }
            }
            Eff::ToLibraryTop => {
                if let Source::Ability { card: Some(c), .. } = item.src {
                    let owner = ctrl;
                    if let Some(i) = self.s[owner].grave.iter().rposition(|g| *g == c) {
                        self.s[owner].grave.remove(i);
                        self.s[owner].library.push(c);
                        let name = self.card(c).name.clone();
                        self.log(
                            Ev::Ability,
                            owner as u8,
                            "log.library_top",
                            vec![("card", name)],
                            vec![],
                        );
                    }
                }
            }
            Eff::Regenerate => {
                if let Some(id) = src {
                    self.set(id, |p| p.regen += 1);
                }
            }
            Eff::Animate => {
                if let Some(id) = src {
                    self.set(id, |p| p.animated = true);
                    let name = self.name_of(id);
                    self.log(
                        Ev::Ability,
                        ctrl as u8,
                        "log.animate",
                        vec![("card", name)],
                        vec![id],
                    );
                }
            }
            Eff::Attach => {
                if let (Some(id), Some(&host)) = (src, perm_targets.first()) {
                    if self.perm(id).is_some() {
                        self.set(id, |p| p.attached = Some(host));
                        let name = self.name_of(host);
                        self.log(
                            Ev::Ability,
                            ctrl as u8,
                            "log.equip",
                            vec![("card", name)],
                            vec![host],
                        );
                    }
                }
            }
            Eff::DestroyAurasOn => {
                for id in perm_targets {
                    let auras: Vec<u32> = self
                        .attachments(id)
                        .filter(|a| self.aura_of(a).is_some())
                        .map(|a| a.id)
                        .collect();
                    for a in auras {
                        self.destroy(a);
                    }
                }
            }
            other => return self.effect_more(item, other),
        }
        false
    }

    fn pump(&mut self, id: u32, p: i32, t: i32, k: u32) {
        self.set(id, |x| x.mods.push((p, t, k)));
        let name = self.name_of(id);
        self.log(
            Ev::Ability,
            0,
            "log.pump",
            vec![("card", name), ("p", signed(p)), ("t", signed(t))],
            vec![id],
        );
    }

    fn detain(&mut self, id: u32, by: usize) {
        self.set(id, |p| p.detained_by = Some(by));
        let name = self.name_of(id);
        self.log(Ev::Detain, by as u8, "log.detain", vec![("card", name)], vec![id]);
    }

    fn gain(&mut self, side: usize, n: i32) {
        if n <= 0 || self.any_static(Static::NoLifeGain) {
            return;
        }
        self.s[side].life += n;
        self.log(
            Ev::Life,
            side as u8,
            "log.gain",
            vec![("side", self.side_name(side)), ("n", n.to_string())],
            vec![],
        );
    }

    fn populate(&mut self, side: usize) {
        let best = self
            .perms
            .iter()
            .filter(|p| p.ctrl == side && p.token.is_some() && self.is_creature(p))
            .max_by_key(|p| (self.base_pt(p).0 + self.base_pt(p).1, p.id))
            .cloned();
        match best {
            Some(b) if matches!(b.token, Some(Token::Copy) | Some(Token::Ooze)) => {
                let p = self.blank(side, b.card, b.token);
                let id = p.id;
                self.perms.push(p);
                self.set(id, |q| q.size = b.size);
                let name = self.perm_name(&b);
                self.log(
                    Ev::Token,
                    side as u8,
                    "log.token",
                    vec![("side", self.side_name(side)), ("token", name)],
                    vec![id],
                );
            }
            Some(b) => {
                if let Some(t) = b.token {
                    self.make_token(side, t, 1);
                }
            }
            None => {}
        }
    }

    /// `who` discards `n`: the player chooses; the computer its least.
    fn discard(&mut self, who: usize, n: usize) -> bool {
        if n == 0 || self.s[who].hand.is_empty() {
            return false;
        }
        self.forcer = who;
        if self.human[who] {
            self.pending = Some(Pending {
                who,
                ask: Ask::Discard { n },
            });
            return true;
        }
        for _ in 0..n {
            if self.s[who].hand.is_empty() {
                break;
            }
            let i = crate::duel_ai::discard_choice(self, who);
            self.discard_at(who, i);
        }
        false
    }

    fn discard_at(&mut self, who: usize, i: usize) {
        if i >= self.s[who].hand.len() {
            return;
        }
        let c = self.s[who].hand.remove(i);
        let name = self.card(c).name.clone();
        self.put_in_grave(who, c);
        self.log(
            Ev::Ability,
            who as u8,
            "log.discard",
            vec![("side", self.side_name(who)), ("card", name)],
            vec![],
        );
    }

    fn computer_sacrifice(&mut self, side: usize) {
        let pick = self
            .perms
            .iter()
            .filter(|p| p.ctrl == side && self.is_creature(p))
            .min_by_key(|p| (self.value_of(p.id), p.id))
            .map(|p| p.id);
        if let Some(id) = pick {
            self.sacrifice(id);
        }
    }

    fn counter_item(&mut self, item: u32, by: &str) {
        let Some(pos) = self.stack.iter().position(|x| x.id == item) else {
            return;
        };
        if let Source::Spell { card, .. } = self.stack[pos].src {
            if self.rules(card).kw & kw::CANT_BE_COUNTERED != 0 {
                return;
            }
        }
        let it = self.stack.remove(pos);
        self.log(
            Ev::Counter,
            it.ctrl as u8,
            "log.countered",
            vec![("by", by.to_string()), ("card", it.name.clone())],
            vec![],
        );
        self.finish_spell(&it);
    }

    /// A card goes to its owner's graveyard, or to exile under Dryad Militant.
    fn put_in_grave(&mut self, owner: usize, card: usize) {
        if self.any_static(Static::NoGraveyards)
            || (self.card(card).is_spell() && self.any_static(Static::SpellsToExile))
        {
            self.s[owner].exile.push(card);
        } else if self.rules(card).statics.contains(&Static::ShuffleBack) {
            // Worldspine Wurm: shuffled into its owner's library instead
            self.s[owner].library.push(card);
            let mut lib = std::mem::take(&mut self.s[owner].library);
            self.shuffle(&mut lib);
            self.s[owner].library = lib;
        } else {
            self.s[owner].grave.push(card);
        }
    }

    // ---------- damage and leaving ----------

    /// `by` is the side whose spell or creature deals it.
    fn damage_perm(&mut self, by: usize, src: Option<u32>, src_name: &str, id: u32, n: i32) {
        if n <= 0 || self.perm(id).is_none() {
            return;
        }
        // Palisade Giant takes the damage to its controller's other permanents
        let owner = self.perm(id).map(|p| p.ctrl).unwrap_or(by);
        let id = self.palisade(owner, Some(id)).unwrap_or(id);
        if let Some(s) = src {
            self.set(s, |p| p.dealt = true);
        }
        if self
            .perm(id)
            .map(|p| self.is_walker(p) && !self.is_creature(p))
            .unwrap_or(false)
        {
            self.set(id, |p| p.loyalty -= n);
            let name = self.name_of(id);
            self.log(
                Ev::Hit,
                by as u8,
                "log.damage",
                vec![
                    ("source", src_name.to_string()),
                    ("n", n.to_string()),
                    ("target", name),
                ],
                src.into_iter().chain([id]).collect(),
            );
            self.lifelink(src, n);
            return;
        }
        let deathtouch = src
            .and_then(|s| self.perm(s))
            .map(|s| self.has(s, kw::DEATHTOUCH))
            .unwrap_or(false);
        self.set(id, |p| {
            p.damage += n;
            if deathtouch {
                p.deathtouched = true;
            }
        });
        let name = self.name_of(id);
        let mut focus = Vec::new();
        if let Some(s) = src {
            focus.push(s);
        }
        focus.push(id);
        self.log(
            Ev::Hit,
            by as u8,
            "log.damage",
            vec![
                ("source", src_name.to_string()),
                ("n", n.to_string()),
                ("target", name),
            ],
            focus,
        );
        self.lifelink(src, n);
        self.was_damaged(id);
    }

    fn damage_player(&mut self, src: Option<u32>, src_name: &str, ctrl: usize, side: usize, n: i32) {
        let n = self.prevented(side, n, self.in_combat_damage);
        if n <= 0 {
            return;
        }
        if let Some(giant) = self.palisade(side, None) {
            self.damage_perm(ctrl, src, src_name, giant, n);
            return;
        }
        if let Some(s) = src {
            self.set(s, |p| p.dealt = true);
        }
        self.s[side].life -= n;
        self.s[side].lost += n;
        self.side_damaged(side);
        let name = if src_name.is_empty() {
            self.side_name(ctrl)
        } else {
            src_name.to_string()
        };
        self.log(
            Ev::Face,
            side as u8,
            "log.face",
            vec![
                ("source", name),
                ("n", n.to_string()),
                ("side", self.side_name(side)),
            ],
            src.into_iter().collect(),
        );
        self.lifelink(src, n);
        if let Some(s) = src.and_then(|s| self.perm(s)).cloned() {
            if let Some(ab) = s.card.and_then(|c| self.rules(c).damages_player.clone()) {
                let t = self.trig_of(&s, ab, 0);
                self.triggers.push(t);
            }
        }
    }

    fn lifelink(&mut self, src: Option<u32>, n: i32) {
        if let Some(s) = src.and_then(|s| self.perm(s)) {
            if self.has(s, kw::LIFELINK) {
                let c = s.ctrl;
                self.gain(c, n);
            }
        }
    }

    /// Destroy: regeneration and the Golgari set can save it.
    fn destroy(&mut self, id: u32) {
        let Some(p) = self.perm(id).cloned() else { return };
        if self.has(&p, kw::INDESTRUCTIBLE) {
            return;
        }
        let name = self.perm_name(&p);
        if self.is_creature(&p) {
            if p.regen > 0 {
                self.regenerate(id);
                return;
            }
            if self.s[p.ctrl].perks & perk::SET_REGENERATE != 0 && !self.s[p.ctrl].regen_used {
                self.s[p.ctrl].regen_used = true;
                self.set(id, |q| q.regen += 1);
                self.regenerate(id);
                return;
            }
        }
        if p.exile_if_dies {
            self.exile(id);
            return;
        }
        self.log(Ev::Dies, p.ctrl as u8, "log.dies", vec![("card", name)], vec![id]);
        self.dies(id);
    }

    fn regenerate(&mut self, id: u32) {
        self.set(id, |p| {
            p.regen = p.regen.saturating_sub(1);
            p.tapped = true;
            p.damage = 0;
            p.deathtouched = false;
            p.attacking = false;
            p.blocking = None;
        });
        let name = self.name_of(id);
        self.log(Ev::Ability, 0, "log.regenerate", vec![("card", name)], vec![id]);
    }

    /// To the graveyard from the battlefield, with its dies triggers.
    fn dies(&mut self, id: u32) {
        let Some(p) = self.perm(id).cloned() else { return };
        if p.exile_if_dies {
            self.exile(id);
            return;
        }
        let creature = self.is_creature(&p);
        let power = self.power(&p);
        self.leave(id);
        if let Some(c) = p.card {
            if creature
                && self.s[p.owner].perks & perk::REGROWTH != 0
                && !self.s[p.owner].regrowth_used
                && p.owner == p.ctrl
            {
                self.s[p.owner].regrowth_used = true;
                self.s[p.owner].hand.push(c);
                let name = self.card(c).name.clone();
                self.log(
                    Ev::Ability,
                    p.owner as u8,
                    "log.regrowth",
                    vec![("card", name)],
                    vec![],
                );
            } else {
                self.put_in_grave(p.owner, c);
            }
            if let Some(ab) = self.rules(c).dies.clone() {
                let t = self.trig_of(&p, ab, 0);
                self.triggers.push(t);
            }
        }
        if creature {
            self.enemy_died(&p);
        }
        if creature {
            let watchers: Vec<Perm> = self
                .perms
                .iter()
                .filter(|w| {
                    w.ctrl == p.ctrl && w.card.map(|c| self.rules(c).ally_dies.is_some()).unwrap_or(false)
                })
                .cloned()
                .collect();
            for w in watchers {
                let ab = w.card.and_then(|c| self.rules(c).ally_dies.clone());
                if let Some(ab) = ab {
                    let t = self.trig_of(&w, ab, power.max(0));
                    self.triggers.push(t);
                }
            }
        }
    }

    fn sacrifice(&mut self, id: u32) {
        let Some(p) = self.perm(id).cloned() else { return };
        let name = self.perm_name(&p);
        self.log(
            Ev::Dies,
            p.ctrl as u8,
            "log.sacrifice",
            vec![("card", name)],
            vec![id],
        );
        self.dies(id);
    }

    fn exile(&mut self, id: u32) {
        let Some(p) = self.perm(id).cloned() else { return };
        let name = self.perm_name(&p);
        self.leave(id);
        if let Some(c) = p.card {
            self.s[p.owner].exile.push(c);
        }
        self.log(
            Ev::Dies,
            p.ctrl as u8,
            "log.exile",
            vec![("card", name)],
            vec![id],
        );
    }

    fn bounce(&mut self, id: u32) {
        let Some(p) = self.perm(id).cloned() else { return };
        let name = self.perm_name(&p);
        self.leave(id);
        if let Some(c) = p.card {
            self.s[p.owner].hand.push(c);
        }
        self.log(
            Ev::Dies,
            p.ctrl as u8,
            "log.bounce",
            vec![("card", name)],
            vec![id],
        );
    }

    /// Takes a permanent off the battlefield and lets go what it held.
    fn leave(&mut self, id: u32) {
        let Some(p) = self.perm(id).cloned() else { return };
        self.perms.retain(|q| q.id != id);
        for q in self.perms.iter_mut() {
            if q.blocking == Some(id) {
                q.blocking = None;
            }
        }
        let release = p.card.and_then(|c| self.rules(c).release);
        for (owner, c) in p.held {
            match release {
                Some(Release::ToBattlefield) => {
                    let host = self.rules(c).aura.map(|a| self.aura_host(owner, a));
                    if host == Some(None) {
                        // an Aura with nothing to enchant stays in exile
                        self.s[owner].exile.push(c);
                        continue;
                    }
                    let nid = self.make_perm(owner, c);
                    if let Some(Some(h)) = host {
                        self.set(nid, |q| q.attached = Some(h));
                    }
                    let name = self.card(c).name.clone();
                    self.log(
                        Ev::Enter,
                        owner as u8,
                        "log.returns",
                        vec![("card", name)],
                        vec![nid],
                    );
                    if self.pending.is_none() && self.rules(c).unleash {
                        self.entering(owner, nid);
                    } else {
                        self.enter_triggers(nid);
                    }
                }
                _ => {
                    self.s[owner].hand.push(c);
                    let name = self.card(c).name.clone();
                    self.log(
                        Ev::Ability,
                        owner as u8,
                        "log.bounce",
                        vec![("card", name)],
                        vec![],
                    );
                }
            }
        }
    }

    /// What a returning Aura enchants, its owner choosing: a hindering one
    /// the opponent's best creature, a helping one their own.
    fn aura_host(&self, owner: usize, a: Aura) -> Option<u32> {
        let want = if a == Aura::EtherealArmor {
            owner
        } else {
            other(owner)
        };
        self.perms
            .iter()
            .filter(|p| self.is_creature(p) && p.ctrl == want)
            .max_by_key(|p| (self.value_of(p.id), p.id))
            .or_else(|| {
                self.perms
                    .iter()
                    .filter(|p| self.is_creature(p))
                    .max_by_key(|p| (self.value_of(p.id), p.id))
            })
            .map(|p| p.id)
    }

    /// State-based actions. True when any applied.
    fn state_based(&mut self) -> bool {
        let lost: Vec<usize> = [P, F].into_iter().filter(|&s| self.s[s].life <= 0).collect();
        if lost.len() == 2 {
            // a draw in Magic; here the foe slips away, as at the turn cap
            self.outcome = Some(Outcome::Withdrew);
            self.log(
                Ev::End,
                F as u8,
                "log.withdrew",
                vec![("side", self.side_name(F))],
                vec![],
            );
            return true;
        }
        if !lost.is_empty() {
            let winner = if lost.contains(&P) { F } else { P };
            self.outcome = Some(if winner == P { Outcome::Won } else { Outcome::Lost });
            self.log(
                Ev::End,
                winner as u8,
                "log.win",
                vec![("side", self.side_name(winner))],
                vec![],
            );
            return true;
        }
        let mut changed = false;
        let ids: Vec<u32> = self.perms.iter().map(|p| p.id).collect();
        for id in ids {
            let Some(p) = self.perm(id).cloned() else { continue };
            if self.is_walker(&p) && !self.is_creature(&p) && p.loyalty <= 0 {
                let name = self.perm_name(&p);
                self.log(
                    Ev::Dies,
                    p.ctrl as u8,
                    "log.walker_falls",
                    vec![("card", name)],
                    vec![id],
                );
                self.dies(id);
                changed = true;
                continue;
            }
            if self.is_creature(&p) {
                let t = self.toughness(&p);
                if t <= 0 {
                    if p.exile_if_dies {
                        self.exile(id);
                    } else {
                        let name = self.perm_name(&p);
                        self.log(Ev::Dies, p.ctrl as u8, "log.dies", vec![("card", name)], vec![id]);
                        self.dies(id);
                    }
                    changed = true;
                    continue;
                }
                if p.damage >= t || (p.deathtouched && p.damage > 0) {
                    self.destroy(id);
                    changed = true;
                    continue;
                }
            }
            if let Some(host) = p.attached {
                let host_ok = self
                    .perm(host)
                    .map(|h| self.is_creature(h) || (self.is_land(h) && self.aura_of(&p).is_none()))
                    .unwrap_or(false);
                if !host_ok {
                    if self.aura_of(&p).is_some() {
                        let c = p.card;
                        let name = self.perm_name(&p);
                        self.log(
                            Ev::Dies,
                            p.ctrl as u8,
                            "log.aura_falls",
                            vec![("card", name)],
                            vec![id],
                        );
                        self.leave(id);
                        if let Some(c) = c {
                            self.put_in_grave(p.owner, c);
                        }
                    } else {
                        self.set(id, |q| q.attached = None);
                    }
                    changed = true;
                }
            }
        }
        // the legend rule: its controller keeps one; the game keeps the newer
        for side in [P, F] {
            let legends: Vec<u32> = self
                .perms
                .iter()
                .filter(|p| p.ctrl == side && p.card.map(|c| self.card(c).legendary).unwrap_or(false))
                .map(|p| p.id)
                .collect();
            for (i, a) in legends.iter().enumerate() {
                for b in legends.iter().skip(i + 1) {
                    if self.name_of(*a) == self.name_of(*b)
                        && self.perm(*a).is_some()
                        && self.perm(*b).is_some()
                    {
                        self.dies(*a);
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    // ---------- combat ----------

    fn declare_attackers(&mut self, ids: Vec<u32>) {
        let a = self.active;
        let mut names = Vec::new();
        for &id in &ids {
            let vig = self.perm(id).map(|p| self.has(p, kw::VIGILANCE)).unwrap_or(false);
            self.set(id, |p| {
                p.attacking = true;
                if !vig {
                    p.tapped = true;
                }
            });
            names.push(self.name_of(id));
        }
        if ids.is_empty() {
            self.enter_step(Step::EndCombat);
            return;
        }
        self.log(
            Ev::Attack,
            a as u8,
            "log.attack",
            vec![("side", self.side_name(a)), ("cards", names.join(", "))],
            ids.clone(),
        );
        for &id in &ids {
            let Some(p) = self.perm(id).cloned() else { continue };
            if let Some(ab) = p.card.and_then(|c| self.rules(c).attacks.clone()) {
                let t = self.trig_of(&p, ab, 0);
                self.triggers.push(t);
            }
        }
        self.attacks_declared(&ids);
    }

    fn declare_blockers(&mut self, pairs: Vec<(u32, u32)>) {
        for (b, at) in pairs {
            self.set(b, |p| p.blocking = Some(at));
            self.set(at, |p| p.blocked = true);
            let (bn, an) = (self.name_of(b), self.name_of(at));
            self.log(
                Ev::Block,
                self.perm(b).map(|p| p.ctrl as u8).unwrap_or(0),
                "log.block",
                vec![("blocker", bn), ("attacker", an)],
                vec![b, at],
            );
        }
        self.lure_blocks();
    }

    fn strikes_now(&self, p: &Perm, first: bool) -> bool {
        let k = self.kws(p);
        if first {
            k & (kw::FIRST_STRIKE | kw::DOUBLE_STRIKE) != 0
        } else {
            k & kw::DOUBLE_STRIKE != 0 || !p.dealt_first
        }
    }

    fn combat_damage(&mut self, first: bool) {
        let defender = other(self.active);
        // (source, target perm or None for the player, amount)
        let mut hits: Vec<(u32, Option<u32>, i32)> = Vec::new();
        let attackers: Vec<Perm> = self.perms.iter().filter(|p| p.attacking).cloned().collect();
        for a in &attackers {
            if !self.strikes_now(a, first) {
                continue;
            }
            let mut power = self.power(a);
            if power <= 0 {
                continue;
            }
            let blockers: Vec<Perm> = self
                .perms
                .iter()
                .filter(|b| b.blocking == Some(a.id))
                .cloned()
                .collect();
            if !a.blocked {
                hits.push((a.id, None, power));
                continue;
            }
            let trample = self.has(a, kw::TRAMPLE);
            let deathtouch = self.has(a, kw::DEATHTOUCH);
            if blockers.is_empty() {
                if trample {
                    hits.push((a.id, None, power));
                }
                continue;
            }
            for (i, b) in blockers.iter().enumerate() {
                let need = if deathtouch {
                    1
                } else {
                    (self.toughness(b) - b.damage).max(0)
                };
                let last = i + 1 == blockers.len();
                let give = if last && !trample { power } else { need.min(power) };
                if give > 0 {
                    hits.push((a.id, Some(b.id), give));
                }
                power -= give;
                if power <= 0 {
                    break;
                }
            }
            if trample && power > 0 {
                hits.push((a.id, None, power));
            }
        }
        let blockers: Vec<Perm> = self
            .perms
            .iter()
            .filter(|b| b.blocking.is_some())
            .cloned()
            .collect();
        for b in &blockers {
            if !self.strikes_now(b, first) {
                continue;
            }
            let power = self.power(b);
            if power > 0 {
                hits.push((b.id, b.blocking, power));
            }
        }
        if first {
            let strikers: Vec<u32> = self
                .perms
                .iter()
                .filter(|p| (p.attacking || p.blocking.is_some()) && self.strikes_now(p, true))
                .map(|p| p.id)
                .collect();
            for s in strikers {
                self.set(s, |p| p.dealt_first = true);
            }
        }
        self.in_combat_damage = true;
        let mut warded: Vec<u32> = Vec::new();
        for (s, t, n) in hits {
            let sname = self.name_of(s);
            // an attacker aimed at a planeswalker strikes it, not the player
            let aim = self
                .perm(s)
                .and_then(|p| p.aim)
                .filter(|w| self.perm(*w).is_some());
            let by = self.perm(s).map(|p| p.ctrl).unwrap_or(self.active);
            match (t, aim) {
                (Some(t), _) => self.damage_perm(by, Some(s), &sname, t, n),
                (None, Some(w)) => {
                    if self.perm(w).map(|p| p.ward).unwrap_or(false) {
                        warded.push(s);
                    }
                    self.damage_perm(by, Some(s), &sname, w, n);
                }
                (None, None) => {
                    let before = self.s[defender].life;
                    self.damage_player(Some(s), &sname, by, defender, n);
                    let dealt = before - self.s[defender].life;
                    if dealt <= 0 {
                        continue;
                    }
                    let Some(p) = self.perm(s).cloned() else { continue };
                    if p.token == Some(Token::Assassin) {
                        self.s[defender].life = 0;
                        self.log(
                            Ev::End,
                            by as u8,
                            "log.assassin",
                            vec![("side", self.side_name(defender))],
                            vec![s],
                        );
                    }
                    if let Some(ab) = p.card.and_then(|c| self.rules(c).hits.clone()) {
                        let tr = self.trig_of(&p, ab, dealt);
                        self.triggers.push(tr);
                    }
                    // Necropolis Regent: that many +1/+1 counters on the creature that hit
                    let regents: Vec<Perm> = self
                        .perms
                        .iter()
                        .filter(|w| {
                            w.ctrl == by
                                && w.card
                                    .map(|c| self.rules(c).when.iter().any(|(x, _)| *x == When::AllyHits))
                                    .unwrap_or(false)
                        })
                        .cloned()
                        .collect();
                    for r in regents {
                        let mut tr = self.trig_of(
                            &r,
                            Ability {
                                target: None,
                                effects: vec![Eff::CountersX],
                            },
                            dealt,
                        );
                        tr.fixed = vec![Target::Perm(s)];
                        self.triggers.push(tr);
                    }
                }
            }
        }
        self.in_combat_damage = false;
        // Vraska's ward: a creature that dealt her combat damage is destroyed
        for s in warded {
            self.destroy(s);
        }
    }

    // ---------- cleanup ----------

    fn cleanup(&mut self) {
        let a = self.active;
        if self.s[a].hand.len() > MAX_HAND {
            let n = self.s[a].hand.len() - MAX_HAND;
            self.resume.push(Resume::Cleanup);
            if self.discard(a, n) {
                return;
            }
            self.resume.pop();
        }
        for p in self.perms.iter_mut() {
            p.damage = 0;
            p.deathtouched = false;
            p.mods.clear();
            p.animated = false;
            p.regen = 0;
            p.exile_if_dies = false;
            p.dealt = false;
            // Traitorous Instinct's loan ends
            if let Some(back) = p.borrowed.take() {
                p.ctrl = back;
            }
        }
        if self.state_based() {
            // a toughness boost ending can kill; triggers then get their turn next turn
        }
        self.begin_turn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{find, for_tests};

    fn cards() -> Rc<Vec<Card>> {
        Rc::new(for_tests())
    }

    fn names(cards: &[Card], n: &[&str]) -> Vec<usize> {
        n.iter().map(|x| find(cards, x)).collect()
    }

    fn setup(cards: &[Card], me: &[&str], foe: &[&str], my_lands: &[&str], foe_lands: &[&str]) -> Setup {
        Setup {
            sides: [
                SideSetup {
                    name: "Your side".into(),
                    life: 20,
                    deck: names(cards, me),
                    lands: names(cards, my_lands),
                    in_play: Vec::new(),
                    perks: 0,
                },
                SideSetup {
                    name: "The foe".into(),
                    life: 20,
                    deck: names(cards, foe),
                    lands: names(cards, foe_lands),
                    in_play: Vec::new(),
                    perks: 0,
                },
            ],
            seed: 7,
        }
    }

    /// A game with no human: both sides played by the computer.
    fn auto(cards: Rc<Vec<Card>>, s: &Setup) -> Duel {
        let mut d = Duel::new(cards, s, [false, false]);
        d.run();
        d
    }

    #[test]
    fn a_game_between_two_computers_ends() {
        let c = cards();
        let s = setup(
            &c,
            &[
                "Gore-House Chainwalker",
                "Splatter Thug",
                "Bloodfray Giant",
                "Annihilating Fire",
                "Cobblebrute",
                "Ash Zealot",
                "Electrickery",
                "Tenement Crasher",
            ],
            &[
                "Drudge Beetle",
                "Brushstrider",
                "Towering Indrik",
                "Giant Growth",
                "Korozda Monitor",
                "Archweaver",
                "Savage Surge",
                "Axebane Stag",
            ],
            &[
                "Mountain", "Mountain", "Mountain", "Mountain", "Mountain", "Mountain",
            ],
            &[
                "Forest", "Forest", "Forest", "Forest", "Forest", "Forest", "Forest",
            ],
        );
        let d = auto(c, &s);
        assert!(d.outcome.is_some());
        assert!(d.turn > 2, "the game lasted {} turns", d.turn);
    }

    #[test]
    fn a_land_is_played_from_the_pile_once_a_turn() {
        let c = cards();
        let s = setup(
            &c,
            &["Cobblebrute"; 8],
            &["Cobblebrute"; 8],
            &["Mountain"; 5],
            &["Mountain"; 5],
        );
        let mut d = Duel::new(c, &s, [true, false]);
        d.run();
        // the player's first main phase: lands and nothing castable but a land
        assert!(matches!(
            d.pending,
            Some(Pending {
                who: P,
                ask: Ask::Priority
            })
        ));
        assert_eq!(d.step, Step::Main1);
        let acts = d.actions(P);
        assert_eq!(acts[0], Action::Land(0));
        d.answer(P, Answer::Act(Action::Land(0))).unwrap();
        assert_eq!(d.s[P].pile.len(), 4);
        // with nothing castable the player passes on; through the turns, one land a turn each
        d.human = [false, false];
        d.run();
        for t in 1..=d.turn {
            let lands = d
                .events
                .iter()
                .filter(|e| e.turn == t && e.ev == Ev::Land)
                .count();
            assert!(lands <= 1, "turn {t}: {lands} lands");
        }
        assert!(
            d.events
                .iter()
                .filter(|e| e.ev == Ev::Land && e.side == 0)
                .count()
                >= 5
        );
    }

    #[test]
    fn mana_is_spent_from_untapped_lands_and_empties_by_the_turn() {
        let c = cards();
        let s = setup(
            &c,
            &["Concordia Pegasus"; 8],
            &["Cobblebrute"; 8],
            &["Plains"; 5],
            &["Mountain"; 5],
        );
        let mut d = Duel::new(c.clone(), &s, [true, false]);
        d.run();
        d.answer(P, Answer::Act(Action::Land(0))).unwrap();
        // one Plains: a {1}{W} Pegasus can't be cast yet
        assert!(!d.actions(P).iter().any(|a| matches!(a, Action::Cast { .. })));
        let peg = find(&c, "Concordia Pegasus");
        let id = d.make_perm(P, find(&c, "Plains"));
        assert!(d.can_pay(P, &d.cast_cost(P, peg, false), None));
        d.set(id, |p| p.tapped = true);
        assert!(!d.can_pay(P, &d.cast_cost(P, peg, false), None));
    }

    #[test]
    fn a_counterspell_stops_a_spell_and_both_go_to_the_graveyard() {
        let c = cards();
        let s = setup(
            &c,
            &["Cobblebrute"; 6],
            &["Cancel"; 6],
            &["Mountain"; 6],
            &["Island"; 6],
        );
        let mut d = Duel::new(c.clone(), &s, [true, true]);
        d.run();
        for _ in 0..4 {
            let id = d.make_perm(P, find(&c, "Mountain"));
            d.set(id, |p| p.fresh = false);
            let id = d.make_perm(F, find(&c, "Island"));
            d.set(id, |p| p.fresh = false);
        }
        let brute = d.s[P]
            .hand
            .iter()
            .position(|&x| x == find(&c, "Cobblebrute"))
            .unwrap();
        d.answer(
            P,
            Answer::Act(Action::Cast {
                hand: brute,
                mode: 0,
                overload: false,
            }),
        )
        .unwrap();
        assert_eq!(d.stack.len(), 1);
        // the snapshot shows the spell on the stack as its card
        assert_eq!(
            d.snap().stack,
            vec![(P as u8, "Cobblebrute".to_string(), find(&c, "Cobblebrute") as i32)]
        );
        // the foe is asked, with Cancel in hand and three Islands untapped
        let item = d.stack[0].id;
        assert!(matches!(
            d.pending,
            Some(Pending {
                who: F,
                ask: Ask::Priority
            })
        ));
        let cancel = d.s[F].hand.iter().position(|&x| x == find(&c, "Cancel")).unwrap();
        d.answer(
            F,
            Answer::Act(Action::Cast {
                hand: cancel,
                mode: 0,
                overload: false,
            }),
        )
        .unwrap();
        d.answer(F, Answer::Targets(vec![Target::Item(item)])).unwrap();
        // both pass: Cancel resolves and counters the Cobblebrute
        while !d.stack.is_empty() {
            let who = d.pending.as_ref().unwrap().who;
            d.answer(who, Answer::Act(Action::Pass)).unwrap();
        }
        assert!(d.s[P].grave.contains(&find(&c, "Cobblebrute")));
        assert!(d.s[F].grave.contains(&find(&c, "Cancel")));
        assert!(!d.perms.iter().any(|p| p.card == Some(find(&c, "Cobblebrute"))));
    }

    fn table(c: &Rc<Vec<Card>>) -> Duel {
        let s = setup(
            c,
            &["Cobblebrute"; 6],
            &["Cobblebrute"; 6],
            &["Mountain"; 6],
            &["Mountain"; 6],
        );
        let mut d = Duel::new(c.clone(), &s, [true, true]);
        d.pending = None;
        d
    }

    fn put(d: &mut Duel, side: usize, name: &str) -> u32 {
        let c = find(&d.cards.clone(), name);
        let id = d.make_perm(side, c);
        d.set(id, |p| p.fresh = false);
        id
    }

    /// What the computer does when it has priority at its own main phase.
    fn ai_move(d: &mut Duel, who: usize) -> Action {
        d.active = who;
        d.step = Step::Main1;
        d.s[who].pile.clear();
        d.pending = Some(Pending {
            who,
            ask: Ask::Priority,
        });
        match crate::duel_ai::answer(d, who) {
            Answer::Act(a) => a,
            other => panic!("expected an action, got {other:?}"),
        }
    }

    #[test]
    fn swampwalk_counts_any_land_with_the_swamp_type() {
        let c = cards();
        let mut d = table(&c);
        let shambler = put(&mut d, P, "Sewer Shambler"); // swampwalk
        d.set(shambler, |p| p.attacking = true);
        let blocker = put(&mut d, F, "Cobblebrute");
        let b = d.perm(blocker).unwrap().clone();
        assert!(d.can_block(&b, shambler), "no Swamp yet: it can be blocked");
        put(&mut d, F, "Overgrown Tomb"); // a Swamp Forest, not named Swamp
        assert!(
            !d.can_block(&b, shambler),
            "a land with the Swamp type lets it walk"
        );
    }

    #[test]
    fn the_computer_draws_with_an_ability_at_the_end_of_your_turn_not_its_own() {
        let c = cards();
        let mut d = table(&c);
        let chem = put(&mut d, F, "Mercurial Chemister"); // {U}, tap: draw two
        put(&mut d, F, "Island");
        // its own main phase: it keeps the Chemister back
        assert_ne!(ai_move(&mut d, F), Action::Activate { perm: chem, n: 0 });
        // your end step: it draws
        d.active = P;
        d.step = Step::End;
        d.pending = Some(Pending {
            who: F,
            ask: Ask::Priority,
        });
        assert_eq!(
            crate::duel_ai::answer(&d, F),
            Answer::Act(Action::Activate { perm: chem, n: 0 })
        );
    }

    #[test]
    fn the_computer_spends_a_planeswalkers_loyalty() {
        let c = cards();
        // Vraska at 5 facing a creature: -3 destroys it (ability 1), leaving her at 2
        let mut d = table(&c);
        let v = put(&mut d, F, "Vraska the Unseen");
        d.set(v, |p| p.loyalty = 5);
        put(&mut d, P, "Cobblebrute");
        assert_eq!(ai_move(&mut d, F), Action::Activate { perm: v, n: 1 });
        // nothing to destroy: the +1 (ability 0)
        let mut d = table(&c);
        let v = put(&mut d, F, "Vraska the Unseen");
        d.set(v, |p| p.loyalty = 5);
        assert_eq!(ai_move(&mut d, F), Action::Activate { perm: v, n: 0 });
        // Jace at 8 can afford his -8 ultimate (ability 2) and takes it
        let mut d = table(&c);
        let j = put(&mut d, F, "Jace, Architect of Thought");
        d.set(j, |p| p.loyalty = 8);
        assert_eq!(ai_move(&mut d, F), Action::Activate { perm: j, n: 2 });
        // Jace at 2: his -2 would kill him, so the +1
        let mut d = table(&c);
        let j = put(&mut d, F, "Jace, Architect of Thought");
        d.set(j, |p| p.loyalty = 2);
        assert_eq!(ai_move(&mut d, F), Action::Activate { perm: j, n: 0 });
    }

    #[test]
    fn first_strike_kills_a_blocker_before_it_strikes_back() {
        let c = cards();
        let mut d = table(&c);
        d.active = P;
        let thug = put(&mut d, P, "Splatter Thug"); // 2/2 first strike
        let chain = put(&mut d, F, "Gore-House Chainwalker"); // 2/1
        d.set(thug, |p| p.attacking = true);
        d.set(thug, |p| p.blocked = true);
        d.set(chain, |p| p.blocking = Some(thug));
        d.combat_damage(true);
        d.state_based();
        assert!(
            d.perm(chain).is_none(),
            "the blocker should die in the first-strike step"
        );
        d.combat_damage(false);
        assert_eq!(d.perm(thug).unwrap().damage, 0);
    }

    #[test]
    fn trample_and_deathtouch_assign_one_to_the_blocker() {
        let c = cards();
        let mut d = table(&c);
        d.active = P;
        let arch = put(&mut d, P, "Archweaver"); // 5/5 trample
        d.set(arch, |p| p.mods.push((0, 0, kw::DEATHTOUCH)));
        let wall = put(&mut d, F, "Hover Barrier"); // 0/6
        d.set(arch, |p| {
            p.attacking = true;
            p.blocked = true;
        });
        d.set(wall, |p| p.blocking = Some(arch));
        let life = d.s[F].life;
        d.combat_damage(false);
        assert_eq!(d.s[F].life, life - 4);
        d.state_based();
        assert!(d.perm(wall).is_none());
    }

    #[test]
    fn a_detained_creature_cannot_attack_until_the_detainers_next_turn() {
        let c = cards();
        let mut d = table(&c);
        let brute = put(&mut d, F, "Cobblebrute");
        d.detain(brute, P);
        // the foe's own turn begins: still detained, it can't attack
        d.active = P;
        d.begin_turn();
        assert_eq!(d.active, F);
        assert!(!d.can_attack(d.perm(brute).unwrap()));
        // the player's next turn begins: the detention ends
        d.begin_turn();
        assert_eq!(d.active, P);
        assert!(d.perm(brute).unwrap().detained_by.is_none());
    }

    #[test]
    fn overload_reaches_each_creature_the_caster_does_not_control() {
        let c = cards();
        let mut d = table(&c);
        let mine = put(&mut d, P, "Gore-House Chainwalker");
        let a = put(&mut d, F, "Gore-House Chainwalker");
        let b = put(&mut d, F, "Rakdos Cackler");
        let card = find(&c, "Electrickery");
        d.stack.push(Item {
            id: 900,
            ctrl: P,
            src: Source::Spell {
                card,
                mode: 0,
                overload: true,
            },
            ability: d.rules(card).modes[0].clone(),
            targets: Vec::new(),
            each: true,
            x: 0,
            name: "Electrickery".into(),
            tctrl: Vec::new(),
        });
        d.resolve_top();
        d.state_based();
        assert!(d.perm(a).is_none() && d.perm(b).is_none());
        assert!(d.perm(mine).is_some());
    }

    #[test]
    fn a_spell_whose_only_target_left_does_nothing() {
        let c = cards();
        let mut d = table(&c);
        let a = put(&mut d, F, "Cobblebrute");
        // Inaction Injunction: detain it, and draw; with its target gone, no draw either
        let card = find(&c, "Inaction Injunction");
        d.stack.push(Item {
            id: 901,
            ctrl: P,
            src: Source::Spell {
                card,
                mode: 0,
                overload: false,
            },
            ability: d.rules(card).modes[0].clone(),
            targets: vec![Target::Perm(a)],
            each: false,
            x: 0,
            name: "Inaction Injunction".into(),
            tctrl: Vec::new(),
        });
        d.bounce(a);
        // a card to draw, so a draw would show
        d.s[P].library.push(find(&c, "Cobblebrute"));
        let hand = d.s[P].hand.len();
        d.resolve_top();
        assert_eq!(d.s[P].hand.len(), hand);
        assert!(d.s[P].grave.contains(&card));
    }

    #[test]
    fn hexproof_keeps_a_creature_from_the_opponents_targets_only() {
        let c = cards();
        let mut d = table(&c);
        let rhino = put(&mut d, F, "Rubbleback Rhino");
        let spec = Tgt::One(crate::rules::creatures(Who::Any));
        assert!(!d.legal_targets(P, &spec, None).contains(&Target::Perm(rhino)));
        assert!(d.legal_targets(F, &spec, None).contains(&Target::Perm(rhino)));
    }

    #[test]
    fn detention_sphere_returns_what_it_held_when_it_leaves() {
        let c = cards();
        let mut d = table(&c);
        let a = put(&mut d, F, "Rakdos Cackler");
        let b = put(&mut d, F, "Rakdos Cackler");
        let sphere = put(&mut d, P, "Detention Sphere");
        let card = find(&c, "Detention Sphere");
        let item = Item {
            id: 902,
            ctrl: P,
            src: Source::Ability {
                perm: sphere,
                card: Some(card),
            },
            ability: d.rules(card).etb.clone().unwrap(),
            targets: vec![Target::Perm(a)],
            each: false,
            x: 0,
            name: "Detention Sphere".into(),
            tctrl: Vec::new(),
        };
        d.run_effects(Box::new(item), 0);
        assert!(
            d.perm(a).is_none() && d.perm(b).is_none(),
            "both Cacklers share a name"
        );
        assert_eq!(d.perm(sphere).unwrap().held.len(), 2);
        d.bounce(sphere);
        let back = d
            .perms
            .iter()
            .filter(|p| p.ctrl == F && p.card == Some(find(&c, "Rakdos Cackler")))
            .count();
        assert_eq!(back, 2);
    }

    #[test]
    fn a_regeneration_shield_saves_a_creature_from_destruction() {
        let c = cards();
        let mut d = table(&c);
        let g = put(&mut d, P, "Grim Roustabout");
        d.set(g, |p| p.regen = 1);
        d.destroy(g);
        let p = d.perm(g).expect("regenerated");
        assert!(p.tapped && p.regen == 0);
        d.destroy(g);
        assert!(d.perm(g).is_none());
    }

    #[test]
    fn the_same_seed_and_answers_give_the_same_game() {
        let c = cards();
        let s = setup(
            &c,
            &[
                "Trained Caracal",
                "Azorius Arrester",
                "Sunspire Griffin",
                "Arrest",
                "Swift Justice",
                "Seller of Songbirds",
                "Concordia Pegasus",
                "Phantom General",
            ],
            &[
                "Rakdos Cackler",
                "Dead Reveler",
                "Mind Rot",
                "Ultimate Price",
                "Daggerdrome Imp",
                "Grim Roustabout",
                "Slum Reaper",
                "Catacomb Slug",
            ],
            &["Plains"; 7],
            &["Swamp"; 7],
        );
        let a = auto(c.clone(), &s);
        let b = auto(c, &s);
        assert_eq!(a.events, b.events);
        assert_eq!(a.outcome, b.outcome);
    }

    #[test]
    fn an_unleashed_creature_cannot_block() {
        let c = cards();
        let mut d = table(&c);
        d.active = P;
        let att = put(&mut d, P, "Cobblebrute");
        d.set(att, |p| p.attacking = true);
        let dr = put(&mut d, F, "Dead Reveler");
        assert!(d.can_block(d.perm(dr).unwrap(), att));
        d.set(dr, |p| {
            p.unleashed = true;
            p.counters = 1;
        });
        assert!(!d.can_block(d.perm(dr).unwrap(), att));
    }

    #[test]
    fn a_flier_is_blocked_only_by_flying_or_reach() {
        let c = cards();
        let mut d = table(&c);
        d.active = P;
        let peg = put(&mut d, P, "Concordia Pegasus");
        d.set(peg, |p| p.attacking = true);
        let brute = put(&mut d, F, "Cobblebrute");
        let indrik = put(&mut d, F, "Towering Indrik");
        let griffin = put(&mut d, F, "Sunspire Griffin");
        assert!(!d.can_block(d.perm(brute).unwrap(), peg));
        assert!(d.can_block(d.perm(indrik).unwrap(), peg));
        assert!(d.can_block(d.perm(griffin).unwrap(), peg));
        // only an attacker can be blocked
        d.set(peg, |p| p.attacking = false);
        assert!(!d.can_block(d.perm(griffin).unwrap(), peg));
    }

    #[test]
    fn a_creature_damaged_by_annihilating_fire_is_exiled_even_when_sacrificed() {
        let c = cards();
        let mut d = table(&c);
        let g = put(&mut d, F, "Cobblebrute");
        d.set(g, |p| p.exile_if_dies = true);
        d.sacrifice(g);
        let brute = find(&c, "Cobblebrute");
        assert!(d.s[F].exile.contains(&brute) && !d.s[F].grave.contains(&brute));
    }

    #[test]
    fn izzet_charm_asks_no_payment_for_a_spell_that_cannot_be_countered() {
        let c = cards();
        let mut d = table(&c);
        for _ in 0..3 {
            put(&mut d, F, "Forest");
        }
        let decay = find(&c, "Abrupt Decay");
        d.stack.push(Item {
            id: 950,
            ctrl: F,
            src: Source::Spell {
                card: decay,
                mode: 0,
                overload: false,
            },
            ability: d.rules(decay).modes[0].clone(),
            targets: Vec::new(),
            each: false,
            x: 0,
            name: "Abrupt Decay".into(),
            tctrl: Vec::new(),
        });
        let charm = find(&c, "Izzet Charm");
        let item = Item {
            id: 951,
            ctrl: P,
            src: Source::Spell {
                card: charm,
                mode: 0,
                overload: false,
            },
            ability: d.rules(charm).modes[0].clone(),
            targets: vec![Target::Item(950)],
            each: false,
            x: 0,
            name: "Izzet Charm".into(),
            tctrl: Vec::new(),
        };
        d.run_effects(Box::new(item), 0);
        assert!(d.pending.is_none(), "no payment is asked");
        assert!(d.stack.iter().any(|i| i.id == 950));
        assert_eq!(d.mana_available(F), 3);
    }

    #[test]
    fn a_keyrune_can_tap_for_its_own_animation() {
        let c = cards();
        let mut d = table(&c);
        d.active = P;
        d.step = Step::Main1;
        let rune = put(&mut d, P, "Azorius Keyrune"); // {W}{U}: animate; taps for W or U
        put(&mut d, P, "Plains");
        assert!(d.actions(P).contains(&Action::Activate { perm: rune, n: 0 }));
    }

    #[test]
    fn lifelink_gains_what_it_deals() {
        let c = cards();
        let mut d = table(&c);
        d.active = P;
        let imp = put(&mut d, P, "Daggerdrome Imp"); // 1/1 flying, lifelink
        d.set(imp, |p| p.attacking = true);
        let (me, foe) = (d.s[P].life, d.s[F].life);
        d.combat_damage(false);
        assert_eq!((d.s[P].life, d.s[F].life), (me + 1, foe - 1));
    }

    #[test]
    fn goblin_electromancer_takes_one_off_instants_and_sorceries_only() {
        let c = cards();
        let mut d = table(&c);
        put(&mut d, P, "Goblin Electromancer");
        let fire = find(&c, "Annihilating Fire"); // {1}{R}{R}
        let brute = find(&c, "Cobblebrute"); // {3}{R}
        assert_eq!(d.cast_cost(P, fire, false).value(), 2);
        assert_eq!(d.cast_cost(P, brute, false).value(), 4);
        assert_eq!(d.cast_cost(F, fire, false).value(), 3);
    }

    #[test]
    fn ethereal_armor_counts_its_controllers_enchantments_and_gives_first_strike() {
        let c = cards();
        let mut d = table(&c);
        let ace = put(&mut d, P, "Fencing Ace"); // 1/1
        let armor = put(&mut d, P, "Ethereal Armor");
        d.set(armor, |p| p.attached = Some(ace));
        assert_eq!(
            (d.power(d.perm(ace).unwrap()), d.toughness(d.perm(ace).unwrap())),
            (2, 2)
        );
        put(&mut d, P, "Shrieking Affliction");
        assert_eq!(d.power(d.perm(ace).unwrap()), 3);
        assert!(d.kws(d.perm(ace).unwrap()) & kw::FIRST_STRIKE != 0);
    }

    #[test]
    fn scavenging_exiles_the_card_and_gives_counters_equal_to_its_power() {
        let c = cards();
        let mut d = table(&c);
        let beetle = put(&mut d, P, "Drudge Beetle");
        for _ in 0..6 {
            put(&mut d, P, "Forest");
        }
        d.s[P].grave.push(find(&c, "Deadbridge Goliath")); // power 5, scavenge {4}{G}{G}
        d.active = P;
        d.step = Step::Main1;
        d.pending = Some(Pending {
            who: P,
            ask: Ask::Priority,
        });
        let g = d
            .actions(P)
            .into_iter()
            .find(|a| matches!(a, Action::Scavenge { .. }))
            .expect("scavenge offered");
        d.answer(P, Answer::Act(g)).unwrap();
        d.answer(P, Answer::Targets(vec![Target::Perm(beetle)])).unwrap();
        while !d.stack.is_empty() {
            let who = d.pending.as_ref().unwrap().who;
            d.answer(who, Answer::Act(Action::Pass)).unwrap();
        }
        assert_eq!(d.perm(beetle).unwrap().counters, 5);
        assert!(d.s[P].exile.contains(&find(&c, "Deadbridge Goliath")));
        assert!(d.s[P].grave.is_empty());
    }

    #[test]
    fn an_empty_library_reshuffles_the_graveyard_for_half_your_life() {
        let c = cards();
        let mut d = table(&c);
        d.s[P].library.clear();
        d.s[P].grave = vec![find(&c, "Cobblebrute"), find(&c, "Cobblebrute")];
        // 20 life: lose 10; then at 9, lose 5 (half, rounded up)
        d.draw(P, 1);
        assert_eq!(d.s[P].life, 10);
        assert_eq!(d.s[P].library.len(), 1);
        d.s[P].life = 9;
        d.s[P].library.clear();
        d.s[P].grave = vec![find(&c, "Cobblebrute")];
        d.draw(P, 1);
        assert_eq!(d.s[P].life, 4);
    }
}
