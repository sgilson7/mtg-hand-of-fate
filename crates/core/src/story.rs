//! Story cards: the game's reading of each card as a moment in the Tenth
//! District (`data/stories/*.json`, checked by `tools/stories/check.py`).
//! A creature's story wraps its fight; an enchantment's or artifact's is a
//! scene with two or three choices; an instant's or sorcery's is a magic
//! scene; a land's is a short find. Outcomes are effects this module names
//! and `session.rs` applies, and fate draws: Hand of Fate's four cards,
//! shown face up, turned and shuffled, and one picked.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::rng::Rng;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Take(bool),
    Fight(bool),
    Leave(bool),
    Gold(i32),
    Supplies(i32),
    Health(i32),
    PoolCard(bool),
    Style((String, i32)),
    QuestStart(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Fate {
    pub faces: [u8; 4],
    pub huge_success: Option<Box<Outcome>>,
    pub success: Option<Box<Outcome>>,
    pub failure: Option<Box<Outcome>>,
    pub huge_failure: Option<Box<Outcome>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Outcome {
    pub text: String,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub fate: Option<Fate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Need {
    Charisma(i32),
    Style((String, i32)),
    Gold(i32),
    Supplies(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Choice {
    pub text: String,
    #[serde(default, deserialize_with = "need")]
    pub needs: Option<Need>,
    pub outcome: Outcome,
}

fn need<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Need>, D::Error> {
    let v: Option<BTreeMap<String, serde_json::Value>> = Option::deserialize(d)?;
    let Some(v) = v else { return Ok(None) };
    let Some((k, val)) = v.into_iter().next() else {
        return Ok(None);
    };
    let n = |x: &serde_json::Value| x.as_i64().unwrap_or(0) as i32;
    Ok(Some(match k.as_str() {
        "charisma" => Need::Charisma(n(&val)),
        "gold" => Need::Gold(n(&val)),
        "supplies" => Need::Supplies(n(&val)),
        _ => {
            let g = val.get(0).and_then(|g| g.as_str()).unwrap_or("").to_string();
            let k = val.get(1).map(n).unwrap_or(0);
            Need::Style((g, k))
        }
    }))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Story {
    pub number: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub win: String,
    #[serde(default)]
    pub lose: String,
    #[serde(default)]
    pub flee: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct StoryFile {
    stories: Vec<Story>,
}

/// Reads the story files, keyed by collector number.
pub fn parse(files: &[&str]) -> Result<BTreeMap<String, Story>, String> {
    let mut out = BTreeMap::new();
    for f in files {
        let file: StoryFile =
            serde_json::from_str(f).map_err(|e| format!("A story file could not be read: {e}"))?;
        for s in file.stories {
            out.insert(s.number.clone(), s);
        }
    }
    Ok(out)
}

/// The four kinds of fate card, in the order `Fate::faces` counts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Face {
    HugeSuccess,
    Success,
    Failure,
    HugeFailure,
}

impl Face {
    pub const ALL: [Face; 4] = [Face::HugeSuccess, Face::Success, Face::Failure, Face::HugeFailure];
    pub fn key(self) -> &'static str {
        match self {
            Face::HugeSuccess => "fate.huge_success",
            Face::Success => "fate.success",
            Face::Failure => "fate.failure",
            Face::HugeFailure => "fate.huge_failure",
        }
    }
    pub fn good(self) -> bool {
        matches!(self, Face::HugeSuccess | Face::Success)
    }
}

impl Fate {
    /// The cards shown before the shuffle. `extra_success` adds one Success
    /// (the Azorius circlet).
    pub fn cards(&self, extra_success: bool) -> Vec<Face> {
        let mut out = Vec::new();
        for (face, n) in Face::ALL.iter().zip(self.faces) {
            for _ in 0..n {
                out.push(*face);
            }
        }
        if extra_success {
            out.push(Face::Success);
        }
        out
    }

    /// The outcome for a face; a missing branch falls back to the nearest
    /// one on the same side.
    pub fn branch(&self, face: Face) -> Option<&Outcome> {
        fn pick<'a>(a: &'a Option<Box<Outcome>>, b: &'a Option<Box<Outcome>>) -> Option<&'a Outcome> {
            a.as_deref().or(b.as_deref())
        }
        match face {
            Face::HugeSuccess => pick(&self.huge_success, &self.success),
            Face::Success => pick(&self.success, &self.huge_success),
            Face::Failure => pick(&self.failure, &self.huge_failure),
            Face::HugeFailure => pick(&self.huge_failure, &self.failure),
        }
    }
}

/// Shuffles fate cards: Fisher–Yates on the seed.
pub fn shuffle(faces: &mut [Face], rng: &mut Rng) {
    for i in (1..faces.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        faces.swap(i, j);
    }
}

#[cfg(test)]
pub fn all_for_tests() -> BTreeMap<String, Story> {
    parse(&[
        include_str!("../../../data/stories/W.json"),
        include_str!("../../../data/stories/U.json"),
        include_str!("../../../data/stories/B.json"),
        include_str!("../../../data/stories/R.json"),
        include_str!("../../../data/stories/G.json"),
        include_str!("../../../data/stories/Azorius.json"),
        include_str!("../../../data/stories/Izzet.json"),
        include_str!("../../../data/stories/Rakdos.json"),
        include_str!("../../../data/stories/Golgari.json"),
        include_str!("../../../data/stories/Selesnya.json"),
        include_str!("../../../data/stories/colourless.json"),
    ])
    .expect("stories")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{for_tests, Kind};

    #[test]
    fn every_card_has_a_story_of_its_kind() {
        let cards = for_tests();
        let stories = all_for_tests();
        for c in &cards {
            let s = stories
                .get(&c.number)
                .unwrap_or_else(|| panic!("no story for {}", c.name));
            assert_eq!(s.kind, c.kind().name(), "{}", c.name);
            if matches!(c.kind(), Kind::Story | Kind::Magic) {
                assert!((2..=3).contains(&s.choices.len()), "{}", c.name);
            }
        }
    }

    #[test]
    fn a_fate_draw_shows_its_faces_and_falls_back_on_its_own_side() {
        let stories = all_for_tests();
        let fate = stories
            .values()
            .flat_map(|s| s.choices.iter())
            .find_map(|c| c.outcome.fate.clone())
            .expect("a fate draw somewhere");
        let cards = fate.cards(false);
        assert_eq!(cards.len(), 4);
        assert_eq!(fate.cards(true).len(), 5);
        for f in Face::ALL {
            if let Some(o) = fate.branch(f) {
                assert!(!o.text.is_empty());
            }
        }
    }

    #[test]
    fn the_lantern_starts_its_quest() {
        let stories = all_for_tests();
        let lantern = &stories["226"];
        let starts = |o: &Outcome| -> bool {
            o.effects.contains(&Effect::QuestStart("lantern".into()))
                || o.fate
                    .as_ref()
                    .map(|f| {
                        Face::ALL.iter().any(|x| {
                            f.branch(*x)
                                .map(|b| b.effects.contains(&Effect::QuestStart("lantern".into())))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
        };
        assert!(lantern.choices.iter().any(|c| starts(&c.outcome)));
    }

    #[test]
    fn every_quest_that_starts_from_a_card_is_started_by_that_cards_story() {
        let stories = all_for_tests();
        let quests = crate::quest::parse(include_str!("../../../data/quests.json")).unwrap();
        for q in &quests {
            let Some(n) = q.starts.strip_prefix("card:") else {
                continue;
            };
            let want = Effect::QuestStart(q.id.clone());
            let starts = |o: &Outcome| -> bool {
                o.effects.contains(&want)
                    || o.fate
                        .as_ref()
                        .map(|f| {
                            Face::ALL
                                .iter()
                                .any(|x| f.branch(*x).map(|b| b.effects.contains(&want)).unwrap_or(false))
                        })
                        .unwrap_or(false)
            };
            let story = stories.get(n).unwrap_or_else(|| panic!("no story for card {n}"));
            assert!(
                story.choices.iter().any(|c| starts(&c.outcome)),
                "{}: card {n}'s story never starts it",
                q.id
            );
        }
    }

    #[test]
    fn a_shuffle_keeps_the_faces_and_follows_the_seed() {
        let mut a = vec![Face::Success, Face::Success, Face::Failure, Face::HugeFailure];
        let mut b = a.clone();
        shuffle(&mut a, &mut Rng::new(5));
        shuffle(&mut b, &mut Rng::new(5));
        assert_eq!(a, b);
        let mut sorted = a.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            vec![Face::Success, Face::Success, Face::Failure, Face::HugeFailure]
        );
    }
}
