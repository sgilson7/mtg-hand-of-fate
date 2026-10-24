//! The computer's side of a game of Magic: a fixed set of habits, no
//! search and no chance, so a game replays the same from its seed. It plays
//! a land each turn, casts what it can from its biggest card down, holds its
//! tricks and counterspells for the moment they matter, attacks when the
//! attack is safe or the race is won, and blocks to trade up or to live.
//!
//! The same habits play the player's side in the balance tests.

use crate::content::ty;
use crate::duel::{Action, Answer, Ask, Duel, Perm, Source, Step, Target, Then, P};
use crate::rules::{kw, Aura, Eff, Tgt};

fn other(side: usize) -> usize {
    1 - side
}

/// The computer's answer to what is pending for `who`.
pub fn answer(d: &Duel, who: usize) -> Answer {
    let Some(p) = &d.pending else {
        return Answer::Act(Action::Pass);
    };
    match &p.ask {
        Ask::Priority => Answer::Act(priority(d, who)),
        Ask::Targets {
            legal,
            min,
            max,
            then,
            ..
        } => Answer::Targets(choose_targets(d, who, then, legal, *min, *max)),
        Ask::Attackers { legal, .. } => Answer::Attack(attackers(d, who, legal)),
        Ask::Blockers { legal, attackers, .. } => Answer::Block(blockers(d, who, legal, attackers)),
        Ask::Unleash { .. } => Answer::Yes(unleash(d, who)),
        Ask::Discard { .. } => Answer::Pick(Some(discard_choice(d, who))),
        Ask::Sacrifice { legal } => {
            let i = (0..legal.len())
                .min_by_key(|&i| (d.value_of(legal[i]), i))
                .unwrap_or(0);
            Answer::Pick(Some(i))
        }
        Ask::Impostor { legal, .. } => {
            // return the cheapest, best of all one with an enter ability to use again
            let i = (0..legal.len()).max_by_key(|&i| {
                let p = d.perm(legal[i]);
                let etb = p
                    .and_then(|p| p.card)
                    .map(|c| d.card(c).rules.etb.is_some())
                    .unwrap_or(false);
                (etb, -d.value_of(legal[i]))
            });
            Answer::Pick(i)
        }
        Ask::Pay { .. } => Answer::Yes(true),
    }
}

/// The card to discard: the one worth least.
pub fn discard_choice(d: &Duel, who: usize) -> usize {
    let hand = &d.s[who].hand;
    (0..hand.len())
        .min_by_key(|&i| {
            let c = d.card(hand[i]);
            let castable = c.value <= d.mana_in_play(who) + d.s[who].pile.len().min(2);
            (castable, c.value, i)
        })
        .unwrap_or(0)
}

fn unleash(d: &Duel, who: usize) -> bool {
    let their_power: i32 = d
        .perms
        .iter()
        .filter(|p| p.ctrl != who && d.is_creature(p))
        .map(|p| d.power(p))
        .sum();
    d.s[who].life > their_power * 2 || their_power == 0
}

// ---------- targets ----------

/// How much `who` gains from an effect landing on `t`: above 0 is good.
fn target_score(d: &Duel, who: usize, effects: &[Eff], aura: Option<Aura>, t: &Target) -> i32 {
    let harmful = aura.map(|a| a != Aura::EtherealArmor).unwrap_or(false)
        || effects.iter().any(|e| {
            matches!(
                e,
                Eff::Damage(_)
                    | Eff::Destroy
                    | Eff::Exile
                    | Eff::Bounce
                    | Eff::Detain
                    | Eff::Tap
                    | Eff::Counter
                    | Eff::CounterUnlessPays(_)
                    | Eff::TargetDiscards(_)
                    | Eff::ExileHeld
                    | Eff::ExileHeldSameName
                    | Eff::ItsControllerDiscards(_)
            ) || matches!(e, Eff::Pump(p, _, _) if *p < 0)
        });
    let damage: i32 = effects
        .iter()
        .map(|e| match e {
            Eff::Damage(n) => *n,
            _ => 0,
        })
        .sum();
    match t {
        Target::Perm(id) => {
            let Some(p) = d.perm(*id) else { return 0 };
            let v = d.value_of(*id).max(1);
            let mine = p.ctrl == who;
            if harmful {
                if mine {
                    return -v;
                }
                if damage > 0 && d.is_creature(p) {
                    let left = d.toughness(p) - p.damage;
                    return if damage >= left { v * 2 } else { 0 };
                }
                if aura == Some(Aura::StabWound) {
                    return if d.toughness(p) <= 2 { v * 2 } else { v };
                }
                if effects.contains(&Eff::Tap) && p.tapped {
                    return 0;
                }
                v
            } else if mine {
                // a boost: on something that can use it
                if effects.contains(&Eff::Untap) && !p.tapped {
                    return v / 2;
                }
                v
            } else {
                -v
            }
        }
        Target::Player(pl) => {
            let opp = *pl != who;
            if effects.iter().any(|e| matches!(e, Eff::TargetDraws(_))) {
                return if opp { -10 } else { 10 };
            }
            if effects.iter().any(|e| matches!(e, Eff::TargetDiscards(_))) {
                return if opp { d.s[*pl].hand.len() as i32 * 4 } else { -10 };
            }
            if damage > 0 {
                if !opp {
                    return -20;
                }
                let lethal = d.s[*pl].life <= damage;
                return if lethal { 1000 } else { damage * 2 - 1 };
            }
            0
        }
        Target::Item(i) => {
            let Some(it) = d.stack.iter().find(|x| x.id == *i) else {
                return 0;
            };
            if it.ctrl == who {
                return -50;
            }
            match it.src {
                Source::Spell { card, .. } => {
                    let c = d.card(card);
                    if c.rules.kw & kw::CANT_BE_COUNTERED != 0 {
                        return 0;
                    }
                    c.value as i32 * 3 + 1
                }
                _ => 0,
            }
        }
        Target::Grave(owner, c) => {
            let v = d.card(*c).value as i32;
            if *owner == who {
                v
            } else {
                1
            }
        }
    }
}

fn then_effects(d: &Duel, then: &Then) -> (Vec<Eff>, Option<Aura>) {
    match then {
        Then::Cast { card, mode, .. } => {
            let r = &d.card(*card).rules;
            (
                r.modes.get(*mode).map(|m| m.effects.clone()).unwrap_or_default(),
                r.aura,
            )
        }
        Then::Activate { perm, n } => {
            let effects = d
                .perm(*perm)
                .and_then(|p| p.card)
                .and_then(|c| {
                    d.card(c)
                        .rules
                        .activated
                        .get(*n)
                        .map(|a| a.ability.effects.clone())
                })
                .unwrap_or_default();
            (effects, None)
        }
        Then::Scavenge { .. } => (vec![Eff::Counters(1)], None),
        Then::Trigger(t) => (t.ability.effects.clone(), None),
    }
}

fn choose_targets(
    d: &Duel,
    who: usize,
    then: &Then,
    legal: &[Target],
    min: usize,
    max: usize,
) -> Vec<Target> {
    let (effects, aura) = then_effects(d, then);
    ranked(d, who, &effects, aura, legal, min, max)
}

fn ranked(
    d: &Duel,
    who: usize,
    effects: &[Eff],
    aura: Option<Aura>,
    legal: &[Target],
    min: usize,
    max: usize,
) -> Vec<Target> {
    let mut scored: Vec<(i32, usize)> = legal
        .iter()
        .enumerate()
        .map(|(i, t)| (target_score(d, who, effects, aura, t), i))
        .collect();
    scored.sort_by_key(|(s, i)| (std::cmp::Reverse(*s), *i));
    let mut out = Vec::new();
    for (s, i) in scored {
        if out.len() >= max {
            break;
        }
        if s > 0 || out.len() < min {
            out.push(legal[i]);
        }
    }
    out
}

/// The best targets' total worth for a spell or ability, or None when it
/// has nothing worth aiming at.
fn aim_worth(
    d: &Duel,
    who: usize,
    spec: Option<Tgt>,
    effects: &[Eff],
    aura: Option<Aura>,
    source: Option<u32>,
) -> Option<i32> {
    let Some(spec) = spec else { return Some(0) };
    let legal = d.legal_targets(who, &spec, source);
    let max = match spec {
        Tgt::UpTo(n, _) => n as usize,
        Tgt::Angel => 3,
        _ => 1,
    };
    let picks = ranked(d, who, effects, aura, &legal, 0, max);
    let total: i32 = picks.iter().map(|t| target_score(d, who, effects, aura, t)).sum();
    if total > 0 {
        Some(total)
    } else {
        None
    }
}

// ---------- priority ----------

fn is_trick(effects: &[Eff]) -> bool {
    effects
        .iter()
        .any(|e| matches!(e, Eff::Pump(p, _, _) if *p > 0) || matches!(e, Eff::Chorus | Eff::Untap))
        && !effects.iter().any(|e| matches!(e, Eff::Token(_, _)))
}

fn is_counter(effects: &[Eff]) -> bool {
    effects
        .iter()
        .any(|e| matches!(e, Eff::Counter | Eff::CounterUnlessPays(_)))
}

/// What casting this is worth in a main phase, or None to hold it.
fn cast_worth(d: &Duel, who: usize, card: usize, mode: usize, overload: bool) -> Option<i32> {
    let c = d.card(card);
    let r = &c.rules;
    let base = c.value as i32 * 3;
    if c.is(ty::CREATURE) {
        return Some(10 + base);
    }
    if r.aura.is_some() {
        let spec = r.modes.first().and_then(|m| m.target);
        return aim_worth(d, who, spec, &[], r.aura, None).map(|w| w + base);
    }
    if !c.is_spell() {
        // an enchantment or artifact with no target
        return Some(6 + base);
    }
    let m = r.modes.get(mode)?;
    if is_counter(&m.effects) || is_trick(&m.effects) {
        return None;
    }
    if m.effects.iter().any(|e| matches!(e, Eff::Loot(_, _))) {
        return None;
    }
    if m.effects
        .iter()
        .any(|e| matches!(e, Eff::Pump(p, _, _) if *p < 0))
        && d.active == who
    {
        return None;
    }
    if overload {
        let f = match m.target {
            Some(Tgt::One(f)) => f,
            _ => return None,
        };
        let hit: Vec<u32> = d
            .perms
            .iter()
            .filter(|p| p.ctrl != who && d.is_creature(p) == f.creature && !d.is_land(p))
            .map(|p| p.id)
            .collect();
        if hit.len() < 2 {
            return None;
        }
        let worth: i32 = hit
            .iter()
            .map(|&id| target_score(d, who, &m.effects, None, &Target::Perm(id)))
            .filter(|s| *s > 0)
            .sum();
        return if worth > 0 { Some(worth + base + 5) } else { None };
    }
    if m.effects.iter().any(|e| matches!(e, Eff::Tap)) && d.active != who {
        return None;
    }
    if m.effects.iter().any(|e| matches!(e, Eff::Tap | Eff::Detain)) && d.step != Step::Main1 {
        return None;
    }
    let w = aim_worth(d, who, m.target, &m.effects, None, None)?;
    Some(w + base)
}

fn sorcery_time(d: &Duel, who: usize) -> bool {
    d.active == who && matches!(d.step, Step::Main1 | Step::Main2) && d.stack.is_empty()
}

fn priority(d: &Duel, who: usize) -> Action {
    let acts = d.actions(who);
    if acts.len() <= 1 {
        return Action::Pass;
    }
    if let Some(top) = d.stack.last() {
        if top.ctrl != who {
            if let Some(a) = respond(d, who, &acts) {
                return a;
            }
        }
        return Action::Pass;
    }
    if sorcery_time(d, who) {
        if acts.iter().any(|a| matches!(a, Action::Land(_))) {
            let best = d.best_pile_land(who);
            let want = d.s[who].pile[best];
            if let Some(i) = d.s[who].pile.iter().position(|&c| c == want) {
                if acts.contains(&Action::Land(i)) {
                    return Action::Land(i);
                }
            }
        }
        if let Some(a) = best_cast(d, who, &acts) {
            return a;
        }
        if let Some(a) = main_ability(d, who, &acts) {
            return a;
        }
        return Action::Pass;
    }
    if d.step == Step::Blockers {
        if let Some(a) = combat_trick(d, who, &acts) {
            return a;
        }
    }
    if d.step == Step::End && d.active != who {
        if let Some(a) = end_step(d, who, &acts) {
            return a;
        }
    }
    Action::Pass
}

fn best_cast(d: &Duel, who: usize, acts: &[Action]) -> Option<Action> {
    let mut best: Option<(i32, Action)> = None;
    for a in acts {
        let Action::Cast { hand, mode, overload } = *a else {
            continue;
        };
        let card = d.s[who].hand[hand];
        if let Some(w) = cast_worth(d, who, card, mode, overload) {
            if best.map(|(b, _)| w > b).unwrap_or(true) {
                best = Some((w, *a));
            }
        }
    }
    best.map(|(_, a)| a)
}

fn ability_effects(d: &Duel, perm: u32, n: usize) -> (Vec<Eff>, Option<Tgt>, crate::rules::Use) {
    let a = d
        .perm(perm)
        .and_then(|p| p.card)
        .and_then(|c| d.card(c).rules.activated.get(n).cloned());
    match a {
        Some(a) => (a.ability.effects, a.ability.target, a.when),
        None => (Vec::new(), None, crate::rules::Use::Pump),
    }
}

fn main_ability(d: &Duel, who: usize, acts: &[Action]) -> Option<Action> {
    use crate::rules::Use;
    for a in acts {
        match *a {
            Action::Scavenge { .. } => {
                let best = d
                    .perms
                    .iter()
                    .filter(|p| p.ctrl == who && d.is_creature(p))
                    .max_by_key(|p| d.value_of(p.id));
                if best.is_some() {
                    return Some(*a);
                }
            }
            Action::Activate { perm, n } => {
                let (effects, spec, when) = ability_effects(d, perm, n);
                match when {
                    Use::Equip => {
                        let attached = d.perm(perm).and_then(|p| p.attached).is_some();
                        if !attached && aim_worth(d, who, spec, &[Eff::Attach], None, Some(perm)).is_some() {
                            return Some(*a);
                        }
                    }
                    Use::SacForToken if d.step == Step::Main2 => return Some(*a),
                    Use::Ping if d.step == Step::Main2 => {
                        if spec.is_none() || aim_worth(d, who, spec, &effects, None, Some(perm)).is_some() {
                            return Some(*a);
                        }
                    }
                    Use::Animate if d.step == Step::Main1 => {
                        let fresh = d.perm(perm).map(|p| p.fresh).unwrap_or(true);
                        let blockers = d
                            .perms
                            .iter()
                            .any(|p| p.ctrl != who && d.is_creature(p) && !p.tapped);
                        // paid from other mana, so the keyrune stays untapped to attack
                        let cost = d
                            .perm(perm)
                            .and_then(|p| p.card)
                            .and_then(|c| d.card(c).rules.activated.get(n).map(|x| x.cost.clone()));
                        let others = cost.map(|c| d.can_pay(who, &c, Some(perm))).unwrap_or(false);
                        if !fresh && !blockers && others {
                            return Some(*a);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    None
}

/// The damage a creature in combat takes from what it fights.
fn incoming(d: &Duel, p: &Perm) -> (i32, bool) {
    let mut dmg = 0;
    let mut deathtouch = false;
    for q in &d.perms {
        let fights = (p.attacking && q.blocking == Some(p.id)) || (p.blocking == Some(q.id));
        if fights {
            dmg += d.power(q);
            deathtouch |= d.kws(q) & kw::DEATHTOUCH != 0;
        }
    }
    (dmg, deathtouch)
}

fn would_die(d: &Duel, p: &Perm) -> bool {
    let (dmg, dt) = incoming(d, p);
    (dt && dmg > 0) || dmg >= d.toughness(p) - p.damage
}

fn combat_trick(d: &Duel, who: usize, acts: &[Action]) -> Option<Action> {
    use crate::rules::Use;
    let mine: Vec<&Perm> = d
        .perms
        .iter()
        .filter(|p| p.ctrl == who && (p.attacking || p.blocking.is_some()))
        .collect();
    for p in &mine {
        if !would_die(d, p) {
            continue;
        }
        let (dmg, dt) = incoming(d, p);
        for a in acts {
            match *a {
                Action::Activate { perm, n } if perm == p.id => {
                    let (effects, _, when) = ability_effects(d, perm, n);
                    if when == Use::Regenerate && p.regen == 0 {
                        return Some(*a);
                    }
                    if let Some(Eff::PumpSelf(_, t)) = effects.first() {
                        if !dt && *t > 0 && d.toughness(p) - p.damage + t > dmg {
                            return Some(*a);
                        }
                    }
                }
                Action::Cast {
                    hand,
                    mode,
                    overload: false,
                } => {
                    let card = d.s[who].hand[hand];
                    let m = d.card(card).rules.modes.get(mode);
                    if let Some(m) = m {
                        let t = m.effects.iter().find_map(|e| match e {
                            Eff::Pump(_, t, _) if *t > 0 => Some(*t),
                            Eff::Chorus => Some(
                                d.perms
                                    .iter()
                                    .filter(|q| q.ctrl == who && d.is_creature(q))
                                    .count() as i32,
                            ),
                            _ => None,
                        });
                        if let Some(t) = t {
                            if !dt && d.toughness(p) - p.damage + t > dmg {
                                return Some(*a);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // unblocked attackers: spend spare pumps on the face
    if d.active == who {
        for a in acts {
            if let Action::Activate { perm, n } = *a {
                let (effects, _, when) = ability_effects(d, perm, n);
                let unblocked = d.perm(perm).map(|p| p.attacking && !p.blocked).unwrap_or(false);
                if when == Use::Pump
                    && unblocked
                    && matches!(effects.first(), Some(Eff::PumpSelf(x, t)) if *x > 0 && d.perm(perm).map(|p| d.toughness(p) + t > 0).unwrap_or(false))
                {
                    return Some(*a);
                }
            }
        }
    } else {
        // shrink or tap the biggest unblocked attacker
        for a in acts {
            if let Action::Cast {
                hand,
                mode,
                overload: false,
            } = *a
            {
                let card = d.s[who].hand[hand];
                let m = &d.card(card).rules.modes[mode.min(d.card(card).rules.modes.len().saturating_sub(1))];
                if m.effects
                    .iter()
                    .any(|e| matches!(e, Eff::Pump(p, _, _) if *p < 0))
                {
                    let big = d
                        .perms
                        .iter()
                        .any(|p| p.attacking && !p.blocked && d.power(p) >= 3);
                    if big {
                        return Some(*a);
                    }
                }
            }
        }
    }
    None
}

fn respond(d: &Duel, who: usize, acts: &[Action]) -> Option<Action> {
    let top = d.stack.last()?;
    // a counterspell on a spell worth stopping
    if let Source::Spell { card, .. } = top.src {
        let c = d.card(card);
        let worth = c.value >= 3
            || top.targets.iter().any(
                |t| matches!(t, Target::Perm(id) if d.perm(*id).map(|p| p.ctrl == who).unwrap_or(false)),
            );
        if worth && c.rules.kw & kw::CANT_BE_COUNTERED == 0 {
            for a in acts {
                if let Action::Cast {
                    hand,
                    mode,
                    overload: false,
                } = *a
                {
                    let k = d.card(d.s[who].hand[hand]);
                    let m = &k.rules.modes[mode.min(k.rules.modes.len().saturating_sub(1))];
                    if is_counter(&m.effects) {
                        let spec = m.target?;
                        if d.legal_targets(who, &spec, None).contains(&Target::Item(top.id)) {
                            return Some(*a);
                        }
                    }
                }
            }
        }
    }
    // a regeneration shield for a creature about to be destroyed
    for t in &top.targets {
        let Target::Perm(id) = t else { continue };
        let Some(p) = d.perm(*id) else { continue };
        if p.ctrl != who {
            continue;
        }
        let destroys = top.ability.effects.iter().any(|e| matches!(e, Eff::Destroy));
        let damage: i32 = top
            .ability
            .effects
            .iter()
            .map(|e| match e {
                Eff::Damage(n) => *n,
                _ => 0,
            })
            .sum();
        for a in acts {
            match *a {
                Action::Activate { perm, n } if perm == *id => {
                    let (_, _, when) = ability_effects(d, perm, n);
                    if when == crate::rules::Use::Regenerate
                        && p.regen == 0
                        && (destroys || damage >= d.toughness(p) - p.damage)
                    {
                        return Some(*a);
                    }
                }
                Action::Cast {
                    hand,
                    mode,
                    overload: false,
                } if damage > 0 => {
                    let k = d.card(d.s[who].hand[hand]);
                    if let Some(m) = k.rules.modes.get(mode) {
                        if let Some(Eff::Pump(_, tt, _)) = m
                            .effects
                            .iter()
                            .find(|e| matches!(e, Eff::Pump(_, t, _) if *t > 0))
                        {
                            if d.toughness(p) - p.damage + tt > damage && damage >= d.toughness(p) - p.damage
                            {
                                return Some(*a);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn end_step(d: &Duel, who: usize, acts: &[Action]) -> Option<Action> {
    use crate::rules::Use;
    let mut best: Option<(i32, Action)> = None;
    for a in acts {
        match *a {
            Action::Cast { hand, mode, overload } => {
                let card = d.s[who].hand[hand];
                let c = d.card(card);
                let w = if c.is(ty::CREATURE) {
                    Some(10 + c.value as i32 * 3)
                } else {
                    cast_worth(d, who, card, mode, overload)
                };
                if let Some(w) = w {
                    if best.map(|(b, _)| w > b).unwrap_or(true) {
                        best = Some((w, *a));
                    }
                }
            }
            Action::Activate { perm, n } => {
                let (effects, spec, when) = ability_effects(d, perm, n);
                if when == Use::Ping
                    && (spec.is_none() || aim_worth(d, who, spec, &effects, None, Some(perm)).is_some())
                {
                    return Some(*a);
                }
            }
            _ => {}
        }
    }
    best.map(|(_, a)| a)
}

// ---------- combat ----------

fn kills(d: &Duel, a: &Perm, b: &Perm) -> bool {
    let dt = d.kws(a) & kw::DEATHTOUCH != 0;
    let p = d.power(a);
    p > 0 && (dt || p >= d.toughness(b) - b.damage)
}

fn attackers(d: &Duel, who: usize, legal: &[u32]) -> Vec<u32> {
    let opp = other(who);
    let blockers: Vec<&Perm> = d
        .perms
        .iter()
        .filter(|p| p.ctrl == opp && d.is_creature(p) && !p.tapped && p.detained_by.is_none())
        .collect();
    let total: i32 = legal
        .iter()
        .filter_map(|&id| d.perm(id))
        .map(|p| d.power(p))
        .sum();
    let their_power: i32 = blockers.iter().map(|p| d.power(p)).sum();
    // all in when the blockers can't stop enough of it
    let max_block: i32 = {
        let mut powers: Vec<i32> = legal
            .iter()
            .filter_map(|&id| d.perm(id))
            .map(|p| d.power(p))
            .collect();
        powers.sort_unstable_by(|a, b| b.cmp(a));
        powers.iter().take(blockers.len()).sum()
    };
    if total - max_block >= d.s[opp].life {
        return legal.to_vec();
    }
    let mut out = Vec::new();
    for &id in legal {
        let Some(a) = d.perm(id) else { continue };
        if d.power(a) <= 0 {
            continue;
        }
        let can: Vec<&&Perm> = blockers.iter().filter(|b| d.can_block(b, id)).collect();
        let safe = can
            .iter()
            .all(|b| !kills(d, b, a) || (kills(d, a, b) && d.value_of(b.id) >= d.value_of(id)));
        // keep enough back when their attack next turn could finish us
        if safe {
            out.push(id);
        }
    }
    let kept: i32 = d
        .perms
        .iter()
        .filter(|p| p.ctrl == who && d.is_creature(p) && !out.contains(&p.id))
        .map(|p| d.toughness(p))
        .sum();
    if their_power >= d.s[who].life && kept == 0 && out.len() > 1 {
        // hold the toughest at home
        if let Some((i, _)) = out
            .iter()
            .enumerate()
            .max_by_key(|(_, &id)| d.perm(id).map(|p| d.toughness(p)).unwrap_or(0))
        {
            out.remove(i);
        }
    }
    out
}

fn blockers(d: &Duel, who: usize, legal: &[u32], attackers: &[u32]) -> Vec<(u32, u32)> {
    let mut free: Vec<u32> = legal.to_vec();
    let mut out: Vec<(u32, u32)> = Vec::new();
    let mut atk: Vec<u32> = attackers.to_vec();
    atk.sort_by_key(|&id| std::cmp::Reverse(d.perm(id).map(|p| d.power(p)).unwrap_or(0)));
    let power_of = |id: u32| d.perm(id).map(|p| d.power(p)).unwrap_or(0);
    // good blocks: kill it and live, or live (a wall), or trade up
    for &a in &atk {
        let Some(ap) = d.perm(a) else { continue };
        let mut pick: Option<(i32, u32)> = None;
        for &b in &free {
            let Some(bp) = d.perm(b) else { continue };
            if !d.can_block(bp, a) {
                continue;
            }
            let k = kills(d, bp, ap);
            let dies = kills(d, ap, bp);
            let first_a = d.kws(ap) & (kw::FIRST_STRIKE | kw::DOUBLE_STRIKE) != 0
                && d.kws(bp) & (kw::FIRST_STRIKE | kw::DOUBLE_STRIKE) == 0;
            let k = k && !(first_a && dies);
            let score = if k && !dies {
                100 + d.value_of(a)
            } else if !dies {
                50
            } else if k && d.value_of(b) < d.value_of(a) {
                20 + d.value_of(a) - d.value_of(b)
            } else {
                continue;
            };
            if pick.map(|(s, _)| score > s).unwrap_or(true) {
                pick = Some((score, b));
            }
        }
        if let Some((_, b)) = pick {
            out.push((b, a));
            free.retain(|&x| x != b);
        }
    }
    // chump blocks when the rest would kill us
    let unblocked = |out: &Vec<(u32, u32)>| -> i32 {
        atk.iter()
            .filter(|a| !out.iter().any(|(_, x)| x == *a))
            .map(|&a| power_of(a))
            .sum()
    };
    while unblocked(&out) >= d.s[who].life {
        let Some(&a) = atk.iter().find(|a| {
            !out.iter().any(|(_, x)| x == *a)
                && free
                    .iter()
                    .any(|&b| d.perm(b).map(|bp| d.can_block(bp, **a)).unwrap_or(false))
        }) else {
            break;
        };
        let Some(b) = free
            .iter()
            .copied()
            .filter(|&b| d.perm(b).map(|bp| d.can_block(bp, a)).unwrap_or(false))
            .min_by_key(|&b| d.value_of(b))
        else {
            break;
        };
        out.push((b, a));
        free.retain(|&x| x != b);
    }
    let _ = P;
    out
}
