//! A playthrough: a bot plays the first region through `act()` alone, the
//! way the page does, from a new game to the fight with Niv-Mizzet. It walks
//! toward New Prahv for the writ, then to Nivix, takes whatever each tile
//! draws (fights, the first choice it can make, the first fate card), places
//! lands near the lair, and rests and buys armour in cities. It shows the
//! region can be played to its end; it does not show the bot plays well.
//!
//! The save from the moment before the guardian fight is written to
//! target/playthrough-lair.json, for screenshots (tools/shots.py).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use tool_core::session::{Room, Screen, Session};
use tool_core::world::{Feature, Pos};

fn bundle() -> String {
    let mut b: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in [
        ("cards.json", include_str!("../../../data/cards.json")),
        ("quests.json", include_str!("../../../data/quests.json")),
        ("armour.json", include_str!("../../../data/armour.json")),
        ("palette.json", include_str!("../../../data/palette.json")),
        ("models.json", include_str!("../../../data/models.json")),
        ("strings.json", include_str!("../../../data/strings.json")),
        ("glossary.md", include_str!("../../../GLOSSARY.md")),
        ("stories/W.json", include_str!("../../../data/stories/W.json")),
        ("stories/U.json", include_str!("../../../data/stories/U.json")),
        ("stories/B.json", include_str!("../../../data/stories/B.json")),
        ("stories/R.json", include_str!("../../../data/stories/R.json")),
        ("stories/G.json", include_str!("../../../data/stories/G.json")),
        (
            "stories/Azorius.json",
            include_str!("../../../data/stories/Azorius.json"),
        ),
        (
            "stories/Izzet.json",
            include_str!("../../../data/stories/Izzet.json"),
        ),
        (
            "stories/Rakdos.json",
            include_str!("../../../data/stories/Rakdos.json"),
        ),
        (
            "stories/Golgari.json",
            include_str!("../../../data/stories/Golgari.json"),
        ),
        (
            "stories/Selesnya.json",
            include_str!("../../../data/stories/Selesnya.json"),
        ),
        (
            "stories/colourless.json",
            include_str!("../../../data/stories/colourless.json"),
        ),
    ] {
        b.insert(k.to_string(), v.to_string());
    }
    serde_json::to_string(&b).unwrap()
}

/// The next step from `from` toward `goal` over tiles known to be passable,
/// or toward the known tile nearest the goal when the goal is not made yet.
fn next_step(s: &Session, from: Pos, goal: Pos) -> Option<Pos> {
    let w = s.world();
    let target = if w.tile(goal).is_some() {
        goal
    } else {
        *w.tiles
            .iter()
            .filter(|(_, t)| t.passable())
            .min_by_key(|(p, _)| ((p.0 - goal.0).abs() + (p.1 - goal.1).abs(), p.0, p.1))?
            .0
    };
    let mut prev: BTreeMap<Pos, Pos> = BTreeMap::new();
    let mut seen = BTreeSet::from([from]);
    let mut q = VecDeque::from([from]);
    while let Some(p) = q.pop_front() {
        if p == target {
            let mut at = p;
            while prev.get(&at) != Some(&from) {
                at = *prev.get(&at)?;
            }
            return Some(at);
        }
        for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (p.0 + d.0, p.1 + d.1);
            if seen.contains(&n) || !w.tile(n).map(|t| t.passable()).unwrap_or(false) {
                continue;
            }
            seen.insert(n);
            prev.insert(n, p);
            q.push_back(n);
        }
    }
    None
}

fn dir(from: Pos, to: Pos) -> &'static str {
    match (to.0 - from.0, to.1 - from.1) {
        (1, 0) => "e",
        (-1, 0) => "w",
        (0, 1) => "s",
        _ => "n",
    }
}

#[test]
fn a_bot_can_play_the_first_region_to_the_guardian() {
    let mut s = Session::new(&bundle(), "", 2026).unwrap();
    s.act("game.new", "").unwrap();
    s.act("game.guild", "Izzet").unwrap();
    let prahv = (0, -5);
    let lair = (5, -5);
    let mut reached_guardian = false;
    let mut won = false;
    let mut fights = 0;
    let mut lands_placed = 0;
    for _ in 0..4000 {
        s.walk = None;
        let screen = s.screen.clone();
        match screen {
            Screen::Result { .. } => s.act("result.next", "").unwrap(),
            Screen::Draw { .. } => s.act("first", "").unwrap(),
            Screen::Story { card, .. } => {
                let st = s.stories[&s.cards[card].number].clone();
                let i = st.choices.iter().position(|c| s.can_choose(c)).unwrap_or(0);
                s.act("story.choose", &i.to_string()).unwrap();
            }
            Screen::Fate { shuffled, .. } => {
                if shuffled {
                    s.act("fate.pick", "0").unwrap();
                } else {
                    s.act("fate.shuffle", "").unwrap();
                }
            }
            Screen::Land { .. } => {
                // place it on the visited tile with room nearest the lair
                let spot = s
                    .world()
                    .tiles
                    .iter()
                    .filter(|(_, t)| t.visited && t.lands.len() < t.land_room())
                    .min_by_key(|(p, _)| ((p.0 - lair.0).abs() + (p.1 - lair.1).abs(), p.0, p.1))
                    .map(|(p, _)| *p)
                    .unwrap();
                s.act("land.place", &format!("{},{}", spot.0, spot.1)).unwrap();
                lands_placed += 1;
            }
            Screen::Battle(f) => {
                if f.guardian {
                    reached_guardian = true;
                }
                fights += 1;
                s.act("battle.skip", "").unwrap();
                s.act("battle.done", "").unwrap();
            }
            Screen::City { room, .. } => {
                if room == Room::Menu {
                    s.act("city.room", "inn").unwrap();
                    let _ = s.act("inn.rest", "");
                    let _ = s.act("inn.supplies", "");
                    s.act("city.room", "menu").unwrap();
                }
                s.act("city.leave", "").unwrap();
            }
            Screen::Lair => {
                if s.player().quests.get("dracogenius").copied().unwrap_or(0) >= 2 {
                    if let Ok(save) = serde_json::to_string(&s.saved) {
                        let _ = std::fs::write(
                            concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/playthrough-lair.json"),
                            save,
                        );
                    }
                    s.act("lair.climb", "").unwrap();
                } else {
                    s.act("lair.leave", "").unwrap();
                }
            }
            Screen::Victory => {
                won = true;
                break;
            }
            Screen::Board => {
                let at = s.player().pos;
                let writ = s.player().quests.get("dracogenius").copied().unwrap_or(0) >= 2;
                let goal = if writ { lair } else { prahv };
                // a guardian fight lost: build up by wandering, then try again
                let goal = if reached_guardian && s.player().fights_won < 12 {
                    (at.0 - 1, at.1 + 1)
                } else {
                    goal
                };
                match next_step(&s, at, goal) {
                    Some(n) => {
                        let _ = s.act("move", dir(at, n));
                    }
                    None => {
                        // stuck: try each direction
                        for d in ["n", "e", "s", "w"] {
                            if s.act("move", d).is_ok() {
                                break;
                            }
                        }
                    }
                }
            }
            Screen::Title | Screen::NewGame | Screen::Deck { .. } | Screen::Models { .. } => {
                s.act("screen.show", "board").unwrap()
            }
        }
    }
    let lair_found = s.world().find(Feature::Lair).is_some();
    assert!(lair_found, "the lair's chunk was never made");
    assert!(
        s.player().quests["dracogenius"] >= 2,
        "the bot never got the writ at New Prahv"
    );
    assert!(
        reached_guardian,
        "the bot never fought Niv-Mizzet ({fights} fights, {lands_placed} lands)"
    );
    println!("playthrough: {fights} fights, {lands_placed} lands placed, guardian beaten: {won}");
}
