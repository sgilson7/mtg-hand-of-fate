//! The second mission, the Riot (T32): Rakdos, Lord of Riots, leaves Rix
//! Maadi for New Prahv to tear down the Azorius Senate. He walks a route
//! through the region fixed by the seed, one tile each few of the player's
//! steps, and each tile he comes to is razed: its pool is gone and one of
//! his minions holds it, a hard fight. Beat him before he reaches New Prahv;
//! the grade is how few steps it took. Two places of this mission only, the
//! Calliope and the Lyev Column (T34), hold quests that slow and weaken him.

use serde::{Deserialize, Serialize};

use crate::content::{ty, Card};
use crate::mana::{B, R};
use crate::world::{chunk_centre, chunk_of, chunk_plan, Pos};

/// Player steps to each of his, and to each once the Calliope is silenced.
pub const EVERY: u32 = 3;
pub const EVERY_SLOWED: u32 = 4;
/// Where he sets out from (Rix Maadi) and what he means to raze (New Prahv).
pub const START: Pos = (0, 5);
pub const TARGET: Pos = (0, -5);
/// The chunks his route passes through, a tile of each chosen by the seed.
pub const WAYPOINTS: [Pos; 6] = [(1, 1), (1, 0), (0, 0), (-1, 1), (-1, 0), (-1, -1)];
/// Steps within which each grade is earned; past the last, the last grade.
pub const GRADE_STEPS: [u32; 3] = [70, 100, 130];
pub const GRADES: [&str; 4] = ["a", "b", "c", "d"];
/// A minion is a black or red creature of this mana value or more.
pub const MINION_VALUE: [usize; 2] = [3, 5];

/// The two places of the Riot (T34).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Place {
    /// The Rakdos calliope, playing the riot on.
    Calliope,
    /// An Azorius arresters' post.
    Column,
}

impl Place {
    pub const ALL: [Place; 2] = [Place::Calliope, Place::Column];
    pub fn id(self) -> &'static str {
        match self {
            Place::Calliope => "calliope",
            Place::Column => "column",
        }
    }
    pub fn from_id(s: &str) -> Option<Place> {
        Place::ALL.into_iter().find(|p| p.id() == s)
    }
    /// Where it stands: in the Rakdos quarter, and in the Selesnya quarter
    /// north-west, off its chunk's centre.
    pub fn spot(self) -> Pos {
        match self {
            Place::Calliope => (6, 4),
            Place::Column => (-6, -4),
        }
    }
    pub fn model(self) -> &'static str {
        match self {
            Place::Calliope => "rakdos_calliope",
            Place::Column => "lyev_column",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ending {
    Beaten {
        steps: u32,
    },
    /// He reached New Prahv.
    Fell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Riot {
    pub route: Vec<Pos>,
    /// Where on the route he stands.
    pub at: usize,
    /// The player's steps since his last.
    pub clock: u32,
    pub slowed: bool,
    pub weakened: bool,
    /// Minions left on tiles of chunks not made yet, placed when they are.
    pub pending: Vec<(Pos, usize)>,
    pub ended: Option<Ending>,
}

impl Riot {
    pub fn new(seed: u64) -> Riot {
        Riot {
            route: route(seed),
            at: 0,
            clock: 0,
            slowed: false,
            weakened: false,
            pending: Vec::new(),
            ended: None,
        }
    }
    pub fn pos(&self) -> Pos {
        self.route[self.at.min(self.route.len() - 1)]
    }
    /// His moves left before New Prahv.
    pub fn left(&self) -> usize {
        self.route.len() - 1 - self.at.min(self.route.len() - 1)
    }
    pub fn every(&self) -> u32 {
        if self.slowed {
            EVERY_SLOWED
        } else {
            EVERY
        }
    }
    pub fn roaming(&self) -> bool {
        self.ended.is_none()
    }
}

/// A city, the lair or a place of the Riot: he passes over them and stops on none.
pub fn fixed(p: Pos) -> bool {
    let c = chunk_of(p);
    (p == chunk_centre(c) && chunk_plan(c).1.is_some()) || Place::ALL.iter().any(|x| x.spot() == p)
}

fn hash(seed: u64, a: i64, b: i64) -> u64 {
    let mut h = seed ^ 0xA076_1D64_78BD_642F;
    for v in [a, b] {
        h = (h ^ v as u64).wrapping_mul(0x100_0000_01B3);
        h ^= h >> 31;
    }
    h
}

/// His road from Rix Maadi to New Prahv, a tile at a time: through a tile of
/// each waypoint chunk, along x and then along y, leaving out the tiles he
/// passes over.
pub fn route(seed: u64) -> Vec<Pos> {
    let mut stops = vec![START];
    for (i, &c) in WAYPOINTS.iter().enumerate() {
        let h = hash(seed, i as i64, 0);
        let centre = chunk_centre(c);
        let mut p = (centre.0 + (h % 5) as i32 - 2, centre.1 + (h / 5 % 5) as i32 - 2);
        if fixed(p) {
            p.0 += 1;
        }
        stops.push(p);
    }
    stops.push(TARGET);
    let mut out = vec![START];
    for w in stops.windows(2) {
        let (mut x, mut y) = w[0];
        while x != w[1].0 {
            x += (w[1].0 - x).signum();
            out.push((x, y));
        }
        while y != w[1].1 {
            y += (w[1].1 - y).signum();
            out.push((x, y));
        }
    }
    let mut kept: Vec<Pos> = Vec::new();
    for (i, p) in out.into_iter().enumerate() {
        if (i == 0 || p == TARGET || !fixed(p)) && kept.last() != Some(&p) {
            kept.push(p);
        }
    }
    kept
}

/// The minion he leaves at `p`: a black or red creature, chosen by the seed and the place.
pub fn minion_for(cards: &[Card], seed: u64, p: Pos) -> Option<usize> {
    let options: Vec<usize> = (0..cards.len())
        .filter(|&i| {
            let c = &cards[i];
            c.is(ty::CREATURE)
                && c.identity != 0
                && c.identity & !(B | R) == 0
                && (MINION_VALUE[0]..=MINION_VALUE[1]).contains(&c.value)
                && c.name != "Rakdos, Lord of Riots"
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    Some(options[(hash(seed, p.0 as i64, p.1 as i64) % options.len() as u64) as usize])
}

/// The grade for beating him in `steps` of the player's.
pub fn grade(steps: u32) -> &'static str {
    let k = GRADE_STEPS
        .iter()
        .position(|&n| steps <= n)
        .unwrap_or(GRADE_STEPS.len());
    GRADES[k]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::distance;

    #[test]
    fn his_route_runs_from_rix_maadi_to_new_prahv_a_tile_at_a_time_past_cities() {
        for seed in 0..30 {
            let r = route(seed);
            assert_eq!(r[0], (0, 5), "seed {seed}");
            assert_eq!(*r.last().unwrap(), (0, -5));
            for w in r.windows(2) {
                // one tile, or two where he passes over a city or place
                assert!((1..=2).contains(&distance(w[0], w[1])), "{w:?} seed {seed}");
            }
            for p in &r[1..r.len() - 1] {
                assert!(!fixed(*p), "{p:?} seed {seed}");
                assert!(p.0.abs() <= 7 && p.1.abs() <= 7, "{p:?} is off the region");
            }
            // long enough to give the player time, short enough to press
            assert!((30..=60).contains(&r.len()), "{} tiles, seed {seed}", r.len());
            // through each waypoint chunk
            for c in WAYPOINTS {
                assert!(r.iter().any(|p| chunk_of(*p) == c), "{c:?} seed {seed}");
            }
        }
        assert_ne!(route(1), route(2));
    }

    #[test]
    fn grades_follow_the_steps_taken() {
        assert_eq!(grade(0), "a");
        assert_eq!(grade(70), "a");
        assert_eq!(grade(71), "b");
        assert_eq!(grade(100), "b");
        assert_eq!(grade(130), "c");
        assert_eq!(grade(131), "d");
        assert_eq!(grade(9999), "d");
    }

    #[test]
    fn a_minion_is_a_black_or_red_creature_of_three_to_five() {
        let cards = crate::content::parse(include_str!("../../../data/cards.json")).unwrap();
        let mut seen = std::collections::BTreeSet::new();
        for x in -7..=7 {
            for y in -7..=7 {
                let m = minion_for(&cards, 4, (x, y)).unwrap();
                let c = &cards[m];
                assert!(
                    c.is(ty::CREATURE) && c.identity & !(B | R) == 0 && c.identity != 0,
                    "{}",
                    c.name
                );
                assert!((3..=5).contains(&c.value), "{}", c.name);
                assert_ne!(c.name, "Rakdos, Lord of Riots");
                seen.insert(m);
            }
        }
        assert!(seen.len() >= 8, "only {} kinds of minion", seen.len());
    }

    #[test]
    fn the_two_places_stand_off_every_city_and_inside_the_region() {
        for p in Place::ALL {
            let at = p.spot();
            let c = chunk_of(at);
            assert_ne!(at, chunk_centre(c));
            assert!(at.0.abs() <= 7 && at.1.abs() <= 7);
            assert_eq!(Place::from_id(p.id()), Some(p));
        }
        assert_eq!(
            chunk_plan(chunk_of(Place::Calliope.spot())).0,
            Some(crate::world::Guild::Rakdos)
        );
    }
}
