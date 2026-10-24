//! Balance: many seeded games of Magic against the guardian and a few foes,
//! the computer playing both sides, with decks a player could have at
//! different points of the region. The ranges are the plan's targets
//! (PLAN.md): a starting deck cannot beat Niv-Mizzet, a deck built through
//! the region often can. Run with --nocapture to see the rates.

use std::rc::Rc;

use tool_core::content;
use tool_core::duel::{Duel, Outcome, Setup, SideSetup};
use tool_core::encounter;
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
    let shared = Rc::new(cards.to_vec());
    (0..n)
        .filter(|&seed| {
            let mut d = Duel::new(
                shared.clone(),
                &Setup {
                    sides: [me.clone(), foe(seed)],
                    seed: seed * 7919 + 3,
                },
                [false, false],
            );
            d.run();
            d.outcome == Some(Outcome::Won)
        })
        .count() as u64
}

fn side(cards: &[content::Card], deck: Vec<usize>, life: i32, lands: &[&str]) -> SideSetup {
    let lands = encounter::fill_pile(cards, &deck, self::deck(cards, lands));
    SideSetup {
        name: "you".into(),
        life,
        deck,
        lands,
        ..SideSetup::default()
    }
}

#[test]
fn the_guardian_beats_a_starting_deck_and_falls_to_a_built_one() {
    let c = cards();
    let guardian = |_s: u64| encounter::guardian(&c);
    let starter = side(
        &c,
        encounter::starter_deck(&c, tool_core::world::Guild::Izzet),
        18,
        &["Island", "Mountain", "Izzet Guildgate"],
    );
    let built = side(
        &c,
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
        &[
            "Island",
            "Island",
            "Mountain",
            "Mountain",
            "Izzet Guildgate",
            "Izzet Guildgate",
            "Island",
        ],
    );
    let green = side(
        &c,
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
        &[
            "Forest",
            "Forest",
            "Forest",
            "Plains",
            "Selesnya Guildgate",
            "Forest",
            "Plains",
        ],
    );
    let n = 60;
    let a = rate(&c, &starter, &guardian, n);
    let b = rate(&c, &built, &guardian, n);
    let g = rate(&c, &green, &guardian, n);
    println!("guardian: starter {a}/{n}, built blue-red {b}/{n}, built green {g}/{n}");
    assert!(a <= n / 10, "a starting deck beat the guardian {a} of {n}");
    // the computer plays the player's side here; a person plays it better
    assert!(b >= n / 4 && b <= n * 9 / 10, "a built blue-red deck: {b} of {n}");
    assert!(g >= n / 4 && g <= n * 9 / 10, "a built green deck: {g} of {n}");
}

#[test]
fn a_starting_deck_beats_most_tier_one_foes_and_few_tier_three() {
    let c = cards();
    let mut by_tier = [(0u64, 0u64); 3];
    for guild in tool_core::world::Guild::ALL {
        let lands = encounter::starter_lands(&c, guild);
        let names: Vec<&str> = lands.iter().map(|&l| c[l].name.as_str()).collect();
        let me = side(&c, encounter::starter_deck(&c, guild), 18, &names);
        for (f, k) in c.iter().enumerate() {
            if !k.is(content::ty::CREATURE) || k.name == "Niv-Mizzet, Dracogenius" {
                continue;
            }
            let foe = |s: u64| encounter::foe(&c, f, &[], &mut Rng::new(s));
            let n = 4;
            let t = k.tier() as usize - 1;
            by_tier[t].0 += rate(&c, &me, &foe, n);
            by_tier[t].1 += n;
        }
    }
    println!("starters against foes by tier (won, played): {by_tier:?}");
    let pct = |(w, n): (u64, u64)| w * 100 / n;
    assert!(pct(by_tier[0]) >= 70, "tier one: {:?}", by_tier[0]);
    assert!(
        pct(by_tier[1]) >= 35 && pct(by_tier[1]) <= 80,
        "tier two: {:?}",
        by_tier[1]
    );
    assert!(pct(by_tier[2]) <= 50, "tier three: {:?}", by_tier[2]);
}
