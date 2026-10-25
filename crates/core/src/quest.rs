//! Quests (`data/quests.json`): three steps each, done at a city, in a
//! district of a guild, by winning a fight in such a district, or at the
//! lair. The main quest, the Dracogenius, starts with a new game and ends
//! with the guardian; each guild has one, started by one of its cards.
//! The second mission's quests (T32, T34) start with its new game
//! (`mission:riot`), and add two kinds of step: at one of its places, and
//! Rakdos beaten; and a reward that slows or weakens him.

use serde::Deserialize;

use crate::riot::Place;
use crate::world::{City, Guild};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Step {
    pub at: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub guild: String,
    #[serde(default)]
    pub place: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct Reward {
    #[serde(default)]
    pub gold: i32,
    #[serde(default)]
    pub armour: String,
    #[serde(default)]
    pub style: Option<(String, i32)>,
    /// What it does to Rakdos in the Riot: "slow" or "weaken".
    #[serde(default)]
    pub rakdos: String,
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
        let starts = q.starts == "new_game"
            || q.starts == "mission:riot"
            || q.starts.strip_prefix("card:").is_some()
            || q.starts
                .strip_prefix("hall:")
                .map(|c| City::from_id(c).is_some())
                .unwrap_or(false);
        if !starts {
            return Err(format!(
                "The quest \"{}\" starts in a way this game does not know.",
                q.id
            ));
        }
        for s in &q.steps {
            let ok = match s.at.as_str() {
                "city" => City::from_id(&s.city).is_some(),
                "district" | "win_in_district" => Guild::from_name(&s.guild).is_some(),
                "lair" | "rakdos" => true,
                "place" => Place::from_id(&s.place).is_some(),
                _ => false,
            };
            if !ok {
                return Err(format!(
                    "The quest \"{}\" has a step this game cannot place.",
                    q.id
                ));
            }
        }
        if !["", "slow", "weaken"].contains(&q.reward.rakdos.as_str()) {
            return Err(format!(
                "The quest \"{}\" does something to Rakdos this game does not know.",
                q.id
            ));
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
    AtPlace(Place),
    BeatRakdos,
}

impl Step {
    pub fn met_by(&self, h: Happened) -> bool {
        match (self.at.as_str(), h) {
            ("city", Happened::AtCity(c)) => c.id() == self.city,
            ("district", Happened::InDistrict(g)) => g.name() == self.guild,
            ("win_in_district", Happened::WonInDistrict(g)) => g.name() == self.guild,
            ("lair", Happened::AtLair) => true,
            ("place", Happened::AtPlace(p)) => p.id() == self.place,
            ("rakdos", Happened::BeatRakdos) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_three_quests_of_three_steps_three_from_cards_for_each_guild_two_at_each_new_hall_three_for_the_riot(
    ) {
        let q = parse(include_str!("../../../data/quests.json")).unwrap();
        assert_eq!(q.len(), 23);
        let riot: Vec<&str> = q
            .iter()
            .filter(|q| q.starts == "mission:riot")
            .map(|q| q.id.as_str())
            .collect();
        assert_eq!(riot, vec!["riot", "calliope", "column"], "T32, T34");
        for c in ["rix", "svogthos"] {
            let at = q.iter().filter(|q| q.starts == format!("hall:{c}")).count();
            assert_eq!(at, 2, "{c}: two quests in its hall (T27)");
        }
        for g in ["Azorius", "Izzet", "Rakdos", "Golgari", "Selesnya"] {
            let side = q
                .iter()
                .filter(|q| q.guild == g && q.starts.starts_with("card:"))
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
    fn the_riots_quests_slow_and_weaken_rakdos_and_end_with_him() {
        let q = parse(include_str!("../../../data/quests.json")).unwrap();
        let get = |id: &str| q.iter().find(|q| q.id == id).unwrap().clone();
        assert_eq!(get("calliope").reward.rakdos, "slow");
        assert_eq!(get("column").reward.rakdos, "weaken");
        assert!(get("riot").steps[2].met_by(Happened::BeatRakdos));
        assert!(!get("riot").steps[2].met_by(Happened::AtLair));
        let c = get("calliope");
        assert!(c.steps[0].met_by(Happened::AtPlace(Place::Calliope)));
        assert!(!c.steps[0].met_by(Happened::AtPlace(Place::Column)));
        let bad = include_str!("../../../data/quests.json").replacen("\"weaken\"", "\"banish\"", 1);
        assert!(parse(&bad).unwrap_err().contains("column"));
        let nowhere = include_str!("../../../data/quests.json").replacen(
            "\"place\": \"column\"",
            "\"place\": \"atlantis\"",
            1,
        );
        assert!(parse(&nowhere).unwrap_err().contains("column"));
    }

    #[test]
    fn a_step_in_a_place_that_does_not_exist_is_refused() {
        let bad = include_str!("../../../data/quests.json").replacen("\"prahv\"", "\"atlantis\"", 1);
        assert!(parse(&bad).unwrap_err().contains("dracogenius"));
        let hall = include_str!("../../../data/quests.json").replacen("\"hall:rix\"", "\"hall:atlantis\"", 1);
        assert!(parse(&hall).unwrap_err().contains("encore"));
    }
}
