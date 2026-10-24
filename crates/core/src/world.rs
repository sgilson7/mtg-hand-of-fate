//! The map: a 2D grid of tiles, drawn in 3D but integer cells underneath
//! (MTG-2). It grows in chunks of 5×5. The first chunk is centred on the
//! Tenth District, where the player starts; stepping onto a tile at the edge
//! of the map adds the chunk beyond it (the person's rule: "if you get to an
//! edge, you add an additional 5x5 set of tiles"). The first region is the
//! 3×3 chunks around the start, 15×15 tiles; past its edge there is nothing
//! more until the region's guardian is beaten (a later region).
//!
//! A chunk depends only on the seed and its position, so the same seed
//! grows the same world in any order (tested below).
//!
//! Each chunk leans toward one guild's quarter: its terrain is mostly that
//! guild's two colours (Plains white, Island blue, Swamp black, Mountain
//! red, Forest green), and its districts are that guild's. Placement sets
//! each card tile's pool: the terrain's colour, or a district's guild.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::content::{ty, Card};
use crate::mana::{self, B, G, R, U, W};
use crate::rng::Rng;

pub const CHUNK: i32 = 5;
/// Chunks from the start chunk to the region's edge.
pub const REGION: i32 = 1;
pub const POOL_SIZE: usize = 4;
/// Obstacles in a chunk (never on its border, so chunks always join up).
pub const OBSTACLES: usize = 2;
/// Lands a tile can hold; a city holds more.
pub const LANDS_PER_TILE: usize = 1;
pub const LANDS_PER_CITY: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Terrain {
    Plains,
    Island,
    Swamp,
    Mountain,
    Forest,
}

impl Terrain {
    pub const ALL: [Terrain; 5] = [
        Terrain::Plains,
        Terrain::Island,
        Terrain::Swamp,
        Terrain::Mountain,
        Terrain::Forest,
    ];
    pub fn colour(self) -> u8 {
        match self {
            Terrain::Plains => W,
            Terrain::Island => U,
            Terrain::Swamp => B,
            Terrain::Mountain => R,
            Terrain::Forest => G,
        }
    }
    pub fn of_colour(c: u8) -> Terrain {
        match c {
            W => Terrain::Plains,
            U => Terrain::Island,
            B => Terrain::Swamp,
            R => Terrain::Mountain,
            _ => Terrain::Forest,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Terrain::Plains => "plains",
            Terrain::Island => "island",
            Terrain::Swamp => "swamp",
            Terrain::Mountain => "mountain",
            Terrain::Forest => "forest",
        }
    }
    pub fn land_name(self) -> &'static str {
        match self {
            Terrain::Plains => "Plains",
            Terrain::Island => "Island",
            Terrain::Swamp => "Swamp",
            Terrain::Mountain => "Mountain",
            Terrain::Forest => "Forest",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Guild {
    Azorius,
    Izzet,
    Rakdos,
    Golgari,
    Selesnya,
}

impl Guild {
    pub const ALL: [Guild; 5] = [
        Guild::Azorius,
        Guild::Izzet,
        Guild::Rakdos,
        Guild::Golgari,
        Guild::Selesnya,
    ];
    pub fn colours(self) -> u8 {
        match self {
            Guild::Azorius => W | U,
            Guild::Izzet => U | R,
            Guild::Rakdos => B | R,
            Guild::Golgari => B | G,
            Guild::Selesnya => G | W,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Guild::Azorius => "Azorius",
            Guild::Izzet => "Izzet",
            Guild::Rakdos => "Rakdos",
            Guild::Golgari => "Golgari",
            Guild::Selesnya => "Selesnya",
        }
    }
    pub fn from_name(s: &str) -> Option<Guild> {
        Guild::ALL.into_iter().find(|g| g.name() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feature {
    /// A tile with a pool of cards to draw from.
    Cards,
    City(City),
    /// A peak: no one walks through it.
    Peak,
    /// A deep canal: no one walks through it.
    Canal,
    /// The guardian's lair.
    Lair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum City {
    Tenth,
    Prahv,
    Vitu,
    /// Rix Maadi, the Rakdos city (T27).
    Rix,
    /// Svogthos, the Golgari city (T27).
    Svogthos,
}

impl City {
    pub const ALL: [City; 5] = [City::Tenth, City::Prahv, City::Vitu, City::Rix, City::Svogthos];
    pub fn id(self) -> &'static str {
        match self {
            City::Tenth => "tenth",
            City::Prahv => "prahv",
            City::Vitu => "vitu",
            City::Rix => "rix",
            City::Svogthos => "svogthos",
        }
    }
    pub fn from_id(s: &str) -> Option<City> {
        City::ALL.into_iter().find(|c| c.id() == s)
    }
    pub fn guild(self) -> Option<Guild> {
        match self {
            City::Tenth => None,
            City::Prahv => Some(Guild::Azorius),
            City::Vitu => Some(Guild::Selesnya),
            City::Rix => Some(Guild::Rakdos),
            City::Svogthos => Some(Guild::Golgari),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub terrain: Terrain,
    pub guild: Option<Guild>,
    pub feature: Feature,
    pub pool: Vec<usize>,
    pub visited: bool,
    pub cleared: bool,
    /// Land cards attached here.
    pub lands: Vec<usize>,
}

impl Tile {
    pub fn passable(&self) -> bool {
        !matches!(self.feature, Feature::Peak | Feature::Canal)
    }
    pub fn land_room(&self) -> usize {
        match self.feature {
            Feature::City(_) => LANDS_PER_CITY,
            Feature::Peak | Feature::Canal => 0,
            _ => LANDS_PER_TILE,
        }
    }
    /// The colours this tile's pool draws from.
    pub fn colours(&self) -> u8 {
        self.guild.map(|g| g.colours()).unwrap_or(self.terrain.colour())
    }
}

pub type Pos = (i32, i32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    pub seed: u64,
    pub chunks: BTreeSet<Pos>,
    pub tiles: BTreeMap<Pos, Tile>,
}

/// What a chunk leans to, and what fixed place sits at its centre.
pub fn chunk_plan(c: Pos) -> (Option<Guild>, Option<Feature>) {
    match c {
        (0, 0) => (None, Some(Feature::City(City::Tenth))),
        (0, -1) => (Some(Guild::Azorius), Some(Feature::City(City::Prahv))),
        (1, -1) => (Some(Guild::Izzet), Some(Feature::Lair)),
        (1, 0) => (Some(Guild::Izzet), None),
        (1, 1) => (Some(Guild::Rakdos), None),
        (0, 1) => (Some(Guild::Rakdos), Some(Feature::City(City::Rix))),
        (-1, 1) => (Some(Guild::Golgari), Some(Feature::City(City::Svogthos))),
        (-1, 0) => (Some(Guild::Selesnya), Some(Feature::City(City::Vitu))),
        (-1, -1) => (Some(Guild::Selesnya), None),
        _ => (None, None),
    }
}

pub fn chunk_of(p: Pos) -> Pos {
    // chunk (0,0) spans -2..=2
    let f = |v: i32| (v + CHUNK / 2).div_euclid(CHUNK);
    (f(p.0), f(p.1))
}

pub fn chunk_centre(c: Pos) -> Pos {
    (c.0 * CHUNK, c.1 * CHUNK)
}

pub fn in_region(c: Pos) -> bool {
    c.0.abs() <= REGION && c.1.abs() <= REGION
}

pub fn ring(c: Pos) -> i32 {
    c.0.abs().max(c.1.abs())
}

/// Tiles from the start to the edge of each band of difficulty (T20).
pub const BAND: i32 = 5;

/// One tile in this many wears its terrain's second look (T24).
pub const ALTERNATE_ONE_IN: u64 = 3;

/// Whether the tile at p wears its terrain's second look: fixed by the seed
/// and the place, so a tile keeps its look for the whole game.
pub fn alternate(seed: u64, p: Pos) -> bool {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    for v in [p.0, p.1] {
        h = (h ^ v as u32 as u64).wrapping_mul(0x100_0000_01B3);
        h ^= h >> 29;
    }
    h.is_multiple_of(ALTERNATE_ONE_IN)
}

/// Which band of difficulty a tile is in: 0 within `BAND` tiles of the start
/// (as the crow flies), 1 within twice that, and so on outward.
pub fn band(p: Pos) -> i32 {
    let d2 = p.0 as i64 * p.0 as i64 + p.1 as i64 * p.1 as i64;
    // the whole square root: the largest r with r * r <= d2
    let mut r: i64 = 0;
    while (r + 1) * (r + 1) <= d2 {
        r += 1;
    }
    r as i32 / BAND
}

pub fn distance(a: Pos, b: Pos) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn mix(seed: u64, c: Pos) -> u64 {
    let x = (c.0 as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (c.1 as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    seed ^ x.rotate_left(17)
}

/// Whether a card may appear in a tile's pool. `ring` is the chunk's ring
/// (colourless cards stay out of the start chunk); `band` the tile's band of
/// difficulty: the creatures met grow with it (T20).
fn fits(card: &Card, tile: &Tile, ring: i32, band: i32, lair_chunk: bool) -> bool {
    if card.name == "Niv-Mizzet, Dracogenius" {
        return false;
    }
    if card.is(ty::LAND) {
        return match tile.guild {
            Some(g) => {
                card.identity == g.colours()
                    || (card.identity & g.colours() != 0 && mana::count_colours(card.identity) == 1)
            }
            None => card.identity == tile.terrain.colour() && mana::count_colours(card.identity) == 1,
        };
    }
    let colours = tile.colours();
    let id = card.identity;
    let ok = match tile.guild {
        Some(g) => id != 0 && id & !g.colours() == 0,
        None => id == colours,
    } || (id == 0 && ring >= 1);
    if !ok {
        return false;
    }
    // the lair's chunk holds the big cards, but not inside the first band
    if lair_chunk && band >= 1 {
        return true;
    }
    let creature = card.is(ty::CREATURE);
    match band {
        0 => card.value <= 2 && matches!(card.rarity, 'c' | 'u'),
        1 if creature => (2..=4).contains(&card.value),
        1 => card.value <= 5,
        _ if creature => card.value >= 3,
        _ => true,
    }
}

impl World {
    pub fn new(seed: u64, cards: &[Card]) -> World {
        let mut w = World {
            seed,
            chunks: BTreeSet::new(),
            tiles: BTreeMap::new(),
        };
        w.add_chunk((0, 0), cards);
        if let Some(t) = w.tiles.get_mut(&(0, 0)) {
            t.visited = true;
        }
        w
    }

    pub fn tile(&self, p: Pos) -> Option<&Tile> {
        self.tiles.get(&p)
    }

    /// Adds a chunk if it is in the region and not made yet. True if added.
    pub fn add_chunk(&mut self, c: Pos, cards: &[Card]) -> bool {
        if !in_region(c) || self.chunks.contains(&c) {
            return false;
        }
        let tiles = generate(self.seed, c, cards);
        for (p, t) in tiles {
            self.tiles.insert(p, t);
        }
        self.chunks.insert(c);
        true
    }

    /// Stepping onto `p`: any chunk beyond an edge it touches is added.
    /// Returns the chunks added.
    pub fn grow_from(&mut self, p: Pos, cards: &[Card]) -> Vec<Pos> {
        let mut added = Vec::new();
        for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (p.0 + d.0, p.1 + d.1);
            if !self.tiles.contains_key(&n) {
                let c = chunk_of(n);
                if self.add_chunk(c, cards) {
                    added.push(c);
                }
            }
        }
        added
    }

    /// Where a city or the lair is, if its chunk exists.
    /// A game saved before a chunk's city was planned gets the city now, at
    /// the chunk's centre, with the lands attached there kept (T27).
    pub fn settle_cities(&mut self) {
        for &c in &self.chunks {
            let (_, Some(f @ Feature::City(_))) = chunk_plan(c) else {
                continue;
            };
            if let Some(t) = self.tiles.get_mut(&chunk_centre(c)) {
                if t.feature != f {
                    t.feature = f;
                    t.pool.clear();
                }
            }
        }
    }

    pub fn find(&self, f: Feature) -> Option<Pos> {
        self.tiles.iter().find(|(_, t)| t.feature == f).map(|(p, _)| *p)
    }

    /// Land units in Reach of a tile: one unit a land.
    pub fn lands_in_reach(&self, at: Pos, reach: i32) -> Vec<(Pos, usize)> {
        self.tiles
            .iter()
            .filter(|(p, _)| distance(**p, at) <= reach)
            .flat_map(|(p, t)| t.lands.iter().map(move |&l| (*p, l)))
            .collect()
    }
}

/// Makes one chunk's 25 tiles.
pub fn generate(seed: u64, c: Pos, cards: &[Card]) -> Vec<(Pos, Tile)> {
    let mut rng = Rng::new(mix(seed, c));
    let (guild, special) = chunk_plan(c);
    let lean = guild.map(|g| g.colours()).unwrap_or(0);
    let centre = chunk_centre(c);
    let half = CHUNK / 2;
    // terrain: mostly the guild's two colours
    let mut terrain = BTreeMap::new();
    for dy in -half..=half {
        for dx in -half..=half {
            let roll = rng.next_u64() % 100;
            let t = if lean != 0 && roll < 75 {
                let both: Vec<u8> = mana::COLOURS.iter().copied().filter(|k| lean & k != 0).collect();
                both[(roll % 2) as usize]
            } else {
                mana::COLOURS[(rng.next_u64() % 5) as usize]
            };
            terrain.insert((dx, dy), Terrain::of_colour(t));
        }
    }
    // one pass of smoothing: a tile surrounded mostly by one terrain joins it
    let before = terrain.clone();
    for (&(dx, dy), t) in terrain.iter_mut() {
        let mut counts: BTreeMap<Terrain, u32> = BTreeMap::new();
        for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if let Some(n) = before.get(&(dx + d.0, dy + d.1)) {
                *counts.entry(*n).or_insert(0) += 1;
            }
        }
        if let Some((n, k)) = counts.into_iter().max_by_key(|(n, k)| (*k, *n)) {
            if k >= 3 {
                *t = n;
            }
        }
    }
    let mut tiles: BTreeMap<Pos, Tile> = terrain
        .into_iter()
        .map(|(d, terrain)| {
            (
                d,
                Tile {
                    terrain,
                    guild: None,
                    feature: Feature::Cards,
                    pool: Vec::new(),
                    visited: false,
                    cleared: false,
                    lands: Vec::new(),
                },
            )
        })
        .collect();
    if let Some(f) = special {
        if let Some(t) = tiles.get_mut(&(0, 0)) {
            t.feature = f;
            if f == Feature::Lair {
                t.terrain = Terrain::Mountain;
            }
        }
    }
    // obstacles inside the border, off the centre
    let inner: Vec<Pos> = tiles
        .keys()
        .copied()
        .filter(|d| d.0.abs() < half && d.1.abs() < half && *d != (0, 0))
        .collect();
    let mut placed = 0;
    let mut tries = 0;
    while placed < OBSTACLES && tries < 40 {
        tries += 1;
        let d = inner[(rng.next_u64() % inner.len() as u64) as usize];
        let t = tiles.get_mut(&d).expect("inner tile");
        if t.feature != Feature::Cards {
            continue;
        }
        t.feature = if t.terrain == Terrain::Island {
            Feature::Canal
        } else {
            Feature::Peak
        };
        if t.feature == Feature::Peak {
            t.terrain = Terrain::Mountain;
        }
        placed += 1;
    }
    // districts: two in a guild's chunk, one of a random guild at the start
    let districts = if guild.is_some() { 2 } else { 1 };
    let mut made = 0;
    tries = 0;
    while made < districts && tries < 60 {
        tries += 1;
        let keys: Vec<Pos> = tiles.keys().copied().collect();
        let d = keys[(rng.next_u64() % keys.len() as u64) as usize];
        let near_special = d.0.abs() + d.1.abs() <= 1 && special.is_some();
        let crowded = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|o| {
            tiles
                .get(&(d.0 + o.0, d.1 + o.1))
                .map(|t| t.guild.is_some())
                .unwrap_or(false)
        });
        let t = tiles.get_mut(&d).expect("tile");
        if t.feature != Feature::Cards || t.guild.is_some() || crowded || near_special {
            continue;
        }
        let g = guild.unwrap_or(Guild::ALL[(rng.next_u64() % 5) as usize]);
        t.guild = Some(g);
        // a district stands on one of its guild's terrains
        if g.colours() & t.terrain.colour() == 0 {
            let both: Vec<u8> = mana::COLOURS
                .iter()
                .copied()
                .filter(|k| g.colours() & k != 0)
                .collect();
            t.terrain = Terrain::of_colour(both[(rng.next_u64() % 2) as usize]);
        }
        made += 1;
    }
    // pools
    let r = ring(c);
    let lair_chunk = special == Some(Feature::Lair);
    for (p, t) in tiles.iter_mut() {
        if t.feature != Feature::Cards {
            continue;
        }
        // the tile's place in the world: the chunk's centre and its offset
        let b = band((centre.0 + p.0, centre.1 + p.1));
        let fit: Vec<usize> = (0..cards.len())
            .filter(|&i| fits(&cards[i], t, r, b, lair_chunk))
            .collect();
        let (lands, rest): (Vec<usize>, Vec<usize>) = fit.into_iter().partition(|&i| cards[i].is(ty::LAND));
        let mut pool = Vec::new();
        let mut rest = rest;
        while pool.len() < POOL_SIZE - 1 && !rest.is_empty() {
            let k = (rng.next_u64() % rest.len() as u64) as usize;
            pool.push(rest.remove(k));
        }
        if !lands.is_empty() && rng.next_u64() % 100 < 45 {
            pool.push(lands[(rng.next_u64() % lands.len() as u64) as usize]);
        } else if !rest.is_empty() {
            let k = (rng.next_u64() % rest.len() as u64) as usize;
            pool.push(rest.remove(k));
        }
        t.pool = pool;
    }
    tiles
        .into_iter()
        .map(|(d, t)| ((centre.0 + d.0, centre.1 + d.1), t))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::for_tests;

    #[test]
    fn the_start_is_a_visited_city_at_the_centre_of_the_first_chunk() {
        let cards = for_tests();
        let w = World::new(7, &cards);
        assert_eq!(w.tiles.len(), 25);
        let t = w.tile((0, 0)).unwrap();
        assert_eq!(t.feature, Feature::City(City::Tenth));
        assert!(t.visited);
        assert!(w.tile((2, 2)).is_some() && w.tile((3, 0)).is_none());
    }

    #[test]
    fn stepping_onto_an_edge_adds_the_chunk_beyond_it_once() {
        let cards = for_tests();
        let mut w = World::new(7, &cards);
        // (2, 0) is on the east edge of the first chunk
        assert_eq!(w.grow_from((2, 0), &cards), vec![(1, 0)]);
        assert_eq!(w.tiles.len(), 50);
        assert!(w.grow_from((2, 0), &cards).is_empty());
        // a corner touches two edges
        assert_eq!(w.grow_from((-2, -2), &cards).len(), 2);
    }

    #[test]
    fn the_region_stops_after_three_by_three_chunks() {
        let cards = for_tests();
        let mut w = World::new(7, &cards);
        for c in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            w.add_chunk(c, &cards);
        }
        assert_eq!(w.tiles.len(), 225);
        // (7, 0) is the region's east edge: nothing beyond it
        assert!(w.grow_from((7, 0), &cards).is_empty());
        assert!(!w.add_chunk((2, 0), &cards));
    }

    #[test]
    fn a_chunk_is_the_same_whatever_order_it_is_made_in() {
        let cards = for_tests();
        let mut a = World::new(11, &cards);
        a.add_chunk((1, 0), &cards);
        a.add_chunk((1, 1), &cards);
        let mut b = World::new(11, &cards);
        b.add_chunk((1, 1), &cards);
        b.add_chunk((1, 0), &cards);
        assert_eq!(a.tiles, b.tiles);
        // two chunks with the same plan still differ: each has its own chance
        // (beyond the region, where no guild or place is planned; since T27
        // the Rakdos quarter's two chunks differ by Rix Maadi alone)
        assert_eq!(chunk_plan((2, 0)), chunk_plan((2, 1)));
        let one = generate(11, (2, 0), &cards);
        let other = generate(11, (2, 1), &cards);
        let layout = |t: &[(Pos, Tile)]| {
            t.iter()
                .map(|(_, t)| (t.terrain, t.feature, t.guild))
                .collect::<Vec<_>>()
        };
        assert_ne!(layout(&one), layout(&other));
    }

    #[test]
    fn a_pool_draws_from_its_tiles_colours_and_the_start_holds_cheap_cards() {
        let cards = for_tests();
        let mut w = World::new(3, &cards);
        for c in [(1, -1), (1, 0)] {
            w.add_chunk(c, &cards);
        }
        for (p, t) in &w.tiles {
            for &i in &t.pool {
                let card = &cards[i];
                let colours = t.colours();
                assert!(card.identity & !colours == 0, "{} at {p:?}", card.name);
                if band(*p) == 0 && !card.is(ty::LAND) {
                    assert!(card.value <= 2, "{} near the start", card.name);
                }
                if band(*p) == 1
                    && card.is(ty::CREATURE)
                    && w.find(Feature::Lair).map(chunk_of) != Some(chunk_of(*p))
                {
                    assert!(
                        (2..=4).contains(&card.value),
                        "{} at {p:?}, the second band",
                        card.name
                    );
                }
                assert_ne!(card.name, "Niv-Mizzet, Dracogenius");
            }
        }
        // the lair is in the north-east chunk
        assert_eq!(w.find(Feature::Lair), Some((5, -5)));
    }

    #[test]
    fn the_bands_of_difficulty_are_rings_five_tiles_wide_around_the_start() {
        assert_eq!(band((0, 0)), 0);
        assert_eq!(band((4, 0)), 0);
        assert_eq!(band((3, 3)), 0); // 4.2 tiles away
        assert_eq!(band((5, 0)), 1);
        assert_eq!(band((4, 3)), 1); // 5 exactly
        assert_eq!(band((-7, -7)), 1); // 9.9
        assert_eq!(band((0, 10)), 2);
    }

    #[test]
    fn obstacles_never_sit_on_a_chunk_border() {
        let cards = for_tests();
        for seed in 0..20 {
            let w = World::new(seed, &cards);
            for (p, t) in &w.tiles {
                if !t.passable() {
                    assert!(p.0.abs() < 2 && p.1.abs() < 2, "{p:?} seed {seed}");
                }
            }
        }
    }

    #[test]
    fn reach_counts_steps_along_the_grid() {
        let cards = for_tests();
        let mut w = World::new(1, &cards);
        let plains = cards.iter().position(|c| c.name == "Plains").unwrap();
        w.tiles.get_mut(&(1, 1)).unwrap().lands.push(plains);
        w.tiles.get_mut(&(2, 2)).unwrap().lands.push(plains);
        // from (0, 0): (1, 1) is 2 steps, (2, 2) is 4
        assert_eq!(w.lands_in_reach((0, 0), 2).len(), 1);
        assert_eq!(w.lands_in_reach((0, 0), 4).len(), 2);
    }

    #[test]
    fn about_a_third_of_tiles_wear_the_second_look_and_keep_it() {
        let cards = crate::content::for_tests();
        let w = World::new(5, &cards);
        let mut by = std::collections::BTreeMap::new();
        for (&p, t) in &w.tiles {
            let e = by.entry(t.terrain).or_insert((0, 0));
            if alternate(w.seed, p) {
                e.0 += 1;
            }
            e.1 += 1;
            assert_eq!(alternate(w.seed, p), alternate(w.seed, p));
        }
        let (alt, all) = by.values().fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        assert!(alt * 4 > all && alt * 5 < all * 2, "{alt} of {all}");
        // the look follows the seed: another game lays it out differently
        assert!(w
            .tiles
            .keys()
            .any(|&p| alternate(w.seed, p) != alternate(w.seed + 1, p)));
    }

    #[test]
    fn rix_maadi_and_svogthos_stand_in_their_guilds_chunks_and_an_old_save_gets_them() {
        let cards = crate::content::for_tests();
        let mut w = World::new(9, &cards);
        for c in [(0, 1), (-1, 1)] {
            w.add_chunk(c, &cards);
        }
        assert_eq!(
            w.tile(chunk_centre((0, 1))).unwrap().feature,
            Feature::City(City::Rix)
        );
        assert_eq!(
            w.tile(chunk_centre((-1, 1))).unwrap().feature,
            Feature::City(City::Svogthos)
        );
        assert_eq!(City::Rix.guild(), Some(Guild::Rakdos));
        // a save from before T27: the centre was an ordinary tile
        let at = chunk_centre((0, 1));
        let t = w.tiles.get_mut(&at).unwrap();
        t.feature = Feature::Cards;
        t.pool = vec![0];
        w.settle_cities();
        let t = w.tile(at).unwrap();
        assert_eq!(t.feature, Feature::City(City::Rix));
        assert!(t.pool.is_empty());
    }
}
