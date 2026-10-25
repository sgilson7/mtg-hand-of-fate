//! The player: where they stand, what they carry, what they wear, and the
//! numbers those decide. Health's maximum is the base plus the armour worn
//! (the person's rule: "your max health is dependent on your armor"),
//! Charisma lowers prices and opens choices, Style is standing with each
//! guild, and Reach is how far a land may be from a fight and still feed it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::duel::perk;
use crate::world::{Guild, Pos, Terrain};

pub const START_SUPPLIES: i32 = 10;
pub const MAX_SUPPLIES: i32 = 20;
pub const PACK_MULE_SUPPLIES: i32 = 5;
pub const START_GOLD: i32 = 20;
pub const START_REACH: i32 = 2;
pub const DECK_MAX: usize = 20;
/// Health lost on a step with no supplies.
pub const HUNGRY_STEP: i32 = 2;
/// Health regained on a step with supplies, up to the maximum.
pub const FED_STEP_HEAL: i32 = 1;
/// After a defeat: the share of gold lost (a third), and health back to half.
pub const DEFEAT_GOLD_DIVISOR: i32 = 3;
pub const HEAL_AFTER_WIN: i32 = 3;
pub const CHARISMA_PRICE_STEP: i32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piece {
    pub id: String,
    pub slot: String,
    pub guild: String,
    pub health: i32,
    pub charisma: i32,
    pub style: i32,
    pub effect: String,
    pub price: i32,
    pub style_needed: i32,
    /// The one city whose armourer sells it; empty for its guild's armourers (T27).
    #[serde(default)]
    pub city: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Armoury {
    pub base_health: i32,
    pub slots: Vec<String>,
    pub pieces: Vec<Piece>,
    pub sets: BTreeMap<String, String>,
}

pub fn parse_armour(json: &str) -> Result<Armoury, String> {
    let a: Armoury = serde_json::from_str(json).map_err(|e| format!("The armour could not be read: {e}"))?;
    for p in &a.pieces {
        if !a.slots.contains(&p.slot) {
            return Err(format!(
                "The armour piece \"{}\" names a slot \"{}\" there is none of.",
                p.id, p.slot
            ));
        }
        if effect_perk(&p.effect).is_none()
            && !MAP_EFFECTS.contains(&p.effect.as_str())
            && !p.effect.is_empty()
        {
            return Err(format!(
                "The armour piece \"{}\" has an effect \"{}\" this game does not know.",
                p.id, p.effect
            ));
        }
    }
    for e in a.sets.values() {
        if effect_perk(e).is_none() {
            return Err(format!("A set has an effect \"{e}\" this game does not know."));
        }
    }
    Ok(a)
}

/// Armour rules that act in a fight.
pub fn effect_perk(e: &str) -> Option<u32> {
    Some(match e {
        "detain_first" => perk::DETAIN_FIRST,
        "start_mana" => perk::START_MANA,
        "extra_draw" => perk::EXTRA_DRAW,
        "spell_ping" => perk::SPELL_PING,
        "opening_damage" => perk::OPENING_DAMAGE,
        "unleash_haste" => perk::UNLEASH_HASTE,
        "haste_first" => perk::HASTE_FIRST,
        "scavenge_cheap" => perk::SCAVENGE_CHEAP,
        "regrowth" => perk::REGROWTH,
        "token_start" => perk::TOKEN_START,
        "set_detain_two" => perk::SET_DETAIN_TWO,
        "set_spell_cheap" => perk::SET_SPELL_CHEAP,
        "set_first_strike" => perk::SET_FIRST_STRIKE,
        "set_regenerate" => perk::SET_REGENERATE,
        "set_populate" => perk::SET_POPULATE,
        _ => return None,
    })
}

/// Armour rules that act on the map or in a story.
pub const MAP_EFFECTS: [&str; 5] = [
    "fate_success",
    "reach",
    "marsh_walker",
    "forest_walker",
    "heal_after_win",
];

/// The trainer's abilities: id, price before Charisma, how many times.
pub const ABILITIES: [(&str, i32, u32); 3] = [("reach", 35, 2), ("banker", 30, 1), ("pack_mule", 20, 1)];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub pos: Pos,
    pub health: i32,
    pub supplies: i32,
    pub gold: i32,
    pub collection: Vec<usize>,
    pub deck: Vec<usize>,
    /// Worn, by slot.
    pub worn: BTreeMap<String, String>,
    pub owned: Vec<String>,
    /// Standing with each guild from quests and stories.
    pub standing: BTreeMap<String, i32>,
    pub abilities: BTreeMap<String, u32>,
    /// Quest id to steps done; a quest at its step count is finished.
    pub quests: BTreeMap<String, u8>,
    pub last_city: Pos,
    pub guardian_beaten: bool,
    pub fights_won: u32,
    /// Steps taken this game: the Riot's grade (T32).
    #[serde(default)]
    pub steps: u32,
}

impl Player {
    pub fn new(armoury: &Armoury) -> Player {
        let mut worn = BTreeMap::new();
        let mut owned = Vec::new();
        for p in armoury.pieces.iter().filter(|p| p.price == 0) {
            worn.insert(p.slot.clone(), p.id.clone());
            owned.push(p.id.clone());
        }
        let mut pl = Player {
            pos: (0, 0),
            health: 0,
            supplies: START_SUPPLIES,
            gold: START_GOLD,
            collection: Vec::new(),
            deck: Vec::new(),
            worn,
            owned,
            standing: BTreeMap::new(),
            abilities: BTreeMap::new(),
            quests: BTreeMap::new(),
            last_city: (0, 0),
            guardian_beaten: false,
            fights_won: 0,
            steps: 0,
        };
        pl.health = pl.max_health(armoury);
        pl
    }

    fn pieces<'a>(&'a self, armoury: &'a Armoury) -> impl Iterator<Item = &'a Piece> {
        self.worn
            .values()
            .filter_map(move |id| armoury.pieces.iter().find(|p| &p.id == id))
    }

    pub fn max_health(&self, armoury: &Armoury) -> i32 {
        armoury.base_health + self.pieces(armoury).map(|p| p.health).sum::<i32>()
    }

    pub fn charisma(&self, armoury: &Armoury) -> i32 {
        self.pieces(armoury).map(|p| p.charisma).sum()
    }

    /// Standing with one guild: what quests gave, plus that guild's armour worn.
    pub fn style_with(&self, armoury: &Armoury, guild: &str) -> i32 {
        self.standing.get(guild).copied().unwrap_or(0)
            + self
                .pieces(armoury)
                .filter(|p| p.guild == guild)
                .map(|p| p.style)
                .sum::<i32>()
    }

    /// Style with every guild together: how the Tenth District sees you.
    pub fn style(&self, armoury: &Armoury) -> i32 {
        self.standing.values().sum::<i32>() + self.pieces(armoury).map(|p| p.style).sum::<i32>()
    }

    pub fn reach(&self, armoury: &Armoury) -> i32 {
        START_REACH
            + self.abilities.get("reach").copied().unwrap_or(0) as i32
            + self.pieces(armoury).filter(|p| p.effect == "reach").count() as i32
    }

    pub fn max_supplies(&self) -> i32 {
        MAX_SUPPLIES
            + if self.abilities.contains_key("pack_mule") {
                PACK_MULE_SUPPLIES
            } else {
                0
            }
    }

    pub fn wears_effect(&self, armoury: &Armoury, e: &str) -> bool {
        self.pieces(armoury).any(|p| p.effect == e)
    }

    /// The guild whose four pieces are all worn, if any.
    pub fn full_set(&self, armoury: &Armoury) -> Option<String> {
        let guilds: Vec<String> = self.pieces(armoury).map(|p| p.guild.clone()).collect();
        let first = guilds.first()?.clone();
        (guilds.len() == armoury.slots.len() && !first.is_empty() && guilds.iter().all(|g| *g == first))
            .then_some(first)
    }

    /// The rules the player's armour and abilities bring to a fight.
    pub fn perks(&self, armoury: &Armoury) -> u32 {
        let mut p = self
            .pieces(armoury)
            .filter_map(|p| effect_perk(&p.effect))
            .fold(0, |a, b| a | b);
        if let Some(g) = self.full_set(armoury) {
            p |= armoury.sets.get(&g).and_then(|e| effect_perk(e)).unwrap_or(0);
        }
        if self.abilities.contains_key("banker") {
            p |= perk::START_MANA;
        }
        p
    }

    /// A price after Charisma: five percent off for each point, never below 1.
    pub fn price(&self, armoury: &Armoury, base: i32) -> i32 {
        let off = (self.charisma(armoury) * CHARISMA_PRICE_STEP).clamp(0, 50);
        (base * (100 - off) / 100).max(1)
    }

    /// The supply cost of stepping onto a terrain.
    pub fn step_cost(&self, armoury: &Armoury, t: Terrain) -> i32 {
        let free = (t == Terrain::Swamp && self.wears_effect(armoury, "marsh_walker"))
            || (t == Terrain::Forest && self.wears_effect(armoury, "forest_walker"));
        if free {
            0
        } else {
            1
        }
    }

    /// Spends supplies for a step; heals a little when fed, hurts when not.
    pub fn step(&mut self, armoury: &Armoury, t: Terrain) {
        let cost = self.step_cost(armoury, t);
        if cost == 0 || self.supplies >= cost {
            self.supplies -= cost;
            self.health = (self.health + FED_STEP_HEAL).min(self.max_health(armoury));
        } else {
            self.health -= HUNGRY_STEP;
        }
    }

    pub fn guild_standing(&mut self, g: Guild, n: i32) {
        *self.standing.entry(g.name().to_string()).or_insert(0) += n;
    }

    /// Wear a piece the player owns; health keeps its distance from the maximum.
    pub fn wear(&mut self, armoury: &Armoury, id: &str) -> Result<(), String> {
        let piece = armoury
            .pieces
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("There is no armour \"{id}\"."))?;
        if !self.owned.iter().any(|o| o == id) {
            return Err(format!("You do not own \"{id}\"."));
        }
        let before = self.max_health(armoury);
        self.worn.insert(piece.slot.clone(), id.to_string());
        let after = self.max_health(armoury);
        self.health = (self.health + after - before).clamp(1, after);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn armoury() -> Armoury {
        parse_armour(include_str!("../../../data/armour.json")).unwrap()
    }

    #[test]
    fn health_is_the_base_plus_the_armour_worn() {
        let a = armoury();
        let p = Player::new(&a);
        // base 10 + hood 2 + coat 4 + wraps 0 + boots 2
        assert_eq!(p.max_health(&a), 18);
        assert_eq!(p.health, 18);
        assert_eq!(p.reach(&a), 2);
    }

    #[test]
    fn new_armour_raises_the_maximum_and_keeps_the_wound() {
        let a = armoury();
        let mut p = Player::new(&a);
        p.health = 12;
        p.owned.push("azorius_chest".into());
        p.wear(&a, "azorius_chest").unwrap();
        // the coat's 4 becomes the plate's 8: maximum 22, health 12 + 4
        assert_eq!(p.max_health(&a), 22);
        assert_eq!(p.health, 16);
        assert_eq!(p.charisma(&a), 1);
        assert!(p.perks(&a) & perk::DETAIN_FIRST != 0);
    }

    #[test]
    fn charisma_takes_five_percent_a_point_off_a_price() {
        let a = armoury();
        let mut p = Player::new(&a);
        assert_eq!(p.price(&a, 40), 40);
        for id in ["azorius_head", "azorius_chest", "azorius_hands"] {
            p.owned.push(id.into());
            p.wear(&a, id).unwrap();
        }
        // charisma 2 + 1 + 1 = 4: 20 percent off 40 is 32
        assert_eq!(p.charisma(&a), 4);
        assert_eq!(p.price(&a, 40), 32);
    }

    #[test]
    fn a_full_guild_set_adds_its_set_rule() {
        let a = armoury();
        let mut p = Player::new(&a);
        for id in ["izzet_head", "izzet_chest", "izzet_hands", "izzet_feet"] {
            p.owned.push(id.into());
            p.wear(&a, id).unwrap();
        }
        assert_eq!(p.full_set(&a).as_deref(), Some("Izzet"));
        assert!(p.perks(&a) & perk::SET_SPELL_CHEAP != 0);
        // the Izzet gloves add one Reach
        assert_eq!(p.reach(&a), 3);
    }

    #[test]
    fn a_hungry_step_hurts_and_a_fed_step_heals() {
        let a = armoury();
        let mut p = Player::new(&a);
        p.health = 10;
        p.step(&a, Terrain::Plains);
        assert_eq!((p.supplies, p.health), (9, 11));
        p.supplies = 0;
        p.step(&a, Terrain::Plains);
        assert_eq!((p.supplies, p.health), (0, 9));
    }

    #[test]
    fn an_unknown_armour_effect_is_refused_by_name() {
        let bad = include_str!("../../../data/armour.json").replacen("\"fate_success\"", "\"fly\"", 1);
        assert_eq!(
            parse_armour(&bad).unwrap_err(),
            "The armour piece \"azorius_head\" has an effect \"fly\" this game does not know."
        );
    }
}
