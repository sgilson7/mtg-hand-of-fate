//! Quests (`data/quests.json`): three steps each, done at a city, in a
//! district of a guild, by winning a fight in such a district, or at the
//! lair. The main quest, the Dracogenius, starts with a new game and ends
//! with the guardian; each guild has one, started by one of its cards.

use serde::Deserialize;

use crate::world::{City, Guild};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Step {
    pub at: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub guild: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct Reward {
    #[serde(default)]
    pub gold: i32,
    #[serde(default)]
    pub armour: String,
    #[serde(default)]
    pub style: Option<(String, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Quest {
    pub id: String,
    pub guild: String,
    pub starts: String,
    pub steps: Vec<Step>,
    pub reward: Reward,
}

#[derive(Deserialize)]
struct QuestFile {
    quests: Vec<Quest>,
}

pub fn parse(json: &str) -> Result<Vec<Quest>, String> {
    let f: QuestFile =
        serde_json::from_str(json).map_err(|e| format!("The quests could not be read: {e}"))?;
    for q in &f.quests {
        for s in &q.steps {
            let ok = match s.at.as_str() {
                "city" => City::from_id(&s.city).is_some(),
                "district" | "win_in_district" => Guild::from_name(&s.guild).is_some(),
                "lair" => true,
                _ => false,
            };
            if !ok {
                return Err(format!(
                    "The quest \"{}\" has a step this game cannot place.",
                    q.id
                ));
            }
        }
    }
    Ok(f.quests)
}

/// Something that happened that a quest step may wait on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Happened {
    AtCity(City),
    InDistrict(Guild),
    WonInDistrict(Guild),
    AtLair,
}

impl Step {
    pub fn met_by(&self, h: Happened) -> bool {
        match (self.at.as_str(), h) {
            ("city", Happened::AtCity(c)) => c.id() == self.city,
            ("district", Happened::InDistrict(g)) => g.name() == self.guild,
            ("win_in_district", Happened::WonInDistrict(g)) => g.name() == self.guild,
            ("lair", Happened::AtLair) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixteen_quests_of_three_steps_three_for_each_guild_and_izzet_with_the_main_one() {
        let q = parse(include_str!("../../../data/quests.json")).unwrap();
        assert_eq!(q.len(), 16);
        for g in ["Azorius", "Izzet", "Rakdos", "Golgari", "Selesnya"] {
            let side = q
                .iter()
                .filter(|q| q.guild == g && q.starts != "new_game")
                .count();
            assert_eq!(side, 3, "{g}: its first quest and two more (T21)");
        }
        assert!(q.iter().all(|q| q.steps.len() == 3));
        assert_eq!(q[0].id, "dracogenius");
        assert_eq!(q[0].starts, "new_game");
    }

    #[test]
    fn a_step_is_met_only_by_its_own_place() {
        let q = parse(include_str!("../../../data/quests.json")).unwrap();
        let lantern = q.iter().find(|q| q.id == "lantern").unwrap();
        assert!(lantern.steps[0].met_by(Happened::InDistrict(Guild::Izzet)));
        assert!(!lantern.steps[0].met_by(Happened::InDistrict(Guild::Rakdos)));
        assert!(!lantern.steps[0].met_by(Happened::WonInDistrict(Guild::Izzet)));
        assert!(lantern.steps[2].met_by(Happened::AtCity(City::Prahv)));
    }

    #[test]
    fn a_step_in_a_place_that_does_not_exist_is_refused() {
        let bad = include_str!("../../../data/quests.json").replacen("\"prahv\"", "\"atlantis\"", 1);
        assert!(parse(&bad).unwrap_err().contains("dracogenius"));
    }
}
