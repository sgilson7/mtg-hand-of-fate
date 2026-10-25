//! The cards, as the game uses them: read from `data/cards.json` (MTGJSON's
//! Return to Ravnica data, text verbatim) and joined with each card's rules
//! from `rules.rs`. A card with no rules is refused here (MTG-5).

use serde::Deserialize;

use crate::mana::{self, Cost};
use crate::rules::{self, Rules};

pub mod ty {
    pub const CREATURE: u8 = 1;
    pub const INSTANT: u8 = 2;
    pub const SORCERY: u8 = 4;
    pub const ENCHANTMENT: u8 = 8;
    pub const ARTIFACT: u8 = 16;
    pub const LAND: u8 = 32;
    pub const PLANESWALKER: u8 = 64;
}

/// What a card is as an encounter on the map, from its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A creature (an artifact creature too): an auto-battle.
    Creature,
    /// An enchantment or an artifact: a story card.
    Story,
    /// An instant or a sorcery: a magic story node.
    Magic,
    /// A land: attach it to a tile you have visited.
    Land,
    /// A planeswalker: a patron you meet, whose loyalty buys help.
    Patron,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Creature => "creature",
            Kind::Story => "story",
            Kind::Magic => "magic",
            Kind::Land => "land",
            Kind::Patron => "patron",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Card {
    pub number: String,
    pub name: String,
    pub cost_text: String,
    pub cost: Cost,
    pub value: usize,
    pub colours: u8,
    pub identity: u8,
    pub types: u8,
    pub type_line: String,
    pub subtypes: Vec<String>,
    pub power: i32,
    pub toughness: i32,
    pub power_text: String,
    pub toughness_text: String,
    pub rarity: char,
    pub group: String,
    pub text: String,
    pub flavor: String,
    pub artist: String,
    pub hybrid: bool,
    pub legendary: bool,
    pub rules: Rules,
}

impl Card {
    pub fn kind(&self) -> Kind {
        // a planeswalker is met as a patron, not fought on sight
        if self.is(ty::PLANESWALKER) {
            Kind::Patron
        } else if self.is(ty::CREATURE) {
            Kind::Creature
        } else if self.types & ty::LAND != 0 {
            Kind::Land
        } else if self.types & (ty::INSTANT | ty::SORCERY) != 0 {
            Kind::Magic
        } else {
            Kind::Story
        }
    }

    pub fn is(&self, t: u8) -> bool {
        self.types & t != 0
    }

    /// Instants and sorceries: cast, resolved, gone.
    pub fn is_spell(&self) -> bool {
        self.is(ty::INSTANT | ty::SORCERY)
    }

    pub fn multicoloured(&self) -> bool {
        mana::count_colours(self.colours) >= 2
    }

    /// Mana value tier for encounters: 1 for 0–2, 2 for 3–4, 3 for 5 and up.
    pub fn tier(&self) -> u8 {
        match self.value {
            0..=2 => 1,
            3..=4 => 2,
            _ => 3,
        }
    }
}

#[derive(Deserialize)]
struct RawCard {
    number: String,
    name: String,
    #[serde(rename = "manaCost")]
    mana_cost: Option<String>,
    #[serde(rename = "manaValue")]
    mana_value: u32,
    colors: Vec<String>,
    #[serde(rename = "colorIdentity")]
    color_identity: Vec<String>,
    types: Vec<String>,
    subtypes: Vec<String>,
    supertypes: Vec<String>,
    rarity: String,
    text: Option<String>,
    #[serde(rename = "flavorText")]
    flavor_text: Option<String>,
    power: Option<String>,
    toughness: Option<String>,
    artist: String,
    #[serde(rename = "isHybrid")]
    is_hybrid: bool,
    group: String,
}

#[derive(Deserialize)]
struct RawCards {
    cards: Vec<RawCard>,
}

fn stat(s: &Option<String>) -> i32 {
    s.as_deref().and_then(|v| v.parse::<i32>().ok()).unwrap_or(0)
}

pub fn parse(cards_json: &str) -> Result<Vec<Card>, String> {
    let raw: RawCards =
        serde_json::from_str(cards_json).map_err(|e| format!("The cards could not be read: {e}"))?;
    let mut out = Vec::new();
    for c in raw.cards {
        let rules = rules::for_card(&c.name)
            .ok_or_else(|| format!("The card \"{}\" has no rules in this game yet.", c.name))?;
        let cost_text = c.mana_cost.clone().unwrap_or_default();
        let cost = Cost::parse(&cost_text)?;
        let mut types = 0;
        for t in &c.types {
            types |= match t.as_str() {
                "Creature" => ty::CREATURE,
                "Instant" => ty::INSTANT,
                "Sorcery" => ty::SORCERY,
                "Enchantment" => ty::ENCHANTMENT,
                "Artifact" => ty::ARTIFACT,
                "Land" => ty::LAND,
                "Planeswalker" => ty::PLANESWALKER,
                other => {
                    return Err(format!(
                        "The card \"{}\" has a type \"{other}\" this game has no place for.",
                        c.name
                    ))
                }
            };
        }
        let mut type_line = c.supertypes.join(" ");
        if !type_line.is_empty() {
            type_line.push(' ');
        }
        type_line.push_str(&c.types.join(" "));
        if !c.subtypes.is_empty() {
            type_line.push_str(" — ");
            type_line.push_str(&c.subtypes.join(" "));
        }
        out.push(Card {
            number: c.number,
            value: c.mana_value as usize,
            colours: mana::mask_of(&c.colors),
            identity: mana::mask_of(&c.color_identity),
            types,
            type_line,
            subtypes: c.subtypes,
            power: stat(&c.power),
            toughness: stat(&c.toughness),
            power_text: c.power.unwrap_or_default(),
            toughness_text: c.toughness.unwrap_or_default(),
            rarity: c.rarity.chars().next().unwrap_or('c'),
            group: c.group,
            text: c.text.unwrap_or_default(),
            flavor: c.flavor_text.unwrap_or_default(),
            artist: c.artist,
            hybrid: c.is_hybrid,
            legendary: c.supertypes.iter().any(|s| s == "Legendary"),
            name: c.name,
            cost_text,
            cost,
            rules,
        });
    }
    Ok(out)
}

/// The cards, parsed once for tests.
#[cfg(test)]
pub fn for_tests() -> Vec<Card> {
    parse(include_str!("../../../data/cards.json")).expect("data/cards.json")
}

#[cfg(test)]
pub fn find(cards: &[Card], name: &str) -> usize {
    cards
        .iter()
        .position(|c| c.name == name)
        .unwrap_or_else(|| panic!("no card {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_card_of_the_region_loads_with_rules() {
        let cards = for_tests();
        assert_eq!(cards.len(), 254);
        let niv = &cards[find(&cards, "Niv-Mizzet, Dracogenius")];
        assert_eq!((niv.power, niv.toughness, niv.value), (5, 5, 6));
        assert_eq!(niv.kind(), Kind::Creature);
        assert!(niv.legendary && niv.multicoloured());
    }

    #[test]
    fn a_card_type_decides_its_encounter() {
        let cards = for_tests();
        let k = |n: &str| cards[find(&cards, n)].kind();
        assert_eq!(k("Street Sweeper"), Kind::Creature);
        assert_eq!(k("Chromatic Lantern"), Kind::Story);
        assert_eq!(k("Arrest"), Kind::Story);
        assert_eq!(k("Mizzium Mortars"), Kind::Magic);
        assert_eq!(k("Cancel"), Kind::Magic);
        assert_eq!(k("Izzet Guildgate"), Kind::Land);
        assert_eq!(k("Jace, Architect of Thought"), Kind::Patron);
        assert_eq!(k("Vraska the Unseen"), Kind::Patron);
    }

    #[test]
    fn a_card_without_rules_is_refused_by_name() {
        let bad =
            include_str!("../../../data/cards.json").replacen("\"Cobblebrute\"", "\"Grizzly Bears\"", 1);
        let e = parse(&bad).unwrap_err();
        assert_eq!(e, "The card \"Grizzly Bears\" has no rules in this game yet.");
    }

    #[test]
    fn the_tier_follows_mana_value() {
        let cards = for_tests();
        let t = |n: &str| cards[find(&cards, n)].tier();
        assert_eq!(t("Trained Caracal"), 1);
        assert_eq!(t("Dreg Mangler"), 2);
        assert_eq!(t("Angel of Serenity"), 3);
    }
}
