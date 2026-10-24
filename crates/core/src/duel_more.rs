//! The rest of Return to Ravnica's rules, beside the engine in `duel.rs`:
//! the effects the first region's cards did not need (X spells, mill,
//! planeswalkers, control changes, coin flips, library searches, free
//! casting, an extra turn), the costs beyond mana, the land entries, and the
//! moments the newer cards watch for (each combat, the end step, a land
//! becoming tapped, a creature entering).
//!
//! A few choices are made for the player and the computer alike, where a
//! prompt would add a question for little (SECOND-ORDER R5.1): which basics
//! go to the land pile, the two colours Tablet of the Guilds names, how a pile is
//! split for Jace, and what Epic Experiment and Jace's last ability cast.

use super::*;

impl Duel {
    // ---------- counters, life, discards ----------

    /// Puts +1/+1 counters on a permanent: twice as many under Corpsejack Menace.
    pub(super) fn add_counters(&mut self, id: u32, n: i32) {
        let Some(p) = self.perm(id) else { return };
        let n = if self.is_creature(p) && self.statics_count(p.ctrl, Static::DoubleCounters) > 0 {
            n * 2 * self.statics_count(p.ctrl, Static::DoubleCounters)
        } else {
            n
        };
        self.set(id, |p| p.counters += n);
        let name = self.name_of(id);
        self.log(
            Ev::Ability,
            0,
            "log.counters",
            vec![("card", name), ("n", n.to_string())],
            vec![id],
        );
    }

    /// A side loses life by a rule or a cost (not damage).
    pub(super) fn lose(&mut self, side: usize, n: i32) {
        if n <= 0 {
            return;
        }
        self.s[side].life -= n;
        self.s[side].lost += n;
        self.log(
            Ev::Life,
            side as u8,
            "log.lose",
            vec![("side", self.side_name(side)), ("n", n.to_string())],
            vec![],
        );
    }

    /// `who` discards `n` at random.
    fn discard_random(&mut self, who: usize, n: usize, by: usize) {
        for _ in 0..n {
            if self.s[who].hand.is_empty() {
                return;
            }
            let i = self.rng.below(self.s[who].hand.len() as u32) as usize;
            self.discard_by(who, i, by);
        }
    }

    /// A discard forced by `by`: Loxodon Smiter, made to be discarded by an
    /// opponent, goes onto the battlefield instead.
    pub(super) fn discard_by(&mut self, who: usize, i: usize, by: usize) {
        let Some(&c) = self.s[who].hand.get(i) else { return };
        if by != who && self.rules(c).statics.contains(&Static::Smiter) {
            self.s[who].hand.remove(i);
            let id = self.make_perm(who, c);
            self.entering(who, id);
            return;
        }
        self.discard_at(who, i);
    }

    fn mill(&mut self, who: usize, n: usize) {
        let mut names = Vec::new();
        for _ in 0..n {
            let Some(c) = self.s[who].library.pop() else { break };
            names.push(self.card(c).name.clone());
            self.put_in_grave(who, c);
        }
        if !names.is_empty() {
            self.log(
                Ev::Ability,
                who as u8,
                "log.mill",
                vec![("side", self.side_name(who)), ("n", names.len().to_string())],
                vec![],
            );
        }
    }

    fn first_player(item: &Item) -> Option<usize> {
        item.targets.iter().find_map(|t| match t {
            Target::Player(p) => Some(*p),
            _ => None,
        })
    }

    // ---------- mana and lands ----------

    /// A permanent became tapped: Chronic Flooding's land mills its controller three.
    pub(super) fn tapped_for_mana(&mut self, id: u32) {
        let floods = self
            .perms
            .iter()
            .filter(|a| a.attached == Some(id) && self.aura_of(a) == Some(Aura::Flooding))
            .count();
        if floods == 0 {
            return;
        }
        let Some(owner) = self.perm(id).map(|p| p.ctrl) else {
            return;
        };
        for _ in 0..floods {
            self.mill(owner, 3);
        }
    }

    /// A land has entered from the pile: a shock land asks for 2 life,
    /// Transguild Promenade enters tapped and asks for {1}.
    pub(super) fn land_entered(&mut self, side: usize, id: u32) {
        let Some(c) = self.perm(id).and_then(|p| p.card) else {
            return;
        };
        let what = match self.rules(c).entry {
            Entry::Untapped => return,
            Entry::Shock => May::Shock(id),
            Entry::Promenade => {
                self.set(id, |p| p.tapped = true);
                May::Promenade(id)
            }
        };
        self.ask_may(side, what);
    }

    fn ask_may(&mut self, who: usize, what: May) {
        if self.human[who] {
            self.pending = Some(Pending {
                who,
                ask: Ask::May { what },
            });
        } else {
            let y = crate::duel_ai::may(self, who, what);
            self.answer_may(who, what, y);
        }
    }

    pub(super) fn answer_may(&mut self, who: usize, what: May, yes: bool) {
        match what {
            May::Shock(id) => {
                if yes && self.s[who].life > 2 {
                    self.lose(who, 2);
                } else {
                    self.set(id, |p| p.tapped = true);
                }
            }
            May::Promenade(id) => {
                let one = Cost {
                    syms: vec![crate::mana::Sym::Generic(1)],
                };
                if !(yes && self.pay(who, &one, Some(id))) {
                    self.sacrifice(id);
                }
            }
            May::Rummage => {
                if yes && !self.s[who].hand.is_empty() {
                    let i = if self.human[who] {
                        0
                    } else {
                        crate::duel_ai::discard_choice(self, who)
                    };
                    if self.human[who] {
                        // the player chooses the card: a discard, then the draw
                        self.forcer = who;
                        self.pending = Some(Pending {
                            who,
                            ask: Ask::Discard { n: 1 },
                        });
                        self.resume.push(Resume::Draw { who });
                        return;
                    }
                    self.discard_at(who, i);
                    self.draw(who, 1);
                }
            }
            May::Vermin(target) => {
                let b = Cost {
                    syms: vec![crate::mana::Sym::Colour(crate::mana::B)],
                };
                if yes && self.pay(who, &b, None) {
                    let _ = self.discard_or_ask(target, 1, who);
                }
            }
            May::Tithe(id, n) => {
                let cost = Cost {
                    syms: vec![crate::mana::Sym::Generic(n)],
                };
                if !(yes && n > 0 && self.pay(who, &cost, None)) && n > 0 {
                    self.sacrifice(id);
                }
            }
        }
    }

    /// A discard of `n` by `who`, forced by `by`: the player chooses theirs.
    pub(super) fn discard_or_ask(&mut self, who: usize, n: usize, by: usize) -> bool {
        if n == 0 || self.s[who].hand.is_empty() {
            return false;
        }
        if self.human[who] {
            self.forcer = by;
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
            self.discard_by(who, i, by);
        }
        false
    }

    // ---------- costs beyond mana ----------

    /// Pays an activated ability's cost beyond mana. Returns the number the
    /// ability counts from it (a sacrificed creature's power or toughness, a
    /// discarded card's mana value), when it has one.
    pub(super) fn pay_extra(
        &mut self,
        who: usize,
        src: &Perm,
        extra: Extra,
        pick: Option<Target>,
    ) -> Option<i32> {
        match extra {
            Extra::None => None,
            Extra::PayLife(n) => {
                self.lose(who, n);
                None
            }
            Extra::TapTwo => {
                let mut mine: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|p| p.ctrl == who && p.id != src.id && self.is_creature(p) && !p.tapped)
                    .map(|p| p.id)
                    .collect();
                mine.sort_by_key(|&id| self.value_of(id));
                for id in mine.into_iter().take(2) {
                    self.set(id, |p| p.tapped = true);
                }
                None
            }
            Extra::TapHost | Extra::TapHostPayLife => {
                if let Some(h) = src.attached {
                    self.set(h, |p| p.tapped = true);
                    self.tapped_for_mana(h);
                }
                if extra == Extra::TapHostPayLife {
                    self.lose(who, 1);
                }
                None
            }
            Extra::ExileOwnSpell => {
                let pos = self.stack.iter().rposition(|i| {
                    i.ctrl == who && matches!(i.src, Source::Spell { card, .. } if self.card(card).is_spell())
                });
                if let Some(pos) = pos {
                    let it = self.stack.remove(pos);
                    if let Source::Spell { card, .. } = it.src {
                        self.s[who].exile.push(card);
                    }
                    self.log(
                        Ev::Counter,
                        who as u8,
                        "log.exile",
                        vec![("card", it.name)],
                        vec![],
                    );
                }
                None
            }
            Extra::SwampAndForest => None,
            Extra::SacAnother | Extra::SacNontoken => {
                let Some(Target::Perm(id)) = pick else { return None };
                let (pw, tg) = self
                    .perm(id)
                    .map(|p| (self.power(p), self.toughness(p)))
                    .unwrap_or((0, 0));
                self.sacrifice(id);
                Some(if extra == Extra::SacNontoken { tg } else { pw })
            }
            Extra::Discard | Extra::DiscardCreature => {
                let Some(Target::Hand(i)) = pick else { return None };
                let mv = self.s[who]
                    .hand
                    .get(i)
                    .map(|&c| self.card(c).value as i32)
                    .unwrap_or(0);
                self.discard_at(who, i);
                Some(mv)
            }
            Extra::DiscardPair => {
                let Some(Target::Hand(i)) = pick else { return None };
                let &c = self.s[who].hand.get(i)?;
                let name = self.card(c).name.clone();
                self.discard_at(who, i);
                if let Some(j) = self.s[who].hand.iter().position(|&d| self.card(d).name == name) {
                    self.discard_at(who, j);
                }
                None
            }
        }
    }

    /// Jarad's way back: sacrifice a Swamp and a Forest, and he returns to your hand.
    pub(super) fn grave_ability(&mut self, who: usize, grave: usize, n: usize) {
        let Some(&c) = self.s[who].grave.get(grave) else {
            return;
        };
        let a = self.rules(c).activated[n].clone();
        if !self.pay(who, &a.cost, None) {
            return;
        }
        if a.extra == Extra::SwampAndForest {
            for kind in ["Swamp", "Forest"] {
                let land = self
                    .perms
                    .iter()
                    .filter(|p| {
                        p.ctrl == who
                            && p.card
                                .map(|k| self.card(k).subtypes.iter().any(|t| t == kind))
                                .unwrap_or(false)
                    })
                    .min_by_key(|p| p.id)
                    .map(|p| p.id);
                if let Some(id) = land {
                    self.sacrifice(id);
                }
            }
        }
        if let Some(i) = self.s[who].grave.iter().position(|&g| g == c) {
            self.s[who].grave.remove(i);
            self.s[who].hand.push(c);
            let name = self.card(c).name.clone();
            self.log(
                Ev::Ability,
                who as u8,
                "log.back_to_hand",
                vec![("card", name)],
                vec![],
            );
        }
        self.passes = 0;
    }

    // ---------- entering ----------

    /// As a permanent enters: a planeswalker's loyalty, counters by X, a name or colours chosen.
    pub(super) fn as_it_enters(&mut self, ctrl: usize, id: u32, c: usize) {
        let r = self.rules(c).clone();
        if let Some(l) = r.loyalty {
            self.set(id, |p| p.loyalty = l);
        }
        if r.x_counters && self.card(c).cost.xs() == 0 {
            // Cryptborn Horror: the life its controller's opponents lost this turn
            let n = self.s[other(ctrl)].lost.max(0);
            if n > 0 {
                self.add_counters(id, n);
            }
        }
        if r.choose_colours {
            let mut count = [0usize; 5];
            for &k in self.s[ctrl].library.iter().chain(self.s[ctrl].hand.iter()) {
                for (i, col) in mana::COLOURS.iter().enumerate() {
                    if self.card(k).colours & col != 0 {
                        count[i] += 1;
                    }
                }
            }
            let mut order: Vec<usize> = (0..5).collect();
            order.sort_by_key(|&i| (std::cmp::Reverse(count[i]), i));
            let pick = mana::COLOURS[order[0]] | mana::COLOURS[order[1]];
            self.set(id, |p| p.chosen_colours = pick);
        }
        if r.choose_name {
            let options = self.nameable(ctrl);
            if options.is_empty() {
                return;
            }
            if self.human[ctrl] {
                self.pending = Some(Pending {
                    who: ctrl,
                    ask: Ask::Name { options, perm: id },
                });
            } else {
                let n = crate::duel_ai::name_choice(self, ctrl, &options);
                self.named(ctrl, id, n);
            }
        }
    }

    /// The names Pithing Needle or Slaughter Games may name: the opponent's
    /// cards the chooser has seen (on the battlefield, in the graveyard), then any of the region's.
    pub(super) fn nameable(&self, who: usize) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let o = other(who);
        for p in self.perms.iter().filter(|p| p.ctrl == o) {
            if let Some(c) = p.card {
                let n = self.card(c).name.clone();
                if !self.card(c).is(ty::LAND) && !out.contains(&n) {
                    out.push(n);
                }
            }
        }
        for &c in &self.s[o].grave {
            let n = self.card(c).name.clone();
            if !self.card(c).is(ty::LAND) && !out.contains(&n) {
                out.push(n);
            }
        }
        out
    }

    pub(super) fn named(&mut self, who: usize, perm: u32, name: String) {
        if perm > 0 {
            let n = name.clone();
            self.set(perm, |p| p.named = n);
            self.log(
                Ev::Ability,
                who as u8,
                "log.named",
                vec![("card", name)],
                vec![perm],
            );
            return;
        }
        // Slaughter Games: from the opponent's hand, graveyard and library
        let o = other(who);
        let mut n = 0;
        for zone in 0..3 {
            let list = match zone {
                0 => &mut self.s[o].hand,
                1 => &mut self.s[o].grave,
                _ => &mut self.s[o].library,
            };
            let before = list.len();
            let cards = self.cards.clone();
            list.retain(|&c| cards[c].name != name);
            n += before - list.len();
        }
        let mut lib = std::mem::take(&mut self.s[o].library);
        self.shuffle(&mut lib);
        self.s[o].library = lib;
        self.log(
            Ev::Ability,
            who as u8,
            "log.slaughter",
            vec![("card", name), ("n", n.to_string())],
            vec![],
        );
    }

    /// Another creature has entered under Trostani's controller: gain its toughness.
    pub(super) fn ally_entered(&mut self, p: &Perm) {
        if !self.is_creature(p) {
            return;
        }
        let tough = self.toughness(p).max(0);
        let watchers: Vec<Perm> = self
            .perms
            .iter()
            .filter(|w| {
                w.ctrl == p.ctrl
                    && w.id != p.id
                    && w.card
                        .map(|c| self.rules(c).when.iter().any(|(x, _)| *x == When::AllyEnters))
                        .unwrap_or(false)
            })
            .cloned()
            .collect();
        for w in watchers {
            let t = self.trig_of(
                &w,
                Ability {
                    target: None,
                    effects: vec![Eff::Gain(tough)],
                },
                0,
            );
            self.triggers.push(t);
        }
    }

    /// A card named `name` was played: Search the City gives one of its
    /// exiled cards of that name back, and empty it gives an extra turn.
    pub(super) fn played_name(&mut self, who: usize, name: &str) {
        let cities: Vec<u32> = self
            .perms
            .iter()
            .filter(|p| {
                p.ctrl == who
                    && p.card
                        .map(|c| self.card(c).name == "Search the City")
                        .unwrap_or(false)
            })
            .map(|p| p.id)
            .collect();
        for city in cities {
            let Some(p) = self.perm(city).cloned() else {
                continue;
            };
            let Some(k) = p.held.iter().position(|(_, c)| self.card(*c).name == name) else {
                continue;
            };
            let (owner, c) = p.held[k];
            self.set(city, |q| {
                q.held.remove(k);
            });
            self.s[owner].hand.push(c);
            let n = self.card(c).name.clone();
            self.log(
                Ev::Ability,
                who as u8,
                "log.city_gives",
                vec![("card", n)],
                vec![city],
            );
            if self.perm(city).map(|q| q.held.is_empty()).unwrap_or(false) {
                self.sacrifice(city);
                self.s[who].extra_turns += 1;
                self.log(
                    Ev::Ability,
                    who as u8,
                    "log.extra_turn",
                    vec![("side", self.side_name(who))],
                    vec![],
                );
            }
        }
    }

    // ---------- combat ----------

    /// Whether an attack may be declared: each creature that must attack
    /// does, the attackers can attack, and Sphere of Safety's cost is met.
    pub(super) fn check_attack(
        &mut self,
        who: usize,
        legal: &[u32],
        ids: &[u32],
        aims: &[(u32, u32)],
    ) -> Result<(), String> {
        if ids.iter().any(|i| !legal.contains(i)) {
            return Err("That creature can't attack.".to_string());
        }
        for &id in legal {
            if self.perm(id).map(|p| self.must_attack(p)).unwrap_or(false) && !ids.contains(&id) {
                return Err("A creature that must attack is left out.".to_string());
            }
        }
        for (a, w) in aims {
            let ok = ids.contains(a)
                && self
                    .perm(*w)
                    .map(|p| p.ctrl != who && self.is_walker(p))
                    .unwrap_or(false);
            if !ok {
                return Err("That planeswalker can't be attacked.".to_string());
            }
        }
        let at_player = ids.iter().filter(|i| !aims.iter().any(|(a, _)| a == *i)).count() as i32;
        let tax = self.attack_tax(other(who)) * at_player;
        if tax > 0 {
            let cost = Cost {
                syms: vec![crate::mana::Sym::Generic(tax.min(255) as u8)],
            };
            if !self.pay(who, &cost, None) {
                return Err("Sphere of Safety's cost for those attackers can't be paid.".to_string());
            }
        }
        Ok(())
    }

    /// After attackers are declared: Jace's shield, Isperia, Utvara Hellkite.
    pub(super) fn attacks_declared(&mut self, ids: &[u32]) {
        let a = self.active;
        let d = other(a);
        if self.s[d].jace {
            for &id in ids {
                self.pump(id, -1, 0, 0);
            }
        }
        let mut trigs = Vec::new();
        for w in &self.perms {
            let Some(c) = w.card else { continue };
            for (when, ab) in &self.rules(c).when {
                match when {
                    When::Attacked if w.ctrl == d => {
                        for _ in ids {
                            trigs.push(self.trig_of(w, ab.clone(), 0));
                        }
                    }
                    When::DragonAttacks if w.ctrl == a => {
                        let dragons = ids
                            .iter()
                            .filter(|&&id| {
                                self.perm(id)
                                    .map(|p| self.has_subtype(p, "Dragon") || p.token == Some(Token::Dragon))
                                    .unwrap_or(false)
                            })
                            .count();
                        for _ in 0..dragons {
                            trigs.push(self.trig_of(w, ab.clone(), 0));
                        }
                    }
                    _ => {}
                }
            }
        }
        self.triggers.extend(trigs);
    }

    /// Golgari Decoy: after blocks, each creature able to block it that isn't blocking does.
    pub(super) fn lure_blocks(&mut self) {
        let lures: Vec<u32> = self
            .perms
            .iter()
            .filter(|p| {
                p.attacking
                    && p.card
                        .map(|c| self.rules(c).statics.contains(&Static::Lure))
                        .unwrap_or(false)
            })
            .map(|p| p.id)
            .collect();
        for lure in lures {
            let free: Vec<u32> = self
                .perms
                .iter()
                .filter(|b| b.blocking.is_none() && self.can_block(b, lure))
                .map(|b| b.id)
                .collect();
            for b in free {
                self.set(b, |p| p.blocking = Some(lure));
                self.set(lure, |p| p.blocked = true);
                let (bn, an) = (self.name_of(b), self.name_of(lure));
                self.log(
                    Ev::Block,
                    0,
                    "log.block",
                    vec![("blocker", bn), ("attacker", an)],
                    vec![b, lure],
                );
            }
        }
    }

    /// At the beginning of combat: Desecration Demon's offer.
    pub(super) fn combat_begins(&mut self) {
        let demons: Vec<Perm> = self
            .perms
            .iter()
            .filter(|p| {
                p.card
                    .map(|c| self.rules(c).when.iter().any(|(w, _)| *w == When::EachCombat))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        for d in demons {
            let t = self.trig_of(
                &d,
                Ability {
                    target: None,
                    effects: vec![Eff::DemonOffer],
                },
                0,
            );
            self.triggers.push(t);
        }
    }

    pub(super) fn demon_fed(&mut self, demon: u32) {
        self.set(demon, |p| p.tapped = true);
        self.add_counters(demon, 1);
    }

    /// At the beginning of the end step: Grave Betrayal's returns.
    pub(super) fn end_step_begins(&mut self) {
        for side in [P, F] {
            let back = std::mem::take(&mut self.s[side].betrayal);
            for (owner, c) in back {
                let Some(i) = self.s[owner].grave.iter().rposition(|&g| g == c) else {
                    continue;
                };
                self.s[owner].grave.remove(i);
                let id = self.make_perm(side, c);
                self.set(id, |p| {
                    p.owner = owner;
                    p.extra_colours |= mana::B;
                });
                self.add_counters(id, 1);
                let name = self.card(c).name.clone();
                self.log(
                    Ev::Enter,
                    side as u8,
                    "log.betrayal",
                    vec![("card", name), ("side", self.side_name(side))],
                    vec![id],
                );
                self.enter_triggers(id);
            }
        }
    }

    /// A creature died: Grave Betrayal's controller, if it was not theirs, takes it back at the end step.
    pub(super) fn enemy_died(&mut self, p: &Perm) {
        let Some(c) = p.card else { return };
        if p.token.is_some() {
            return;
        }
        for side in [P, F] {
            if side == p.ctrl {
                continue;
            }
            let n = self
                .perms
                .iter()
                .filter(|w| {
                    w.ctrl == side
                        && w.card
                            .map(|k| self.rules(k).when.iter().any(|(x, _)| *x == When::EnemyDies))
                            .unwrap_or(false)
                })
                .count();
            if n > 0 && self.s[p.owner].grave.contains(&c) {
                self.s[side].betrayal.push((p.owner, c));
            }
        }
    }

    /// After a creature is dealt damage: Volatile Rig flips a coin.
    pub(super) fn was_damaged(&mut self, id: u32) {
        let rig = self
            .perm(id)
            .and_then(|p| p.card)
            .map(|c| self.rules(c).when.iter().any(|(w, _)| *w == When::Damaged))
            .unwrap_or(false);
        if rig && self.rng.below(2) == 0 {
            let name = self.name_of(id);
            self.log(Ev::Ability, 0, "log.flip_lost", vec![("card", name)], vec![id]);
            self.sacrifice(id);
        }
    }

    /// Damage to a side: Azor's Elocutors lose a filibuster counter.
    pub(super) fn side_damaged(&mut self, side: usize) {
        let els: Vec<u32> = self
            .perms
            .iter()
            .filter(|p| {
                p.ctrl == side
                    && p.card
                        .map(|c| self.rules(c).when.iter().any(|(w, _)| *w == When::YouAreDamaged))
                        .unwrap_or(false)
            })
            .map(|p| p.id)
            .collect();
        for id in els {
            self.set(id, |p| p.charge = (p.charge - 1).max(0));
        }
    }

    /// Where damage to `side` or its permanent `to` goes: Palisade Giant takes it.
    pub(super) fn palisade(&self, side: usize, to: Option<u32>) -> Option<u32> {
        self.perms
            .iter()
            .find(|p| {
                p.ctrl == side
                    && Some(p.id) != to
                    && p.card
                        .map(|c| self.rules(c).statics.contains(&Static::TakesYourDamage))
                        .unwrap_or(false)
            })
            .map(|p| p.id)
    }

    /// Prevention for a side: Druid's Deliverance (combat damage), Security Blockade's shield.
    pub(super) fn prevented(&mut self, side: usize, n: i32, combat: bool) -> i32 {
        if combat && self.s[side].no_combat_damage {
            return 0;
        }
        let stop = self.s[side].shield.min(n).max(0);
        self.s[side].shield -= stop;
        n - stop
    }

    // ---------- free casting ----------

    /// Casts the next card waiting to be cast free (Epic Experiment, Jace).
    pub(super) fn next_free_cast(&mut self) -> bool {
        if self.free_casts.is_empty() || self.pending.is_some() {
            return false;
        }
        let (who, card) = self.free_casts.remove(0);
        let Some(i) = self.s[who].exile.iter().rposition(|&c| c == card) else {
            return true;
        };
        // a free cast with no legal target is not cast
        let ok = self.mode_castable(who, card, 0, false);
        if !ok {
            return true;
        }
        self.s[who].exile.remove(i);
        self.prepare(
            who,
            Then::Cast {
                hand: usize::MAX,
                card,
                mode: 0,
                overload: false,
                x: Some(0),
                pick: None,
                free: true,
            },
        );
        true
    }

    // ---------- Jace ----------

    pub(super) fn take_piles(&mut self, who: usize, take: Vec<usize>, rest: Vec<usize>) {
        let n = take.len();
        for c in take {
            self.s[who].hand.push(c);
        }
        for c in rest {
            self.s[who].library.insert(0, c);
        }
        self.log(
            Ev::Ability,
            who as u8,
            "log.piles",
            vec![("side", self.side_name(who)), ("n", n.to_string())],
            vec![],
        );
    }

    // ---------- the effects ----------

    /// Applies one of the effects the first region did not use. True when it waits on an answer.
    pub(super) fn effect_more(&mut self, item: &Item, e: &Eff) -> bool {
        let ctrl = item.ctrl;
        let src = Duel::source_perm(item);
        let x = item.x;
        let perm_targets: Vec<u32> = item
            .targets
            .iter()
            .filter_map(|t| match t {
                Target::Perm(id) => Some(*id),
                _ => None,
            })
            .filter(|id| self.perm(*id).is_some())
            .collect();
        let player = Duel::first_player(item);
        let hit = |d: &mut Duel, t: &Target, n: i32| match t {
            Target::Perm(id) => d.damage_perm(ctrl, src, &item.name, *id, n),
            Target::Player(pl) => d.damage_player(src, &item.name, ctrl, *pl, n),
            _ => {}
        };
        match e {
            Eff::DamageX | Eff::DamageSourcePower | Eff::DamageDiscarded => {
                for t in item.targets.clone() {
                    hit(self, &t, x);
                }
            }
            Eff::DamageSameName(n) => {
                for id in perm_targets {
                    let name = self.name_of(id);
                    let same: Vec<u32> = self
                        .perms
                        .iter()
                        .filter(|p| self.perm_name(p) == name)
                        .map(|p| p.id)
                        .collect();
                    for s in same {
                        self.damage_perm(ctrl, src, &item.name, s, *n);
                    }
                }
            }
            Eff::PumpSelfKw(k) => {
                if let Some(id) = src {
                    self.pump(id, 0, 0, *k);
                }
            }
            Eff::HostKw(k) => {
                if let Some(h) = src.and_then(|s| self.perm(s)).and_then(|p| p.attached) {
                    self.pump(h, 0, 0, *k);
                }
            }
            Eff::CountersX => {
                // Necropolis Regent: on the creature that dealt the damage (the trigger's fixed target)
                let ids = if perm_targets.is_empty() {
                    src.into_iter().collect()
                } else {
                    perm_targets
                };
                for id in ids {
                    self.add_counters(id, x);
                }
            }
            Eff::OozeX => {
                let p = self.blank(ctrl, None, Some(Token::Ooze));
                let id = p.id;
                self.perms.push(p);
                self.set(id, |q| q.size = x);
                self.log(
                    Ev::Token,
                    ctrl as u8,
                    "log.token",
                    vec![("side", self.side_name(ctrl)), ("token", format!("{x}/{x} Ooze"))],
                    vec![id],
                );
            }
            Eff::DrawX => self.draw(ctrl, x.max(0) as usize),
            Eff::GainX => self.gain(ctrl, x),
            Eff::TargetGains(n) => {
                if let Some(pl) = player {
                    self.gain(pl, *n);
                }
            }
            Eff::TargetDiscardsX => {
                if let Some(pl) = player {
                    return self.discard_or_ask(pl, x.max(0) as usize, ctrl);
                }
            }
            Eff::OpponentDiscardsRandom(n) => self.discard_random(other(ctrl), *n as usize, ctrl),
            Eff::ItsControllerLoses(n) => {
                if let Some(&pl) = item.tctrl.first() {
                    self.lose(pl, *n);
                }
            }
            Eff::TargetLoses(n) => {
                if let Some(pl) = player {
                    self.lose(pl, *n);
                }
            }
            Eff::CounterUnlessPaysX => {
                for t in &item.targets {
                    let Target::Item(i) = t else { continue };
                    let Some(victim) = self.stack.iter().find(|s| s.id == *i).map(|s| s.ctrl) else {
                        continue;
                    };
                    let cost = Cost {
                        syms: vec![crate::mana::Sym::Generic(x.clamp(0, 255) as u8)],
                    };
                    let pays = x == 0
                        || (self.can_pay(victim, &cost, None)
                            && (self.human[victim] || crate::duel_ai::pays(self, victim)));
                    if pays && x > 0 {
                        self.pay(victim, &cost, None);
                        let name = self
                            .stack
                            .iter()
                            .find(|s| s.id == *i)
                            .map(|s| s.name.clone())
                            .unwrap_or_default();
                        self.log(
                            Ev::Counter,
                            victim as u8,
                            "log.paid",
                            vec![("side", self.side_name(victim)), ("card", name)],
                            vec![],
                        );
                    } else if !pays {
                        // Syncopate: countered this way, it is exiled
                        if let Some(pos) = self.stack.iter().position(|s| s.id == *i) {
                            let it = self.stack.remove(pos);
                            if let Source::Spell { card, .. } = it.src {
                                self.s[it.ctrl].exile.push(card);
                            }
                            self.log(
                                Ev::Counter,
                                it.ctrl as u8,
                                "log.countered",
                                vec![("by", item.name.clone()), ("card", it.name)],
                                vec![],
                            );
                        }
                    }
                }
            }
            Eff::CounterBacklash => {
                for t in &item.targets {
                    let Target::Item(i) = t else { continue };
                    let Some(it) = self.stack.iter().find(|s| s.id == *i).cloned() else {
                        continue;
                    };
                    let power = match it.src {
                        Source::Spell { card, .. } => self.card(card).power,
                        _ => 0,
                    };
                    self.counter_item(*i, &item.name);
                    self.damage_player(None, &item.name, ctrl, it.ctrl, power);
                }
            }
            Eff::Rummage => self.ask_may(ctrl, May::Rummage),
            Eff::TargetToLibraryTop => {
                for id in perm_targets {
                    let Some(p) = self.perm(id).cloned() else { continue };
                    let name = self.perm_name(&p);
                    self.leave(id);
                    if let Some(c) = p.card {
                        self.s[p.owner].library.push(c);
                    }
                    self.log(
                        Ev::Dies,
                        p.ctrl as u8,
                        "log.to_library",
                        vec![("card", name)],
                        vec![id],
                    );
                }
            }
            Eff::RegenerateYours => {
                let mine: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|p| p.ctrl == ctrl && self.is_creature(p))
                    .map(|p| p.id)
                    .collect();
                for id in mine {
                    self.set(id, |p| p.regen += 1);
                }
            }
            Eff::DestroyAll(f) => {
                for id in self.each_of(f, ctrl) {
                    self.destroy(id);
                }
            }
            Eff::PumpAll(p, t) => {
                let all: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|q| self.is_creature(q))
                    .map(|q| q.id)
                    .collect();
                for id in all {
                    self.set(id, |q| q.mods.push((*p, *t, 0)));
                }
                self.log(
                    Ev::Ability,
                    ctrl as u8,
                    "log.pump_all",
                    vec![("p", signed(*p)), ("t", signed(*t))],
                    vec![],
                );
            }
            Eff::KeywordYours(k) => {
                let mine: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|q| q.ctrl == ctrl && self.is_creature(q))
                    .map(|q| q.id)
                    .collect();
                for id in mine {
                    let k = *k;
                    self.set(id, |q| q.mods.push((0, 0, k)));
                }
            }
            Eff::PumpOthersByPower => {
                let Some(s) = src.and_then(|s| self.perm(s)).cloned() else {
                    return false;
                };
                let n = self.power(&s).max(0);
                let mine: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|q| q.ctrl == ctrl && q.id != s.id && self.is_creature(q))
                    .map(|q| q.id)
                    .collect();
                for id in mine {
                    self.pump(id, n, n, 0);
                }
            }
            Eff::Nth(i, inner) => {
                if let Some(t) = item.targets.get(*i as usize) {
                    let one = Item {
                        targets: vec![*t],
                        ..item.clone()
                    };
                    return self.effect(&one, inner);
                }
            }
            Eff::Mill(n) => {
                if let Some(pl) = player {
                    self.mill(pl, *n as usize);
                }
            }
            Eff::MillX => {
                if let Some(pl) = player.or(Some(other(ctrl))) {
                    self.mill(pl, x.max(0) as usize);
                }
            }
            Eff::MillDefenders => {
                let n = self
                    .perms
                    .iter()
                    .filter(|p| p.ctrl == ctrl && self.is_creature(p) && self.has(p, kw::DEFENDER))
                    .count();
                if let Some(pl) = player {
                    self.mill(pl, n);
                }
            }
            Eff::SpiralMill => {
                let g = std::mem::take(&mut self.s[ctrl].grave);
                let n = g.len();
                self.s[ctrl].library.extend(g);
                let mut lib = std::mem::take(&mut self.s[ctrl].library);
                self.shuffle(&mut lib);
                self.s[ctrl].library = lib;
                if let Some(pl) = player {
                    self.mill(pl, n);
                }
            }
            Eff::MillUntilLand => {
                // a library holds no lands here (they live in the pile): it mills up to four
                if let Some(&pl) = item.tctrl.first() {
                    let mut n = 0;
                    while n < 4 {
                        let Some(&top) = self.s[pl].library.last() else {
                            break;
                        };
                        self.mill(pl, 1);
                        n += 1;
                        if self.card(top).is(ty::LAND) {
                            break;
                        }
                    }
                }
            }
            Eff::Fight => {
                if perm_targets.len() == 2 {
                    let (a, b) = (perm_targets[0], perm_targets[1]);
                    let pa = self.perm(a).map(|p| self.power(p)).unwrap_or(0);
                    let pb = self.perm(b).map(|p| self.power(p)).unwrap_or(0);
                    self.damage_perm(ctrl, Some(a), &self.name_of(a), b, pa);
                    self.damage_perm(ctrl, Some(b), &self.name_of(b), a, pb);
                }
            }
            Eff::BorrowUntilEnd => {
                for id in perm_targets {
                    let was = self.perm(id).map(|p| p.ctrl).unwrap_or(ctrl);
                    self.set(id, |p| {
                        if p.ctrl != ctrl {
                            p.borrowed = Some(was);
                            p.ctrl = ctrl;
                        }
                        p.tapped = false;
                        p.mods.push((2, 0, kw::HASTE));
                    });
                    let name = self.name_of(id);
                    self.log(
                        Ev::Ability,
                        ctrl as u8,
                        "log.borrow",
                        vec![("card", name), ("side", self.side_name(ctrl))],
                        vec![id],
                    );
                }
            }
            Eff::Exchange => {
                if let (Some(s), Some(&t)) = (src, perm_targets.first()) {
                    let (sc, tc) = (self.perm(s).map(|p| p.ctrl), self.perm(t).map(|p| p.ctrl));
                    if let (Some(sc), Some(tc)) = (sc, tc) {
                        self.set(s, |p| p.ctrl = tc);
                        self.set(t, |p| {
                            p.ctrl = sc;
                            p.fresh = true;
                        });
                        let name = self.name_of(t);
                        self.log(
                            Ev::Ability,
                            ctrl as u8,
                            "log.exchange",
                            vec![("card", name)],
                            vec![s, t],
                        );
                    }
                }
            }
            Eff::PreventCombatToYou => self.s[ctrl].no_combat_damage = true,
            Eff::Shield(n) => self.s[ctrl].shield += n,
            Eff::ExileGraveyard => {
                let who: Vec<usize> = match player {
                    Some(p) => vec![p],
                    None => vec![P, F],
                };
                for p in who {
                    let g = std::mem::take(&mut self.s[p].grave);
                    self.s[p].exile.extend(g);
                    self.log(
                        Ev::Ability,
                        ctrl as u8,
                        "log.grave_exiled",
                        vec![("side", self.side_name(p))],
                        vec![],
                    );
                }
            }
            Eff::ExileCard => {
                for t in &item.targets {
                    if let Target::Grave(o, c) = t {
                        if let Some(i) = self.s[*o].grave.iter().position(|g| g == c) {
                            self.s[*o].grave.remove(i);
                            self.s[*o].exile.push(*c);
                            let name = self.card(*c).name.clone();
                            self.log(Ev::Ability, ctrl as u8, "log.exile", vec![("card", name)], vec![]);
                        }
                    }
                }
            }
            Eff::CardToHand => {
                for t in &item.targets {
                    if let Target::Grave(o, c) = t {
                        if let Some(i) = self.s[*o].grave.iter().position(|g| g == c) {
                            self.s[*o].grave.remove(i);
                            self.s[*o].hand.push(*c);
                            let name = self.card(*c).name.clone();
                            self.log(
                                Ev::Ability,
                                ctrl as u8,
                                "log.back_to_hand",
                                vec![("card", name)],
                                vec![],
                            );
                        }
                    }
                }
            }
            Eff::CreaturesHitControllers => {
                let all: Vec<(u32, usize)> = self
                    .perms
                    .iter()
                    .filter(|p| self.is_creature(p))
                    .map(|p| (p.id, p.ctrl))
                    .collect();
                for (id, c) in all {
                    let name = self.name_of(id);
                    self.damage_player(Some(id), &name, c, c, 1);
                }
            }
            Eff::GainPerCardInHand => {
                if let Some(pl) = player {
                    let n = self.s[pl].hand.len() as i32;
                    let names: Vec<String> = self.s[pl]
                        .hand
                        .iter()
                        .map(|&c| self.card(c).name.clone())
                        .collect();
                    self.log(
                        Ev::Ability,
                        ctrl as u8,
                        "log.reveal_hand",
                        vec![("side", self.side_name(pl)), ("cards", names.join(", "))],
                        vec![],
                    );
                    self.gain(ctrl, n);
                }
            }
            Eff::CopySpell => {
                for t in &item.targets {
                    let Target::Item(i) = t else { continue };
                    let Some(orig) = self.stack.iter().find(|s| s.id == *i).cloned() else {
                        continue;
                    };
                    let Source::Spell { card, .. } = orig.src else {
                        continue;
                    };
                    let id = self.id();
                    let name = orig.name.clone();
                    self.stack.push(Item {
                        id,
                        ctrl,
                        src: Source::Copy { card },
                        ..orig
                    });
                    self.log(Ev::Cast, ctrl as u8, "log.copy", vec![("card", name)], vec![]);
                }
            }
            Eff::PileBasics(n) => {
                let lands = crate::encounter::basics_needed(
                    &self.cards,
                    &self.s[ctrl].hand,
                    &self.s[ctrl].library,
                    *n as usize,
                );
                let k = lands.len();
                self.s[ctrl].pile.extend(lands);
                self.log(
                    Ev::Ability,
                    ctrl as u8,
                    "log.pile_basics",
                    vec![("side", self.side_name(ctrl)), ("n", k.to_string())],
                    vec![],
                );
            }
            Eff::Salvage => {
                let n = self.s[ctrl].library.len().min(5);
                let top: Vec<usize> = self.s[ctrl].library.split_off(self.s[ctrl].library.len() - n);
                let best = top
                    .iter()
                    .copied()
                    .filter(|&c| self.card(c).is(ty::CREATURE))
                    .max_by_key(|&c| (self.card(c).value, c));
                for c in top {
                    if Some(c) == best {
                        self.s[ctrl].hand.push(c);
                    } else {
                        self.put_in_grave(ctrl, c);
                    }
                }
                self.log(
                    Ev::Ability,
                    ctrl as u8,
                    "log.salvage",
                    vec![("side", self.side_name(ctrl))],
                    vec![],
                );
            }
            Eff::TwoCreatures => {
                let mut found: Vec<usize> = self.s[ctrl]
                    .library
                    .iter()
                    .copied()
                    .filter(|&c| self.card(c).is(ty::CREATURE))
                    .collect();
                found.sort_by_key(|&c| (std::cmp::Reverse(self.card(c).value), c));
                found.dedup();
                for (k, c) in found.into_iter().take(2).enumerate() {
                    if let Some(i) = self.s[ctrl].library.iter().position(|&l| l == c) {
                        self.s[ctrl].library.remove(i);
                        if k == 0 {
                            self.s[ctrl].hand.push(c);
                        } else {
                            self.put_in_grave(ctrl, c);
                        }
                    }
                }
                let mut lib = std::mem::take(&mut self.s[ctrl].library);
                self.shuffle(&mut lib);
                self.s[ctrl].library = lib;
            }
            Eff::Foresight => {
                for mv in [3, 2, 1] {
                    if let Some(i) = self.s[ctrl]
                        .library
                        .iter()
                        .position(|&c| self.card(c).is(ty::INSTANT) && self.card(c).value == mv)
                    {
                        let c = self.s[ctrl].library.remove(i);
                        self.s[ctrl].hand.push(c);
                    }
                }
                let mut lib = std::mem::take(&mut self.s[ctrl].library);
                self.shuffle(&mut lib);
                self.s[ctrl].library = lib;
            }
            Eff::Experiment => {
                let n = self.s[ctrl].library.len().min(x.max(0) as usize);
                let top: Vec<usize> = self.s[ctrl].library.split_off(self.s[ctrl].library.len() - n);
                for c in top {
                    self.s[ctrl].exile.push(c);
                    if self.card(c).is_spell() && self.card(c).value as i32 <= x {
                        self.free_casts.push((ctrl, c));
                    } else if let Some(i) = self.s[ctrl].exile.iter().rposition(|&e| e == c) {
                        self.s[ctrl].exile.remove(i);
                        self.put_in_grave(ctrl, c);
                    }
                }
            }
            Eff::JaceShield => self.s[ctrl].jace = true,
            Eff::JacePiles => {
                let n = self.s[ctrl].library.len().min(3);
                let top: Vec<usize> = self.s[ctrl].library.split_off(self.s[ctrl].library.len() - n);
                // the opponent splits them: the best card alone, the rest together
                let (a, b) = crate::duel_ai::split_piles(self, &top);
                if self.human[ctrl] {
                    self.pending = Some(Pending {
                        who: ctrl,
                        ask: Ask::Piles { a, b },
                    });
                    return true;
                }
                let take_a = crate::duel_ai::pile_worth(self, &a) >= crate::duel_ai::pile_worth(self, &b);
                if take_a {
                    self.take_piles(ctrl, a, b);
                } else {
                    self.take_piles(ctrl, b, a);
                }
            }
            Eff::JaceUltimate => {
                for side in [P, F] {
                    let best = self.s[side]
                        .library
                        .iter()
                        .copied()
                        .filter(|&c| !self.card(c).is(ty::LAND))
                        .max_by_key(|&c| (self.card(c).value, c));
                    if let Some(c) = best {
                        if let Some(i) = self.s[side].library.iter().position(|&l| l == c) {
                            self.s[side].library.remove(i);
                            self.s[ctrl].exile.push(c);
                            self.free_casts.push((ctrl, c));
                        }
                    }
                    let mut lib = std::mem::take(&mut self.s[side].library);
                    self.shuffle(&mut lib);
                    self.s[side].library = lib;
                }
            }
            Eff::VraskaWard => {
                if let Some(s) = src {
                    self.set(s, |p| p.ward = true);
                }
            }
            Eff::FlipForLife => {
                let won = self.rng.below(2) == 1;
                self.log(
                    Ev::Ability,
                    ctrl as u8,
                    if won { "log.flip_won" } else { "log.flip_lost_life" },
                    vec![("side", self.side_name(ctrl))],
                    vec![],
                );
                if won {
                    self.gain(ctrl, 6);
                }
            }
            Eff::RigBlast => {
                if self.rng.below(2) == 0 {
                    self.log(Ev::Ability, ctrl as u8, "log.rig_blast", vec![], vec![]);
                    let all: Vec<u32> = self
                        .perms
                        .iter()
                        .filter(|p| self.is_creature(p))
                        .map(|p| p.id)
                        .collect();
                    for id in all {
                        self.damage_perm(ctrl, None, &item.name, id, 4);
                    }
                    for p in [P, F] {
                        self.damage_player(None, &item.name, ctrl, p, 4);
                    }
                }
            }
            Eff::SlaughterGames => {
                let options = {
                    let mut o = self.nameable(ctrl);
                    if o.is_empty() {
                        let mut all: Vec<String> = self
                            .cards
                            .iter()
                            .filter(|c| !c.is(ty::LAND))
                            .map(|c| c.name.clone())
                            .collect();
                        all.dedup();
                        o = all;
                    }
                    o
                };
                if self.human[ctrl] {
                    self.pending = Some(Pending {
                        who: ctrl,
                        ask: Ask::Name { options, perm: 0 },
                    });
                    return true;
                }
                let n = crate::duel_ai::name_choice(self, ctrl, &options);
                self.named(ctrl, 0, n);
            }
            Eff::SearchCity => {
                if let Some(s) = src {
                    let n = self.s[ctrl].library.len().min(5);
                    let top: Vec<usize> = self.s[ctrl].library.split_off(self.s[ctrl].library.len() - n);
                    let owner = ctrl;
                    self.set(s, |p| p.held.extend(top.iter().map(|&c| (owner, c))));
                    self.log(
                        Ev::Ability,
                        ctrl as u8,
                        "log.city_exiles",
                        vec![("n", n.to_string())],
                        vec![s],
                    );
                }
            }
            Eff::GuildFeud => {
                let mut put: Vec<u32> = Vec::new();
                for side in [other(ctrl), ctrl] {
                    let n = self.s[side].library.len().min(3);
                    let top: Vec<usize> = self.s[side].library.split_off(self.s[side].library.len() - n);
                    let best = top
                        .iter()
                        .copied()
                        .filter(|&c| self.card(c).is(ty::CREATURE))
                        .max_by_key(|&c| (self.card(c).value, c));
                    for c in top {
                        if Some(c) == best {
                            let id = self.make_perm(side, c);
                            self.entering(side, id);
                            put.push(id);
                        } else {
                            self.put_in_grave(side, c);
                        }
                    }
                }
                if put.len() == 2 {
                    let one = Item {
                        targets: put.iter().map(|&p| Target::Perm(p)).collect(),
                        ..item.clone()
                    };
                    self.effect_more(&one, &Eff::Fight);
                }
            }
            Eff::ExtraTurn => self.s[ctrl].extra_turns += 1,
            Eff::LoseHalf => {
                if let Some(pl) = player {
                    let n = (self.s[pl].life.max(0) + 1) / 2;
                    self.lose(pl, n);
                }
            }
            Eff::TitheCheck => {
                if let Some(&id) = perm_targets.first() {
                    let owner = self.perm(id).map(|p| p.ctrl).unwrap_or(ctrl);
                    self.ask_may(owner, May::Tithe(id, x.clamp(0, 255) as u8));
                    return self.pending.is_some();
                }
            }
            Eff::Filibuster => {
                if let Some(s) = src {
                    self.set(s, |p| p.charge += 1);
                    if self.perm(s).map(|p| p.charge >= 5).unwrap_or(false) {
                        self.outcome = Some(if ctrl == P { Outcome::Won } else { Outcome::Lost });
                        self.log(
                            Ev::End,
                            ctrl as u8,
                            "log.filibuster",
                            vec![("side", self.side_name(ctrl))],
                            vec![s],
                        );
                    }
                }
            }
            Eff::BloomCheck => {
                if let Some(s) = src {
                    if self.perm(s).map(|p| p.charge <= 0).unwrap_or(false) {
                        self.bounce(s);
                    }
                }
            }
            Eff::DemonOffer => {
                let Some(demon) = src.filter(|s| self.perm(*s).is_some()) else {
                    return false;
                };
                let opp = other(ctrl);
                let legal: Vec<u32> = self
                    .perms
                    .iter()
                    .filter(|p| p.ctrl == opp && self.is_creature(p))
                    .map(|p| p.id)
                    .collect();
                if legal.is_empty() {
                    return false;
                }
                if self.human[opp] {
                    self.pending = Some(Pending {
                        who: opp,
                        ask: Ask::Offer { legal, demon },
                    });
                    return true;
                }
                if let Some(id) = crate::duel_ai::demon_offer(self, opp, &legal, demon) {
                    self.sacrifice(id);
                    self.demon_fed(demon);
                }
            }
            Eff::CopySelf => {
                if let Some(s) = src.and_then(|s| self.perm(s)).cloned() {
                    let p = self.blank(ctrl, s.card, Some(Token::Copy));
                    let id = p.id;
                    self.perms.push(p);
                    self.log(
                        Ev::Token,
                        ctrl as u8,
                        "log.token",
                        vec![("side", self.side_name(ctrl)), ("token", self.perm_name(&s))],
                        vec![id],
                    );
                }
            }
            Eff::LoseSacrificedPower => self.lose(other(ctrl), x),
            Eff::SaprolingsByToughness => self.make_token(ctrl, Token::Saproling, x.clamp(0, 20) as u8),
            Eff::Draw4 => self.draw(ctrl, 4),
            Eff::FromGraveToHand => {}
            Eff::PayToDiscard => {
                if let Some(pl) = player {
                    self.ask_may(ctrl, May::Vermin(pl));
                    return self.pending.is_some();
                }
            }
            Eff::RemoveCharge => {
                if let Some(s) = src {
                    self.set(s, |p| p.charge -= 1);
                }
            }
            Eff::MustBeBlocked | Eff::MustAttack => {
                for id in perm_targets {
                    self.pump(id, 0, 0, kw::MUST_ATTACK);
                }
            }
            _ => {}
        }
        false
    }
}
