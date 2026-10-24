//! Setting up what the map hands the player: the foe a creature card brings,
//! the guardian and its deck, the player's side of a fight, a new game's
//! starting deck and lands, and what each city sells.
//!
//! A card's three readings (the person's "3 modes, based on their mana
//! cost, abilities, and type"): its type picks the encounter (`Kind`), its
//! mana value sets the tier (how much life and mana the foe brings and how
//! big its deck is), and its abilities are its rules in the fight.

use crate::battle::SideSetup;
use crate::content::{ty, Card};
use crate::mana::{self, B, R, U};
use crate::player::{Armoury, Player};
use crate::rng::Rng;
use crate::rules;
use crate::world::{City, Guild, Pos, World};

/// Foe life by tier 1, 2, 3.
pub const FOE_LIFE: [i32; 3] = [6, 9, 13];
/// Cards in a foe's deck besides the creature you met.
pub const FOE_DECK: usize = 12;
/// Gold for a won fight, a tier.
pub const GOLD_PER_TIER: i32 = 3;
pub const GUARDIAN_LIFE: i32 = 30;
pub const GUARDIAN_DECK: usize = 40;
pub const GUARDIAN_COPIES: usize = 2;
pub const STARTER_DECK: usize = 12;
pub const SHOP_CARDS: usize = 6;
pub const SELL_DIVISOR: i32 = 3;
pub const INN_REST: i32 = 6;
pub const INN_SUPPLIES: i32 = 4;
pub const INN_SUPPLIES_PRICE: i32 = 5;

pub fn card_price(c: &Card) -> i32 {
    match c.rarity {
        'c' => 6,
        'u' => 12,
        'r' => 24,
        _ => 40,
    }
}

/// The foe a creature card brings from a tile.
pub fn foe(cards: &[Card], creature: usize, pool: &[usize], rng: &mut Rng) -> SideSetup {
    let c = &cards[creature];
    let tier = c.tier() as usize;
    let id = c.identity;
    let mut deck: Vec<usize> = pool
        .iter()
        .copied()
        .filter(|&i| i != creature && !cards[i].is(ty::LAND) && cards[i].identity & !id == 0)
        .collect();
    let mut extra: Vec<usize> = (0..cards.len())
        .filter(|&i| {
            let k = &cards[i];
            !k.is(ty::LAND)
                && k.identity != 0
                && k.identity & !id == 0
                && k.value <= c.value + 1
                && k.name != "Niv-Mizzet, Dracogenius"
        })
        .collect();
    while deck.len() < FOE_DECK && !extra.is_empty() {
        let k = (rng.next_u64() % extra.len() as u64) as usize;
        deck.push(extra.remove(k));
    }
    let unit = if id == 0 { mana::C } else { id };
    let income: Vec<u8> = (0..tier).map(|_| unit).collect();
    let swamps = if id == B { income.len() } else { 0 };
    SideSetup {
        name: c.name.clone(),
        life: FOE_LIFE[tier - 1],
        deck,
        income,
        swamps,
        in_play: vec![creature],
        perks: 0,
    }
}

/// Niv-Mizzet, Dracogenius and his deck: two of each blue, red and Izzet
/// card of the region, dearest first, to forty.
pub fn guardian(cards: &[Card]) -> SideSetup {
    let niv = cards
        .iter()
        .position(|c| c.name == "Niv-Mizzet, Dracogenius")
        .unwrap_or(0);
    let mut pool: Vec<usize> = (0..cards.len())
        .filter(|&i| {
            let k = &cards[i];
            i != niv && !k.is(ty::LAND) && k.identity != 0 && k.identity & !(U | R) == 0
        })
        .collect();
    pool.sort_by_key(|&i| (std::cmp::Reverse(cards[i].value), cards[i].name.clone()));
    let mut deck = Vec::new();
    for _ in 0..GUARDIAN_COPIES {
        deck.extend(pool.iter().copied());
    }
    deck.sort_by_key(|&i| (std::cmp::Reverse(cards[i].value), cards[i].name.clone()));
    deck.truncate(GUARDIAN_DECK);
    SideSetup {
        name: cards[niv].name.clone(),
        life: GUARDIAN_LIFE,
        deck,
        income: vec![U, R, U, R, U | R],
        swamps: 0,
        in_play: vec![niv],
        perks: 0,
    }
}

/// The player's side of a fight at `at`.
pub fn player_side(
    cards: &[Card],
    armoury: &Armoury,
    world: &World,
    p: &Player,
    at: Pos,
    name: &str,
) -> SideSetup {
    let lands = world.lands_in_reach(at, p.reach(armoury));
    let income: Vec<u8> = lands
        .iter()
        .filter_map(|(_, l)| rules::land_unit(&cards[*l].name))
        .collect();
    let swamps = lands.iter().filter(|(_, l)| cards[*l].name == "Swamp").count();
    SideSetup {
        name: name.to_string(),
        life: p.health,
        deck: p.deck.clone(),
        income,
        swamps,
        in_play: Vec::new(),
        perks: p.perks(armoury),
    }
}

/// A new game's deck for each guild: five or six of its common and
/// uncommon creatures and the tricks and removal that suit them, mana value
/// five at most. Chosen by hand so each guild starts with a deck that plays
/// like its guild (SECOND-ORDER R2.5); `balance.rs` checks they hold up.
pub const STARTERS: [(Guild, [&str; STARTER_DECK]); 5] = [
    (
        Guild::Azorius,
        [
            "Trained Caracal",
            "Azorius Arrester",
            "Concordia Pegasus",
            "Sunspire Griffin",
            "Vassal Soul",
            "Vassal Soul",
            "Stealer of Secrets",
            "Hover Barrier",
            "Inaction Injunction",
            "Swift Justice",
            "Downsize",
            "Seller of Songbirds",
        ],
    ),
    (
        Guild::Izzet,
        [
            "Frostburn Weird",
            "Frostburn Weird",
            "Goblin Electromancer",
            "Gore-House Chainwalker",
            "Splatter Thug",
            "Stealer of Secrets",
            "Bellows Lizard",
            "Runewing",
            "Electrickery",
            "Annihilating Fire",
            "Inaction Injunction",
            "Downsize",
        ],
    ),
    (
        Guild::Rakdos,
        [
            "Rakdos Shred-Freak",
            "Rakdos Cackler",
            "Rakdos Cackler",
            "Daggerdrome Imp",
            "Dead Reveler",
            "Splatter Thug",
            "Grim Roustabout",
            "Gore-House Chainwalker",
            "Catacomb Slug",
            "Annihilating Fire",
            "Electrickery",
            "Mind Rot",
        ],
    ),
    (
        Guild::Golgari,
        [
            "Drudge Beetle",
            "Drudge Beetle",
            "Dead Reveler",
            "Daggerdrome Imp",
            "Sluiceway Scorpion",
            "Korozda Monitor",
            "Grim Roustabout",
            "Catacomb Slug",
            "Towering Indrik",
            "Giant Growth",
            "Aerial Predation",
            "Mind Rot",
        ],
    ),
    (
        Guild::Selesnya,
        [
            "Trained Caracal",
            "Seller of Songbirds",
            "Sunspire Griffin",
            "Centaur Healer",
            "Drudge Beetle",
            "Concordia Pegasus",
            "Dryad Militant",
            "Towering Indrik",
            "Brushstrider",
            "Giant Growth",
            "Swift Justice",
            "Savage Surge",
        ],
    ),
];

pub fn starter_deck(cards: &[Card], g: Guild) -> Vec<usize> {
    // every name is a card of the region (tested below)
    STARTERS
        .iter()
        .find(|(k, _)| *k == g)
        .map(|(_, names)| {
            names
                .iter()
                .filter_map(|n| cards.iter().position(|c| c.name == *n))
                .collect()
        })
        .unwrap_or_default()
}

/// A new game's lands, attached to the Tenth District: the guild's two basics and its gate.
pub fn starter_lands(cards: &[Card], g: Guild) -> Vec<usize> {
    let by = |n: &str| cards.iter().position(|c| c.name == n);
    let basics: Vec<&str> = mana::COLOURS
        .iter()
        .filter(|c| g.colours() & **c != 0)
        .map(|c| crate::world::Terrain::of_colour(*c).land_name())
        .collect();
    let gate = format!("{} Guildgate", g.name());
    basics.into_iter().chain([gate.as_str()]).filter_map(by).collect()
}

/// The cards a city sells, fixed per city and seed.
pub fn shop_cards(cards: &[Card], city: City, seed: u64) -> Vec<usize> {
    let colours = city.guild().map(|g| g.colours()).unwrap_or(mana::ANY);
    let mut pool: Vec<usize> = (0..cards.len())
        .filter(|&i| {
            let k = &cards[i];
            !k.is(ty::LAND)
                && k.identity & !colours == 0
                && k.rarity != 'm'
                && k.name != "Niv-Mizzet, Dracogenius"
        })
        .collect();
    let mut rng = Rng::new(seed ^ (city as u64 + 1).wrapping_mul(0x51_7C_C1_B7));
    let mut out = Vec::new();
    while out.len() < SHOP_CARDS && !pool.is_empty() {
        let k = (rng.next_u64() % pool.len() as u64) as usize;
        out.push(pool.remove(k));
    }
    out
}

/// The armour a city's armourer sells.
pub fn shop_armour(armoury: &Armoury, city: City) -> Vec<String> {
    let guilds: [&str; 2] = match city {
        City::Tenth => ["Rakdos", "Golgari"],
        City::Prahv => ["Azorius", "Izzet"],
        City::Vitu => ["Selesnya", "Golgari"],
    };
    armoury
        .pieces
        .iter()
        .filter(|p| guilds.contains(&p.guild.as_str()))
        .map(|p| p.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::{run, Outcome, Setup};
    use crate::content::for_tests;
    use crate::player::parse_armour;

    #[test]
    fn a_foe_brings_its_creature_in_play_and_life_by_tier() {
        let cards = for_tests();
        let mangler = crate::content::find(&cards, "Dreg Mangler");
        let f = foe(&cards, mangler, &[], &mut Rng::new(1));
        assert_eq!(f.in_play, vec![mangler]);
        // mana value 3: tier 2, 9 life, two units of black-green
        assert_eq!(f.life, 9);
        assert_eq!(f.income, vec![B | crate::mana::G; 2]);
        assert_eq!(f.deck.len(), FOE_DECK);
        // twelve cards: enough that a foe does not run out before a fight is decided
        assert!(f
            .deck
            .iter()
            .all(|&i| cards[i].identity & !(B | crate::mana::G) == 0));
    }

    #[test]
    fn the_guardian_has_forty_blue_and_red_cards_and_niv_mizzet_in_play() {
        let cards = for_tests();
        let g = guardian(&cards);
        assert_eq!(g.deck.len(), 40);
        assert_eq!(g.life, 30);
        assert!(g
            .deck
            .iter()
            .all(|&i| cards[i].identity & !(U | R) == 0 && cards[i].identity != 0));
        assert!(g
            .deck
            .iter()
            .all(|&i| g.deck.iter().filter(|&&j| j == i).count() <= 2));
        assert_eq!(cards[g.in_play[0]].name, "Niv-Mizzet, Dracogenius");
    }

    #[test]
    fn a_starter_deck_and_its_lands_fit_the_guild() {
        let cards = for_tests();
        for g in Guild::ALL {
            let d = starter_deck(&cards, g);
            assert_eq!(
                d.len(),
                STARTER_DECK,
                "{g:?}: a starter name is not a card of the region"
            );
            assert!(d.iter().all(|&i| cards[i].identity & !g.colours() == 0));
            assert_eq!(starter_lands(&cards, g).len(), 3);
        }
    }

    #[test]
    fn a_starter_deck_can_beat_a_tier_one_foe_most_of_the_time() {
        let cards = for_tests();
        let a = parse_armour(include_str!("../../../data/armour.json")).unwrap();
        let p = Player::new(&a);
        let caracal = crate::content::find(&cards, "Trained Caracal");
        let mut wins = 0;
        for seed in 0..40 {
            let deck = starter_deck(&cards, Guild::Selesnya);
            let lands = starter_lands(&cards, Guild::Selesnya);
            let me = SideSetup {
                name: "you".into(),
                life: p.health,
                deck,
                income: lands
                    .iter()
                    .filter_map(|&l| rules::land_unit(&cards[l].name))
                    .collect(),
                ..SideSetup::default()
            };
            let f = foe(&cards, caracal, &[], &mut Rng::new(seed));
            let r = run(&cards, &Setup { sides: [me, f], seed });
            if r.outcome == Outcome::Won {
                wins += 1;
            }
        }
        assert!(wins >= 24, "{wins} of 40");
    }

    #[test]
    fn each_city_sells_six_cards_of_its_colours() {
        let cards = for_tests();
        let s = shop_cards(&cards, City::Prahv, 4);
        assert_eq!(s.len(), 6);
        assert!(s.iter().all(|&i| cards[i].identity & !(crate::mana::W | U) == 0));
        assert_eq!(s, shop_cards(&cards, City::Prahv, 4));
    }
}
