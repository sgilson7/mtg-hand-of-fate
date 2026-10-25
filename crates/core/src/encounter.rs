//! Setting up what the map hands the player: the foe a creature card brings,
//! the guardian and its deck, the player's side of a fight, a new game's
//! starting deck and lands, and what each city sells.
//!
//! A card's three readings (the person's "3 modes, based on their mana
//! cost, abilities, and type"): its type picks the encounter (`Kind`), its
//! mana value sets the tier (how much life and mana the foe brings and how
//! big its deck is), and its abilities are its rules in the fight.

use crate::content::{ty, Card};
use crate::duel::SideSetup;
use crate::mana::{self, B, R, U};
use crate::player::{Armoury, Player};
use crate::rng::Rng;
use crate::rules;
use crate::world::{City, Guild, Pos, World};

/// Foe life by tier 1, 2, 3.
pub const FOE_LIFE: [i32; 3] = [8, 12, 16];
/// Lands in a foe's land pile by tier 1, 2, 3.
pub const FOE_LANDS: [usize; 3] = [3, 5, 7];
/// A land pile holds at least this many lands: when fewer lie within Reach,
/// basics of the deck's colours make up the rest (the guard against mana
/// screw; DECISIONS.md, MVP 2).
pub const PILE_MIN: usize = 5;
pub const GUARDIAN_LANDS: usize = 7;
/// Cards in a foe's deck besides the creature you met.
pub const FOE_DECK: usize = 12;
/// Gold for a won fight, a tier.
pub const GOLD_PER_TIER: i32 = 3;
pub const GUARDIAN_LIFE: i32 = 16;
pub const GUARDIAN_DECK: usize = 40;
pub const STARTER_DECK: usize = 12;
pub const SHOP_CARDS: usize = 6;
pub const SELL_DIVISOR: i32 = 3;
pub const INN_REST: i32 = 6;
/// Six supplies for 2 gold (T26: they were four for 5).
pub const INN_SUPPLIES: i32 = 6;
pub const INN_SUPPLIES_PRICE: i32 = 2;

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
    let lands = basics_for(cards, if id == 0 { mana::ANY } else { id }, FOE_LANDS[tier - 1]);
    SideSetup {
        name: c.name.clone(),
        life: FOE_LIFE[tier - 1],
        deck,
        lands,
        in_play: vec![creature],
        perks: 0,
    }
}

/// Niv-Mizzet's forty, chosen by hand: an Izzet spells deck with a curve
/// from one to six, two of each of its best cards (BRIEF: "a sophisticated
/// deck"). Every name is a blue, red or Izzet card of the region (tested).
pub const GUARDIAN_LIST: [(&str, usize); 29] = [
    ("Frostburn Weird", 2),
    ("Goblin Electromancer", 2),
    ("Izzet Charm", 2),
    ("Mizzium Mortars", 2),
    ("Annihilating Fire", 2),
    ("Ash Zealot", 2),
    ("Splatter Thug", 2),
    ("Stealer of Secrets", 2),
    ("Cancel", 2),
    ("Runewing", 2),
    ("Guttersnipe", 2),
    ("Gore-House Chainwalker", 1),
    ("Electrickery", 1),
    ("Inaction Injunction", 1),
    ("Cyclonic Rift", 1),
    ("Hover Barrier", 1),
    ("Soulsworn Spirit", 1),
    ("Voidwielder", 1),
    ("Isperia's Skywatch", 1),
    ("Skyline Predator", 1),
    ("Bloodfray Giant", 1),
    ("Explosive Impact", 1),
    ("Tenement Crasher", 1),
    ("Cobblebrute", 1),
    ("Inspiration", 1),
    ("Downsize", 1),
    ("Blustersquall", 1),
    ("Lobber Crew", 1),
    ("Goblin Rally", 1),
];

/// Niv-Mizzet, Dracogenius, in play from the start, and his forty.
pub fn guardian(cards: &[Card]) -> SideSetup {
    let niv = cards
        .iter()
        .position(|c| c.name == "Niv-Mizzet, Dracogenius")
        .unwrap_or(0);
    let mut deck = Vec::new();
    for (name, n) in GUARDIAN_LIST {
        if let Some(i) = cards.iter().position(|c| c.name == name) {
            deck.extend(std::iter::repeat_n(i, n));
        }
    }
    SideSetup {
        name: cards[niv].name.clone(),
        life: GUARDIAN_LIFE,
        deck,
        lands: {
            let mut l = basics_for(cards, U | R, GUARDIAN_LANDS - 2);
            let gate = cards.iter().position(|c| c.name == "Izzet Guildgate");
            l.extend(gate.into_iter().chain(gate));
            l
        },
        in_play: vec![niv],
        perks: 0,
    }
}

/// Rakdos's forty (T32), chosen by hand to stand with Niv-Mizzet's: like his,
/// mostly spells, removal and burn, two of the best, with fourteen creatures.
/// Rakdos makes each creature cheaper by the life you lost that turn, so a
/// deck of creatures behind him came down free and won nearly every game
/// (SECOND-ORDER R8.4). Every name is a black, red or Rakdos card (tested).
pub const RAKDOS_LIST: [(&str, usize); 33] = [
    ("Rakdos Cackler", 2),
    ("Gore-House Chainwalker", 1),
    ("Ash Zealot", 1),
    ("Thrill-Kill Assassin", 1),
    ("Splatter Thug", 1),
    ("Dead Reveler", 1),
    ("Guttersnipe", 2),
    ("Hellhole Flailer", 1),
    ("Cobblebrute", 1),
    ("Spawn of Rix Maadi", 1),
    ("Bloodfray Giant", 1),
    ("Ogre Jailbreaker", 1),
    ("Annihilating Fire", 2),
    ("Auger Spree", 2),
    ("Dreadbore", 1),
    ("Rakdos Charm", 2),
    ("Ultimate Price", 1),
    ("Mizzium Mortars", 2),
    ("Electrickery", 1),
    ("Street Spasm", 1),
    ("Launch Party", 1),
    ("Explosive Impact", 1),
    ("Skull Rend", 1),
    ("Mind Rot", 2),
    ("Goblin Rally", 1),
    ("Traitorous Instinct", 1),
    ("Assassin's Strike", 1),
    ("Stab Wound", 1),
    ("Rakdos Keyrune", 1),
    ("Shrieking Affliction", 1),
    ("Pyroconvergence", 1),
    ("Dynacharge", 1),
    ("Cremate", 1),
];
/// Less than Niv-Mizzet's 16: Rakdos hits harder (`balance.rs`).
pub const RAKDOS_LIFE: i32 = 12;
/// Life the Lyev Column's quest takes from him (T34).
pub const RAKDOS_WEAKENED: i32 = 4;
/// A minion of his fights as a tier-three foe with this much life (T32).
pub const MINION_LIFE: i32 = 18;

/// Rakdos, Lord of Riots, in play from the start, and his forty.
pub fn rakdos(cards: &[Card], weakened: bool) -> SideSetup {
    let me = cards
        .iter()
        .position(|c| c.name == "Rakdos, Lord of Riots")
        .unwrap_or(0);
    let mut deck = Vec::new();
    for (name, n) in RAKDOS_LIST {
        if let Some(i) = cards.iter().position(|c| c.name == name) {
            deck.extend(std::iter::repeat_n(i, n));
        }
    }
    SideSetup {
        name: cards[me].name.clone(),
        life: RAKDOS_LIFE - if weakened { RAKDOS_WEAKENED } else { 0 },
        deck,
        lands: {
            let mut l = basics_for(cards, B | R, GUARDIAN_LANDS - 2);
            let gate = cards.iter().position(|c| c.name == "Rakdos Guildgate");
            l.extend(gate.into_iter().chain(gate));
            l
        },
        in_play: vec![me],
        perks: 0,
    }
}

/// A minion Rakdos left on a razed tile: whatever its size, a tier-three
/// foe's lands and more life than one.
pub fn minion(cards: &[Card], creature: usize, rng: &mut Rng) -> SideSetup {
    let mut f = foe(cards, creature, &[], rng);
    f.life = MINION_LIFE;
    f.lands = basics_for(cards, B | R, FOE_LANDS[2]);
    f
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
    let mut lands: Vec<usize> = world
        .lands_in_reach(at, p.reach(armoury))
        .into_iter()
        .map(|(_, l)| l)
        .collect();
    lands.sort_unstable();
    SideSetup {
        name: name.to_string(),
        life: p.health,
        deck: p.deck.clone(),
        lands: fill_pile(cards, &p.deck, lands),
        in_play: Vec::new(),
        perks: p.perks(armoury),
    }
}

/// `n` basic lands for the colours in `mask`, the colours taking turns.
pub fn basics_for(cards: &[Card], mask: u8, n: usize) -> Vec<usize> {
    let names: Vec<usize> = mana::COLOURS
        .iter()
        .filter(|c| mask & **c != 0)
        .filter_map(|c| {
            cards
                .iter()
                .position(|k| k.name == crate::world::Terrain::of_colour(*c).land_name())
        })
        .collect();
    if names.is_empty() {
        return Vec::new();
    }
    (0..n).map(|i| names[i % names.len()]).collect()
}

/// `n` basic lands for a library search (Seek the Horizon, Gatecreeper Vine):
/// the colours the cards in hand and library ask for most, by turns.
pub fn basics_needed(cards: &[Card], hand: &[usize], library: &[usize], n: usize) -> Vec<usize> {
    let mut count = [0usize; 5];
    for &c in hand.iter().chain(library.iter()) {
        for (i, col) in mana::COLOURS.iter().enumerate() {
            if cards[c].cost.colours() & col != 0 {
                count[i] += 1;
            }
        }
    }
    let mut order: Vec<usize> = (0..5).filter(|&i| count[i] > 0).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(count[i]), i));
    if order.is_empty() {
        order = (0..5).collect();
    }
    (0..n)
        .filter_map(|k| {
            basics_for(cards, mana::COLOURS[order[k % order.len()]], 1)
                .first()
                .copied()
        })
        .collect()
}

/// The player's land pile: the lands within Reach, made up to `PILE_MIN`
/// with basics of the deck's colours, the colour the pile has least of first.
pub fn fill_pile(cards: &[Card], deck: &[usize], mut lands: Vec<usize>) -> Vec<usize> {
    let want: u8 = deck.iter().fold(0, |m, &c| m | cards[c].identity);
    let want = if want == 0 { mana::ANY } else { want };
    while lands.len() < PILE_MIN {
        let count = |colour: u8| {
            lands
                .iter()
                .filter(|&&l| rules::land_unit(&cards[l].name).unwrap_or(0) & colour != 0)
                .count()
        };
        let Some(&least) = mana::COLOURS
            .iter()
            .filter(|c| want & **c != 0)
            .min_by_key(|c| count(**c))
        else {
            break;
        };
        let Some(&land) = basics_for(cards, least, 1).first() else {
            break;
        };
        lands.push(land);
    }
    lands
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

/// Five more decks for each guild (T25), each rebuilt from this region's
/// cards after a deck Wizards of the Coast sold for that guild: the deck's
/// plan and, where the region has it, its face card. Twelve cards, at most
/// one rare or mythic, and tested to hold up like the guild's first deck
/// (SECOND-ORDER R7.3). Names and the products they follow are strings
/// `starter.<guild>.<id>`.
pub const MORE_STARTERS: [(Guild, &str, [&str; STARTER_DECK]); 25] = [
    (
        Guild::Azorius,
        "advance",
        [
            "Azorius Arrester",
            "Azorius Arrester",
            "Concordia Pegasus",
            "Sunspire Griffin",
            "Tower Drake",
            "Lyev Skyknight",
            "Hussar Patrol",
            "Skymark Roc",
            "Archon of the Triumvirate",
            "Inaction Injunction",
            "Arrest",
            "Azorius Charm",
        ],
    ),
    (
        Guild::Azorius,
        "ascendant",
        [
            "Judge's Familiar",
            "Vassal Soul",
            "Vassal Soul",
            "Tower Drake",
            "Concordia Pegasus",
            "Sunspire Griffin",
            "Runewing",
            "Skymark Roc",
            "Isperia's Skywatch",
            "Eyes in the Skies",
            "Avenging Arrow",
            "Arrest",
        ],
    ),
    (
        Guild::Azorius,
        "authority",
        [
            "Azorius Arrester",
            "Azorius Arrester",
            "Lyev Skyknight",
            "New Prahv Guildmage",
            "Hussar Patrol",
            "Doorkeeper",
            "Armory Guard",
            "Stealer of Secrets",
            "Azor's Elocutors",
            "Inaction Injunction",
            "Paralyzing Grasp",
            "Dramatic Rescue",
        ],
    ),
    (
        Guild::Azorius,
        "kit",
        [
            "Judge's Familiar",
            "Faerie Impostor",
            "Tower Drake",
            "Lyev Skyknight",
            "Concordia Pegasus",
            "Vassal Soul",
            "Skymark Roc",
            "Sunspire Griffin",
            "Hover Barrier",
            "Syncopate",
            "Azorius Charm",
            "Detention Sphere",
        ],
    ),
    (
        Guild::Azorius,
        "dovin",
        [
            "Azorius Arrester",
            "Keening Apparition",
            "Fencing Ace",
            "Seller of Songbirds",
            "Azorius Justiciar",
            "Lyev Skyknight",
            "Skymark Roc",
            "Sphinx of the Chimes",
            "Inaction Injunction",
            "Swift Justice",
            "Downsize",
            "Arrest",
        ],
    ),
    (
        Guild::Izzet,
        "ingenuity",
        [
            "Goblin Electromancer",
            "Guttersnipe",
            "Frostburn Weird",
            "Blistercoil Weird",
            "Splatter Thug",
            "Stealer of Secrets",
            "Hypersonic Dragon",
            "Annihilating Fire",
            "Electrickery",
            "Izzet Charm",
            "Inspiration",
            "Teleportal",
        ],
    ),
    (
        Guild::Izzet,
        "gizmometry",
        [
            "Blistercoil Weird",
            "Frostburn Weird",
            "Izzet Staticaster",
            "Nivix Guildmage",
            "Lobber Crew",
            "Crosstown Courier",
            "Volatile Rig",
            "Izzet Keyrune",
            "Civic Saber",
            "Chemister's Trick",
            "Electrickery",
            "Dynacharge",
        ],
    ),
    (
        Guild::Izzet,
        "duel",
        [
            "Goblin Electromancer",
            "Frostburn Weird",
            "Frostburn Weird",
            "Nivmagus Elemental",
            "Viashino Racketeer",
            "Runewing",
            "Bellows Lizard",
            "Annihilating Fire",
            "Izzet Charm",
            "Essence Backlash",
            "Thoughtflare",
            "Dispel",
        ],
    ),
    (
        Guild::Izzet,
        "kit",
        [
            "Crosstown Courier",
            "Goblin Electromancer",
            "Izzet Staticaster",
            "Bellows Lizard",
            "Viashino Racketeer",
            "Mercurial Chemister",
            "Frostburn Weird",
            "Stealer of Secrets",
            "Chemister's Trick",
            "Electrickery",
            "Teleportal",
            "Inspiration",
        ],
    ),
    (
        Guild::Izzet,
        "ral",
        [
            "Blistercoil Weird",
            "Crosstown Courier",
            "Viashino Racketeer",
            "Splatter Thug",
            "Stealer of Secrets",
            "Cobblebrute",
            "Runewing",
            "Batterhorn",
            "Izzet Staticaster",
            "Annihilating Fire",
            "Mizzium Skin",
            "Izzet Charm",
        ],
    ),
    (
        Guild::Rakdos,
        "raid",
        [
            "Rakdos Cackler",
            "Rakdos Cackler",
            "Rakdos Shred-Freak",
            "Gore-House Chainwalker",
            "Dead Reveler",
            "Grim Roustabout",
            "Hellhole Flailer",
            "Spawn of Rix Maadi",
            "Carnival Hellsteed",
            "Auger Spree",
            "Skull Rend",
            "Annihilating Fire",
        ],
    ),
    (
        Guild::Rakdos,
        "wrack",
        [
            "Rakdos Cackler",
            "Rakdos Cackler",
            "Rakdos Shred-Freak",
            "Thrill-Kill Assassin",
            "Gore-House Chainwalker",
            "Splatter Thug",
            "Hellhole Flailer",
            "Rix Maadi Guildmage",
            "Dreadbore",
            "Rakdos Charm",
            "Auger Spree",
            "Electrickery",
        ],
    ),
    (
        Guild::Rakdos,
        "bloodsport",
        [
            "Drainpipe Vermin",
            "Bellows Lizard",
            "Rakdos Shred-Freak",
            "Gore-House Chainwalker",
            "Daggerdrome Imp",
            "Sewer Shambler",
            "Bloodfray Giant",
            "Slum Reaper",
            "Rakdos Ragemutt",
            "Mind Rot",
            "Deviant Glee",
            "Launch Party",
        ],
    ),
    (
        Guild::Rakdos,
        "revelry",
        [
            "Rakdos Cackler",
            "Gore-House Chainwalker",
            "Grim Roustabout",
            "Splatter Thug",
            "Dead Reveler",
            "Thrill-Kill Assassin",
            "Rakdos Ragemutt",
            "Rakdos Ringleader",
            "Chaos Imps",
            "Rakdos Charm",
            "Annihilating Fire",
            "Racecourse Fury",
        ],
    ),
    (
        Guild::Rakdos,
        "kit",
        [
            "Bellows Lizard",
            "Rakdos Cackler",
            "Daggerdrome Imp",
            "Tavern Swindler",
            "Viashino Racketeer",
            "Hellhole Flailer",
            "Spawn of Rix Maadi",
            "Electrickery",
            "Auger Spree",
            "Stab Wound",
            "Ultimate Price",
            "Rakdos Keyrune",
        ],
    ),
    (
        Guild::Golgari,
        "growth",
        [
            "Drudge Beetle",
            "Korozda Monitor",
            "Korozda Monitor",
            "Golgari Longlegs",
            "Sluiceway Scorpion",
            "Trestle Troll",
            "Dead Reveler",
            "Gobbling Ooze",
            "Corpsejack Menace",
            "Slime Molding",
            "Aerial Predation",
            "Giant Growth",
        ],
    ),
    (
        Guild::Golgari,
        "creep",
        [
            "Slitherhead",
            "Drudge Beetle",
            "Dreg Mangler",
            "Dreg Mangler",
            "Korozda Guildmage",
            "Sluiceway Scorpion",
            "Stonefare Crocodile",
            "Ogre Jailbreaker",
            "Lotleth Troll",
            "Golgari Charm",
            "Ultimate Price",
            "Grisly Salvage",
        ],
    ),
    (
        Guild::Golgari,
        "deathcreep",
        [
            "Drainpipe Vermin",
            "Slitherhead",
            "Golgari Decoy",
            "Dead Reveler",
            "Catacomb Slug",
            "Trestle Troll",
            "Golgari Longlegs",
            "Dreg Mangler",
            "Korozda Monitor",
            "Grisly Salvage",
            "Ultimate Price",
            "Treasured Find",
        ],
    ),
    (
        Guild::Golgari,
        "kit",
        [
            "Drudge Beetle",
            "Drudge Beetle",
            "Sluiceway Scorpion",
            "Gatecreeper Vine",
            "Korozda Guildmage",
            "Dreg Mangler",
            "Rubbleback Rhino",
            "Golgari Longlegs",
            "Treasured Find",
            "Grisly Salvage",
            "Golgari Charm",
            "Abrupt Decay",
        ],
    ),
    (
        Guild::Golgari,
        "vraska",
        [
            "Grim Roustabout",
            "Thrill-Kill Assassin",
            "Tavern Swindler",
            "Dead Reveler",
            "Korozda Monitor",
            "Dreg Mangler",
            "Slum Reaper",
            "Golgari Longlegs",
            "Vraska the Unseen",
            "Ultimate Price",
            "Assassin's Strike",
            "Stab Wound",
        ],
    ),
    (
        Guild::Selesnya,
        "surge",
        [
            "Seller of Songbirds",
            "Trained Caracal",
            "Dryad Militant",
            "Centaur Healer",
            "Selesnya Sentry",
            "Towering Indrik",
            "Rubbleback Rhino",
            "Giant Growth",
            "Common Bond",
            "Rootborn Defenses",
            "Selesnya Charm",
            "Heroes' Reunion",
        ],
    ),
    (
        Guild::Selesnya,
        "united",
        [
            "Centaur's Herald",
            "Seller of Songbirds",
            "Vitu-Ghazi Guildmage",
            "Armory Guard",
            "Centaur Healer",
            "Gatecreeper Vine",
            "Call of the Conclave",
            "Call of the Conclave",
            "Coursers' Accord",
            "Druid's Deliverance",
            "Rootborn Defenses",
            "Eyes in the Skies",
        ],
    ),
    (
        Guild::Selesnya,
        "kit",
        [
            "Trostani, Selesnya's Voice",
            "Seller of Songbirds",
            "Centaur Healer",
            "Dryad Militant",
            "Oak Street Innkeeper",
            "Vitu-Ghazi Guildmage",
            "Concordia Pegasus",
            "Call of the Conclave",
            "Common Bond",
            "Selesnya Charm",
            "Eyes in the Skies",
            "Knightly Valor",
        ],
    ),
    (
        Guild::Selesnya,
        "tokens",
        [
            "Trained Caracal",
            "Seller of Songbirds",
            "Keening Apparition",
            "Precinct Captain",
            "Centaur's Herald",
            "Brushstrider",
            "Call of the Conclave",
            "Call of the Conclave",
            "Eyes in the Skies",
            "Coursers' Accord",
            "Common Bond",
            "Giant Growth",
        ],
    ),
    (
        Guild::Selesnya,
        "zendikar",
        [
            "Drudge Beetle",
            "Brushstrider",
            "Dryad Militant",
            "Selesnya Sentry",
            "Loxodon Smiter",
            "Towering Indrik",
            "Rubbleback Rhino",
            "Korozda Monitor",
            "Arrest",
            "Savage Surge",
            "Chorus of Might",
            "Giant Growth",
        ],
    ),
];

/// The decks a guild can start with: its first deck ("first"), then the five after Wizards' decks.
pub fn starter_ids(g: Guild) -> Vec<&'static str> {
    std::iter::once("first")
        .chain(
            MORE_STARTERS
                .iter()
                .filter(|(k, _, _)| *k == g)
                .map(|(_, id, _)| *id),
        )
        .collect()
}

/// One of a guild's starting decks by its id; "first" is the guild's first deck.
pub fn starter_named(cards: &[Card], g: Guild, id: &str) -> Vec<usize> {
    let Some((_, _, names)) = MORE_STARTERS.iter().find(|(k, i, _)| *k == g && *i == id) else {
        return starter_deck(cards, g);
    };
    names
        .iter()
        .filter_map(|n| cards.iter().position(|c| c.name == *n))
        .collect()
}

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
    let guilds: &[&str] = match city {
        City::Tenth => &["Rakdos", "Golgari"],
        City::Prahv => &["Azorius", "Izzet"],
        City::Vitu => &["Selesnya", "Golgari"],
        City::Rix => &["Rakdos"],
        City::Svogthos => &["Golgari"],
    };
    // a city's own pieces are sold there and nowhere else
    armoury
        .pieces
        .iter()
        .filter(|p| p.city == city.id() || (p.city.is_empty() && guilds.contains(&p.guild.as_str())))
        .map(|p| p.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::for_tests;
    use crate::duel::{Duel, Outcome, Setup};
    use crate::player::parse_armour;

    #[test]
    fn a_foe_brings_its_creature_in_play_and_life_by_tier() {
        let cards = for_tests();
        let mangler = crate::content::find(&cards, "Dreg Mangler");
        let f = foe(&cards, mangler, &[], &mut Rng::new(1));
        assert_eq!(f.in_play, vec![mangler]);
        // mana value 3: tier 2, 12 life, five lands: Swamps and Forests by turns
        assert_eq!(f.life, 12);
        let names: Vec<&str> = f.lands.iter().map(|&l| cards[l].name.as_str()).collect();
        assert_eq!(names, vec!["Swamp", "Forest", "Swamp", "Forest", "Swamp"]);
        assert_eq!(f.deck.len(), FOE_DECK);
        // twelve cards: enough that a foe does not run out before a fight is decided
        assert!(f
            .deck
            .iter()
            .all(|&i| cards[i].identity & !(crate::mana::B | crate::mana::G) == 0));
    }

    #[test]
    fn the_guardian_has_forty_blue_and_red_cards_and_niv_mizzet_in_play() {
        let cards = for_tests();
        let g = guardian(&cards);
        assert_eq!(g.deck.len(), 40);
        assert_eq!(g.life, GUARDIAN_LIFE);
        assert_eq!(g.lands.len(), GUARDIAN_LANDS);
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
    fn rakdos_has_forty_black_and_red_cards_twelve_life_and_the_column_takes_four() {
        let cards = for_tests();
        let g = rakdos(&cards, false);
        assert_eq!(g.deck.len(), 40);
        assert_eq!(g.life, 12);
        assert_eq!(rakdos(&cards, true).life, 8);
        assert_eq!(g.lands.len(), 7);
        assert!(g
            .deck
            .iter()
            .all(|&i| cards[i].identity & !(B | R) == 0 && cards[i].identity != 0));
        assert!(g
            .deck
            .iter()
            .all(|&i| g.deck.iter().filter(|&&j| j == i).count() <= 2));
        assert_eq!(cards[g.in_play[0]].name, "Rakdos, Lord of Riots");
        let names: Vec<&str> = g.lands.iter().map(|&l| cards[l].name.as_str()).collect();
        assert_eq!(names.iter().filter(|n| **n == "Rakdos Guildgate").count(), 2);
    }

    #[test]
    fn a_minion_fights_with_eighteen_life_and_seven_lands_whatever_its_size() {
        let cards = for_tests();
        let small = crate::content::find(&cards, "Splatter Thug");
        let f = minion(&cards, small, &mut Rng::new(3));
        assert_eq!(f.in_play, vec![small]);
        assert_eq!(f.life, 18);
        assert_eq!(f.lands.len(), 7);
        // a plain foe of the same card: tier two, 12 life, five lands
        assert_eq!(foe(&cards, small, &[], &mut Rng::new(3)).life, 12);
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
    fn each_guild_has_six_starting_decks_of_twelve_of_its_cards_with_one_rare_at_most() {
        let cards = for_tests();
        for g in Guild::ALL {
            let ids = starter_ids(g);
            assert_eq!(ids.len(), 6, "{g:?}");
            for id in ids {
                let d = starter_named(&cards, g, id);
                assert_eq!(
                    d.len(),
                    STARTER_DECK,
                    "{g:?} {id}: a name is not a card of the region"
                );
                assert!(
                    d.iter().all(|&i| cards[i].identity & !g.colours() == 0),
                    "{g:?} {id}: off colour"
                );
                let rares = d
                    .iter()
                    .filter(|&&i| matches!(cards[i].rarity, 'r' | 'm'))
                    .count();
                assert!(rares <= 1, "{g:?} {id}: {rares} rares");
                let creatures = d
                    .iter()
                    .filter(|&&i| cards[i].is(ty::CREATURE) || cards[i].is(ty::PLANESWALKER))
                    .count();
                assert!(creatures >= 6, "{g:?} {id}: {creatures} creatures");
                assert!(
                    d.iter().all(|&i| d.iter().filter(|&&j| j == i).count() <= 2),
                    "{g:?} {id}: three of a card"
                );
            }
        }
        // the first deck is the guild's own; an id that is not one gives it too
        let first = starter_deck(&cards, Guild::Rakdos);
        assert_eq!(starter_named(&cards, Guild::Rakdos, "first"), first);
        assert_ne!(starter_named(&cards, Guild::Rakdos, "raid"), first);
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
                lands: fill_pile(&cards, &deck, lands),
                deck,
                ..SideSetup::default()
            };
            let f = foe(&cards, caracal, &[], &mut Rng::new(seed));
            let mut d = Duel::new(
                std::rc::Rc::new(cards.clone()),
                &Setup { sides: [me, f], seed },
                [false, false],
            );
            d.run();
            if d.outcome == Some(Outcome::Won) {
                wins += 1;
            }
        }
        assert!(wins >= 24, "{wins} of 40");
    }

    #[test]
    fn a_short_land_pile_is_made_up_with_the_colour_it_lacks() {
        let cards = for_tests();
        let deck = starter_deck(&cards, Guild::Izzet);
        let gate = crate::content::find(&cards, "Izzet Guildgate");
        let pile = fill_pile(&cards, &deck, vec![gate]);
        let names: Vec<&str> = pile.iter().map(|&l| cards[l].name.as_str()).collect();
        // the gate counts for both; then Island, Mountain, Island, Mountain
        assert_eq!(
            names,
            vec!["Izzet Guildgate", "Island", "Mountain", "Island", "Mountain"]
        );
        let six = vec![gate; 6];
        assert_eq!(fill_pile(&cards, &deck, six.clone()), six);
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
