//! Balance: many seeded fights against the guardian and a few foes, with
//! decks a player could have at different points of the region. The ranges
//! are the plan's targets (PLAN.md): a starting deck cannot beat Niv-Mizzet,
//! a deck built through the region usually can. Run with --nocapture to see
//! the rates.

use tool_core::battle::{run, Outcome, Setup, SideSetup};
use tool_core::content;
use tool_core::encounter;
use tool_core::mana::{G, R, U, W};
use tool_core::rng::Rng;

fn cards() -> Vec<content::Card> {
    content::parse(include_str!("../../../data/cards.json")).unwrap()
}

fn deck(cards: &[content::Card], names: &[&str]) -> Vec<usize> {
    names
        .iter()
        .map(|n| {
            cards
                .iter()
                .position(|c| c.name == *n)
                .unwrap_or_else(|| panic!("{n}"))
        })
        .collect()
}

fn rate(cards: &[content::Card], me: &SideSetup, foe: &dyn Fn(u64) -> SideSetup, n: u64) -> u64 {
    (0..n)
        .filter(|&seed| {
            run(
                cards,
                &Setup {
                    sides: [me.clone(), foe(seed)],
                    seed: seed * 7919 + 3,
                },
            )
            .outcome
                == Outcome::Won
        })
        .count() as u64
}

fn side(deck: Vec<usize>, life: i32, income: Vec<u8>) -> SideSetup {
    SideSetup {
        name: "you".into(),
        life,
        deck,
        income,
        ..SideSetup::default()
    }
}

#[test]
fn the_guardian_beats_a_starting_deck_and_falls_to_a_built_one() {
    let c = cards();
    let guardian = |_s: u64| encounter::guardian(&c);
    let starter = side(
        encounter::starter_deck(&c, tool_core::world::Guild::Izzet),
        18,
        vec![U, R, U | R],
    );
    let built = side(
        deck(
            &c,
            &[
                "Frostburn Weird",
                "Frostburn Weird",
                "Goblin Electromancer",
                "Mizzium Mortars",
                "Mizzium Mortars",
                "Izzet Charm",
                "Annihilating Fire",
                "Explosive Impact",
                "Cancel",
                "Skyline Predator",
                "Isperia's Skywatch",
                "Runewing",
                "Hover Barrier",
                "Guttersnipe",
                "Electrickery",
                "Inspiration",
                "Voidwielder",
                "Cyclonic Rift",
                "Tenement Crasher",
                "Stealer of Secrets",
            ],
        ),
        26,
        vec![U, U, R, R, U | R, U | R, U],
    );
    let green = side(
        deck(
            &c,
            &[
                "Archweaver",
                "Towering Indrik",
                "Towering Indrik",
                "Axebane Stag",
                "Deadbridge Goliath",
                "Rubbleback Rhino",
                "Korozda Monitor",
                "Giant Growth",
                "Giant Growth",
                "Aerial Predation",
                "Aerial Predation",
                "Savage Surge",
                "Horncaller's Chant",
                "Centaur Healer",
                "Selesnya Charm",
                "Brushstrider",
                "Drudge Beetle",
                "Centaur's Herald",
                "Chorus of Might",
                "Death's Presence",
            ],
        ),
        28,
        vec![G, G, G, W, G | W, G, W],
    );
    let n = 60;
    let a = rate(&c, &starter, &guardian, n);
    let b = rate(&c, &built, &guardian, n);
    let g = rate(&c, &green, &guardian, n);
    println!("guardian: starter {a}/{n}, built blue-red {b}/{n}, built green {g}/{n}");
    assert!(a <= n / 10, "a starting deck beat the guardian {a} of {n}");
    assert!(b >= n / 3 && b <= n * 9 / 10, "a built blue-red deck: {b} of {n}");
    assert!(g >= n / 3 && g <= n * 9 / 10, "a built green deck: {g} of {n}");
}

#[test]
fn a_starting_deck_mostly_beats_the_first_chunks_foes() {
    let c = cards();
    for (guild, lands) in [
        (tool_core::world::Guild::Izzet, vec![U, R, U | R]),
        (tool_core::world::Guild::Selesnya, vec![G, W, G | W]),
        (
            tool_core::world::Guild::Rakdos,
            vec![tool_core::mana::B, R, tool_core::mana::B | R],
        ),
    ] {
        let me = side(encounter::starter_deck(&c, guild), 18, lands);
        for foe_name in [
            "Bellows Lizard",
            "Dead Reveler",
            "Sunspire Griffin",
            "Stealer of Secrets",
            "Centaur Healer",
        ] {
            let f = c.iter().position(|k| k.name == foe_name).unwrap();
            let foe = |s: u64| encounter::foe(&c, f, &[], &mut Rng::new(s));
            let n = 40;
            let w = rate(&c, &me, &foe, n);
            println!("{guild:?} starter vs {foe_name}: {w}/{n}");
            assert!(w >= n / 4, "{guild:?} starter beat {foe_name} only {w} of {n}");
        }
    }
}
