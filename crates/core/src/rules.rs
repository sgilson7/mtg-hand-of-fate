//! What each card does in a fight: one definition per card of the first
//! region, written against its Oracle text in `data/cards.json`. A card in a
//! pool with no definition here is refused when the content loads (rule
//! MTG-5), so no card can sit in a deck doing nothing by accident.
//!
//! The auto-battle reads these: keywords, triggers, the effect of a spell,
//! activated abilities with when the battle may use them, and the guild
//! mechanics (detain, overload, unleash, scavenge, populate). Where a card's
//! text refers to something this game has no place for (graveyards being
//! cast from, lands on the battlefield, a library search), the definition
//! says what it does here, and SECOND-ORDER.md lists the card.

use crate::mana::{Cost, Sym, B, G, R, U, W};

pub mod kw {
    pub const FLYING: u32 = 1;
    pub const FIRST_STRIKE: u32 = 1 << 1;
    pub const DOUBLE_STRIKE: u32 = 1 << 2;
    pub const DEATHTOUCH: u32 = 1 << 3;
    pub const LIFELINK: u32 = 1 << 4;
    pub const TRAMPLE: u32 = 1 << 5;
    pub const HASTE: u32 = 1 << 6;
    pub const VIGILANCE: u32 = 1 << 7;
    pub const REACH: u32 = 1 << 8;
    pub const DEFENDER: u32 = 1 << 9;
    pub const HEXPROOF: u32 = 1 << 10;
    pub const UNBLOCKABLE: u32 = 1 << 11;
    pub const SWAMPWALK: u32 = 1 << 12;
    pub const FLASH: u32 = 1 << 13;
    pub const INDESTRUCTIBLE: u32 = 1 << 14;
    pub const CANT_BE_COUNTERED: u32 = 1 << 15;
}

/// Which permanents an effect may pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    Mine,
    Theirs,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filter {
    pub who: Who,
    /// Creatures only; false means any nonland permanent.
    pub creature: bool,
    pub max_value: Option<usize>,
    pub min_power: Option<i32>,
    /// Some(true): flying only; Some(false): without flying only.
    pub flying: Option<bool>,
    pub mono: bool,
    pub enchantment: bool,
}

pub const fn creatures(who: Who) -> Filter {
    Filter {
        who,
        creature: true,
        max_value: None,
        min_power: None,
        flying: None,
        mono: false,
        enchantment: false,
    }
}

pub const fn permanents(who: Who) -> Filter {
    Filter {
        creature: false,
        ..creatures(who)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Bird,
    Soldier,
    Goblin,
    Centaur,
    Rhino,
    Knight,
}

impl Token {
    /// Power, toughness, keywords, colours, and the name the log uses.
    pub fn stats(self) -> (i32, i32, u32, u8, &'static str) {
        match self {
            Token::Bird => (1, 1, kw::FLYING, W, "Bird"),
            Token::Soldier => (1, 1, 0, W, "Soldier"),
            Token::Goblin => (1, 1, 0, R, "Goblin"),
            Token::Centaur => (3, 3, 0, G, "Centaur"),
            Token::Rhino => (4, 4, kw::TRAMPLE, G, "Rhino"),
            Token::Knight => (2, 2, kw::VIGILANCE, W, "Knight"),
        }
    }

    pub fn model(self) -> &'static str {
        match self {
            Token::Bird => "token_bird",
            Token::Soldier => "token_soldier",
            Token::Goblin => "token_goblin",
            Token::Centaur => "token_centaur",
            Token::Rhino => "token_rhino",
            Token::Knight => "token_knight",
        }
    }
}

/// Where damage or a spell goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aim {
    /// One permanent matching the filter.
    One(Filter),
    /// Up to n of them.
    UpTo(u8, Filter),
    /// Every permanent matching the filter (overload, "each").
    Each(Filter),
    /// A creature matching the filter, or the opponent ("any target").
    CreatureOrFace(Filter),
    /// The opponent.
    Face,
    /// The side itself.
    Me,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eff {
    Damage(i32, Aim),
    Destroy(Aim),
    Exile(Aim),
    Bounce(Aim),
    Detain(Aim),
    Tap(Aim),
    /// +p/+t and keywords until end of turn; `untap` untaps it too.
    Pump {
        p: i32,
        t: i32,
        kw: u32,
        aim: Aim,
        untap: bool,
    },
    /// +1/+1 counters.
    Counters(i32, Aim),
    Token(Token, u8),
    Populate,
    Draw(u8),
    Gain(i32),
    /// Each opponent loses life.
    Drain(i32),
    OppDiscard(u8),
    /// Each player sacrifices a creature (Slum Reaper).
    EachSacrifices,
    /// Return another creature you control to hand, or sacrifice this one (Faerie Impostor).
    ImpostorReturn,
    /// Exile up to three other creatures until this leaves (Angel of Serenity).
    AngelExile,
    /// Put this card on top of its owner's library (Dark Revenant).
    ToLibraryTop,
    /// +1/+1 for each creature you control and trample (Chorus of Might).
    Chorus(Aim),
    /// Exile a nonland permanent and all others with its name (Detention Sphere).
    SphereExile,
    /// Counter a spell; `pay` lets its caster pay that much to stop it.
    Counter {
        creature: Option<bool>,
        pay: u8,
    },
    /// Draw two, then discard two (Izzet Charm's third mode).
    Loot(u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// Before attacks, on a creature that will attack.
    Pump,
    /// Damage: a creature it can kill, or the opponent.
    Ping,
    /// Whenever this would die and the side can pay.
    Regenerate,
    /// Sacrifice for a token, when mana is spare.
    SacForToken,
    /// Becomes a creature until end of turn, to attack.
    Animate,
    /// Equip the best creature.
    Equip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activated {
    pub cost: Cost,
    pub tap: bool,
    pub sac: bool,
    pub effects: Vec<Eff>,
    pub when: Use,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aura {
    /// Can't attack or block, no activated abilities (Arrest).
    Arrest,
    /// Doesn't untap (Paralyzing Grasp).
    Grasp,
    /// -2/-2, its controller loses 2 at upkeep (Stab Wound).
    StabWound,
    /// +1/+1 per enchantment you control and first strike (Ethereal Armor).
    EtherealArmor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Static {
    /// Creature tokens you control get +1/+1 (Phantom General).
    TokenAnthem,
    /// Instants and sorceries you cast cost {1} less (Goblin Electromancer).
    SpellsCheaper,
    /// When a creature you control dies, its power in counters on another (Death's Presence).
    DeathsPresence,
    /// Instants and sorceries go to exile instead of a graveyard (Dryad Militant).
    SpellsToExile,
    /// At each opponent's upkeep, if they hold one card or fewer, they lose 3 (Shrieking Affliction).
    Shrieking,
    /// At your upkeep, populate (Growing Ranks).
    UpkeepPopulate,
    /// Lands' mana is any colour, and this taps for any (Chromatic Lantern).
    Lantern,
    /// When it deals combat damage to a player, draw (Stealer of Secrets).
    DrawOnHit,
    /// When it deals combat damage to a player, a 1/1 Soldier (Precinct Captain).
    SoldierOnHit,
    /// Whenever it deals damage to a player, draw (Niv-Mizzet).
    DrawOnAnyHit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnCast {
    /// Whenever you cast an instant or sorcery: 2 damage to each opponent (Guttersnipe).
    SpellBurn,
    /// Whenever you cast a multicoloured spell: 2 damage to any target (Pyroconvergence).
    MulticolourBurn,
    /// Whenever you cast a multicoloured spell, untap (Lobber Crew).
    MulticolourUntap,
}

/// When a spell is cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timing {
    /// In its controller's main phase.
    Main,
    /// When the opponent declares attackers.
    Defend,
    /// In answer to the opponent's spell.
    Answer,
    /// At the opponent's end step (flash creatures, card draw).
    EndStep,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub kw: u32,
    pub etb: Vec<Eff>,
    pub dies: Vec<Eff>,
    pub attacks: Vec<Eff>,
    pub statics: Vec<Static>,
    pub on_cast: Option<OnCast>,
    pub activated: Vec<Activated>,
    /// A spell's modes; one mode for most, three for a charm.
    pub modes: Vec<Vec<Eff>>,
    pub overload: Option<(Cost, Vec<Eff>)>,
    pub aura: Option<Aura>,
    /// The aura's or equipment's target picks.
    pub unleash: bool,
    pub scavenge: Option<Cost>,
    /// Mana it adds each turn while untapped: the colours of one unit.
    pub mana: Option<u8>,
    /// Gain this much life when it enters.
    pub timing: Timing,
    /// Equipment: equipped creature gets +1/+0 for each of its colours (Civic Saber).
    pub equip: Option<Cost>,
    /// Gains life: the Selesnya and Azorius cards that heal.
    pub note: &'static str,
}

impl Rules {
    fn base() -> Rules {
        Rules {
            kw: 0,
            etb: Vec::new(),
            dies: Vec::new(),
            attacks: Vec::new(),
            statics: Vec::new(),
            on_cast: None,
            activated: Vec::new(),
            modes: Vec::new(),
            overload: None,
            aura: None,
            unleash: false,
            scavenge: None,
            mana: None,
            timing: Timing::Main,
            equip: None,
            note: "",
        }
    }
}

fn cost(s: &str) -> Cost {
    Cost::parse(s).unwrap_or_default()
}

fn pump(p: i32, t: i32, kwf: u32, who: Who) -> Eff {
    Eff::Pump {
        p,
        t,
        kw: kwf,
        aim: Aim::One(creatures(who)),
        untap: false,
    }
}

fn act(c: &str, tap: bool, sac: bool, effects: Vec<Eff>, when: Use) -> Activated {
    Activated {
        cost: cost(c),
        tap,
        sac,
        effects,
        when,
    }
}

fn spell(effects: Vec<Eff>) -> Rules {
    Rules {
        modes: vec![effects],
        ..Rules::base()
    }
}

fn creature(kwf: u32) -> Rules {
    Rules {
        kw: kwf,
        ..Rules::base()
    }
}

/// The rules for a card by its exact name, or None for a card this region
/// does not use.
pub fn for_card(name: &str) -> Option<Rules> {
    use kw::*;
    let theirs = creatures(Who::Theirs);
    let mine = creatures(Who::Mine);
    let r = match name {
        // lands: they give mana on the map and never enter a deck
        "Plains" | "Island" | "Swamp" | "Mountain" | "Forest" | "Azorius Guildgate" | "Izzet Guildgate"
        | "Rakdos Guildgate" | "Golgari Guildgate" | "Selesnya Guildgate" => Rules::base(),

        // white
        "Trained Caracal" => creature(LIFELINK),
        "Swift Justice" => Rules {
            timing: Timing::Main,
            ..spell(vec![pump(1, 0, FIRST_STRIKE | LIFELINK, Who::Mine)])
        },
        "Azorius Arrester" => Rules {
            etb: vec![Eff::Detain(Aim::One(theirs))],
            ..Rules::base()
        },
        "Concordia Pegasus" => creature(FLYING),
        "Precinct Captain" => Rules {
            statics: vec![Static::SoldierOnHit],
            ..creature(FIRST_STRIKE)
        },
        "Fencing Ace" => creature(DOUBLE_STRIKE),
        "Seller of Songbirds" => Rules {
            etb: vec![Eff::Token(Token::Bird, 1)],
            ..Rules::base()
        },
        "Sunspire Griffin" => creature(FLYING),
        "Eyes in the Skies" => spell(vec![Eff::Token(Token::Bird, 1), Eff::Populate]),
        "Arrest" => Rules {
            aura: Some(Aura::Arrest),
            ..Rules::base()
        },
        "Ethereal Armor" => Rules {
            aura: Some(Aura::EtherealArmor),
            ..Rules::base()
        },
        "Azorius Justiciar" => Rules {
            etb: vec![Eff::Detain(Aim::UpTo(2, theirs))],
            ..Rules::base()
        },
        "Phantom General" => Rules {
            statics: vec![Static::TokenAnthem],
            ..Rules::base()
        },
        "Trostani's Judgment" => spell(vec![Eff::Exile(Aim::One(theirs)), Eff::Populate]),
        "Angel of Serenity" => Rules {
            etb: vec![Eff::AngelExile],
            ..creature(FLYING)
        },

        // blue
        "Downsize" => Rules {
            timing: Timing::Defend,
            overload: Some((cost("{2}{U}"), vec![pump(-4, 0, 0, Who::Theirs).each()])),
            ..spell(vec![pump(-4, 0, 0, Who::Theirs)])
        },
        "Blustersquall" => Rules {
            timing: Timing::Defend,
            overload: Some((cost("{3}{U}"), vec![Eff::Tap(Aim::Each(theirs))])),
            ..spell(vec![Eff::Tap(Aim::One(theirs))])
        },
        "Faerie Impostor" => Rules {
            etb: vec![Eff::ImpostorReturn],
            ..creature(FLYING)
        },
        "Inaction Injunction" => spell(vec![Eff::Detain(Aim::One(theirs)), Eff::Draw(1)]),
        "Cyclonic Rift" => Rules {
            overload: Some((
                cost("{6}{U}"),
                vec![Eff::Bounce(Aim::Each(permanents(Who::Theirs)))],
            )),
            ..spell(vec![Eff::Bounce(Aim::One(permanents(Who::Theirs)))])
        },
        "Stealer of Secrets" => Rules {
            statics: vec![Static::DrawOnHit],
            ..Rules::base()
        },
        "Cancel" => Rules {
            timing: Timing::Answer,
            ..spell(vec![Eff::Counter {
                creature: None,
                pay: 0,
            }])
        },
        "Hover Barrier" => creature(DEFENDER | FLYING),
        "Runewing" => Rules {
            dies: vec![Eff::Draw(1)],
            ..creature(FLYING)
        },
        "Inspiration" => Rules {
            timing: Timing::EndStep,
            ..spell(vec![Eff::Draw(2)])
        },
        "Soulsworn Spirit" => Rules {
            etb: vec![Eff::Detain(Aim::One(theirs))],
            ..creature(UNBLOCKABLE)
        },
        "Voidwielder" => Rules {
            etb: vec![Eff::Bounce(Aim::One(theirs))],
            ..Rules::base()
        },
        "Isperia's Skywatch" => Rules {
            etb: vec![Eff::Detain(Aim::One(theirs))],
            ..creature(FLYING)
        },
        "Skyline Predator" => Rules {
            timing: Timing::EndStep,
            ..creature(FLASH | FLYING)
        },
        "Paralyzing Grasp" => Rules {
            aura: Some(Aura::Grasp),
            ..Rules::base()
        },

        // black
        "Daggerdrome Imp" => creature(FLYING | LIFELINK),
        "Grim Roustabout" => Rules {
            unleash: true,
            activated: vec![act("{1}{B}", false, false, Vec::new(), Use::Regenerate)],
            ..Rules::base()
        },
        "Thrill-Kill Assassin" => Rules {
            unleash: true,
            ..creature(DEATHTOUCH)
        },
        "Ultimate Price" => spell(vec![Eff::Destroy(Aim::One(Filter { mono: true, ..theirs }))]),
        "Dead Reveler" => Rules {
            unleash: true,
            ..Rules::base()
        },
        "Sewer Shambler" => Rules {
            scavenge: Some(cost("{2}{B}")),
            ..creature(SWAMPWALK)
        },
        "Stab Wound" => Rules {
            aura: Some(Aura::StabWound),
            ..Rules::base()
        },
        "Mind Rot" => spell(vec![Eff::OppDiscard(2)]),
        "Perilous Shadow" => Rules {
            activated: vec![act(
                "{1}{B}",
                false,
                false,
                vec![pump(2, 2, 0, Who::Mine)],
                Use::Pump,
            )],
            ..Rules::base()
        },
        "Dark Revenant" => Rules {
            dies: vec![Eff::ToLibraryTop],
            ..creature(FLYING)
        },
        "Slum Reaper" => Rules {
            etb: vec![Eff::EachSacrifices],
            ..Rules::base()
        },
        "Catacomb Slug" => Rules::base(),
        "Zanikev Locust" => Rules {
            scavenge: Some(cost("{2}{B}{B}")),
            ..creature(FLYING)
        },
        "Shrieking Affliction" => Rules {
            statics: vec![Static::Shrieking],
            ..Rules::base()
        },
        "Assassin's Strike" => spell(vec![Eff::Destroy(Aim::One(theirs)), Eff::OppDiscard(1)]),

        // red
        "Bellows Lizard" => Rules {
            activated: vec![act(
                "{1}{R}",
                false,
                false,
                vec![pump(1, 0, 0, Who::Mine)],
                Use::Pump,
            )],
            ..Rules::base()
        },
        "Electrickery" => Rules {
            overload: Some((cost("{1}{R}"), vec![Eff::Damage(1, Aim::Each(theirs))])),
            ..spell(vec![Eff::Damage(1, Aim::One(theirs))])
        },
        "Gore-House Chainwalker" => Rules {
            unleash: true,
            ..Rules::base()
        },
        "Ash Zealot" => creature(FIRST_STRIKE | HASTE),
        "Mizzium Mortars" => Rules {
            overload: Some((cost("{3}{R}{R}{R}"), vec![Eff::Damage(4, Aim::Each(theirs))])),
            ..spell(vec![Eff::Damage(4, Aim::One(theirs))])
        },
        "Lobber Crew" => Rules {
            on_cast: Some(OnCast::MulticolourUntap),
            activated: vec![act("", true, false, vec![Eff::Drain(1)], Use::Ping)],
            ..creature(DEFENDER)
        },
        "Splatter Thug" => Rules {
            unleash: true,
            ..creature(FIRST_STRIKE)
        },
        "Annihilating Fire" => spell(vec![Eff::Damage(3, Aim::CreatureOrFace(creatures(Who::Theirs)))]),
        "Guttersnipe" => Rules {
            on_cast: Some(OnCast::SpellBurn),
            ..Rules::base()
        },
        "Cobblebrute" => Rules::base(),
        "Bloodfray Giant" => Rules {
            unleash: true,
            ..creature(TRAMPLE)
        },
        "Goblin Rally" => spell(vec![Eff::Token(Token::Goblin, 4)]),
        "Tenement Crasher" => creature(HASTE),
        "Explosive Impact" => spell(vec![Eff::Damage(5, Aim::CreatureOrFace(creatures(Who::Theirs)))]),
        "Pyroconvergence" => Rules {
            on_cast: Some(OnCast::MulticolourBurn),
            ..Rules::base()
        },

        // green
        "Centaur's Herald" => Rules {
            activated: vec![act(
                "{2}{G}",
                false,
                true,
                vec![Eff::Token(Token::Centaur, 1)],
                Use::SacForToken,
            )],
            ..Rules::base()
        },
        "Giant Growth" => spell(vec![pump(3, 3, 0, Who::Mine)]),
        "Drudge Beetle" => Rules {
            scavenge: Some(cost("{5}{G}")),
            ..Rules::base()
        },
        "Brushstrider" => creature(VIGILANCE),
        "Savage Surge" => spell(vec![Eff::Pump {
            p: 2,
            t: 2,
            kw: 0,
            aim: Aim::One(mine),
            untap: true,
        }]),
        "Aerial Predation" => spell(vec![
            Eff::Destroy(Aim::One(Filter {
                flying: Some(true),
                ..theirs
            })),
            Eff::Gain(2),
        ]),
        "Korozda Monitor" => Rules {
            scavenge: Some(cost("{5}{G}{G}")),
            ..creature(TRAMPLE)
        },
        "Towering Indrik" => creature(REACH),
        "Chorus of Might" => spell(vec![Eff::Chorus(Aim::One(mine))]),
        "Deadbridge Goliath" => Rules {
            scavenge: Some(cost("{4}{G}{G}")),
            ..Rules::base()
        },
        "Rubbleback Rhino" => creature(HEXPROOF),
        "Axebane Stag" => Rules::base(),
        "Archweaver" => creature(REACH | TRAMPLE),
        "Horncaller's Chant" => spell(vec![Eff::Token(Token::Rhino, 1), Eff::Populate]),
        "Death's Presence" => Rules {
            statics: vec![Static::DeathsPresence],
            ..Rules::base()
        },

        // Azorius
        "Vassal Soul" => creature(FLYING),
        "Lyev Skyknight" => Rules {
            etb: vec![Eff::Detain(Aim::One(permanents(Who::Theirs)))],
            ..creature(FLYING)
        },
        "Detention Sphere" => Rules {
            etb: vec![Eff::SphereExile],
            ..Rules::base()
        },
        "Azorius Keyrune" => Rules {
            mana: Some(W | U),
            activated: vec![act("{W}{U}", false, false, Vec::new(), Use::Animate)],
            ..Rules::base()
        },

        // Izzet
        "Niv-Mizzet, Dracogenius" => Rules {
            statics: vec![Static::DrawOnAnyHit],
            activated: vec![act(
                "{U}{R}",
                false,
                false,
                vec![Eff::Damage(1, Aim::CreatureOrFace(creatures(Who::Theirs)))],
                Use::Ping,
            )],
            ..creature(FLYING)
        },
        "Frostburn Weird" => Rules {
            activated: vec![Activated {
                cost: Cost::of(&[Sym::Hybrid(U, R)]),
                tap: false,
                sac: false,
                effects: vec![pump(1, -1, 0, Who::Mine)],
                when: Use::Pump,
            }],
            ..Rules::base()
        },
        "Goblin Electromancer" => Rules {
            statics: vec![Static::SpellsCheaper],
            ..Rules::base()
        },
        "Izzet Charm" => Rules {
            modes: vec![
                vec![Eff::Counter {
                    creature: Some(false),
                    pay: 2,
                }],
                vec![Eff::Damage(2, Aim::One(theirs))],
                vec![Eff::Loot(2, 2)],
            ],
            ..Rules::base()
        },

        // Rakdos
        "Rakdos Cackler" => Rules {
            unleash: true,
            ..Rules::base()
        },
        "Rakdos Shred-Freak" => creature(HASTE),
        "Dreadbore" => spell(vec![Eff::Destroy(Aim::One(theirs))]),
        "Carnival Hellsteed" => Rules {
            unleash: true,
            ..creature(FIRST_STRIKE | HASTE)
        },

        // Golgari
        "Dreg Mangler" => Rules {
            scavenge: Some(cost("{3}{B}{G}")),
            ..creature(HASTE)
        },
        "Sluiceway Scorpion" => Rules {
            scavenge: Some(cost("{1}{B}{G}")),
            ..creature(DEATHTOUCH)
        },
        "Abrupt Decay" => Rules {
            kw: CANT_BE_COUNTERED,
            ..spell(vec![Eff::Destroy(Aim::One(Filter {
                max_value: Some(3),
                ..permanents(Who::Theirs)
            }))])
        },
        "Golgari Keyrune" => Rules {
            mana: Some(B | G),
            activated: vec![act("{B}{G}", false, false, Vec::new(), Use::Animate)],
            ..Rules::base()
        },

        // Selesnya
        "Dryad Militant" => Rules {
            statics: vec![Static::SpellsToExile],
            ..Rules::base()
        },
        "Centaur Healer" => Rules {
            etb: vec![Eff::Gain(3)],
            ..Rules::base()
        },
        "Selesnya Charm" => Rules {
            modes: vec![
                vec![pump(2, 2, TRAMPLE, Who::Mine)],
                vec![Eff::Exile(Aim::One(Filter {
                    min_power: Some(5),
                    ..theirs
                }))],
                vec![Eff::Token(Token::Knight, 1)],
            ],
            ..Rules::base()
        },
        "Growing Ranks" => Rules {
            statics: vec![Static::UpkeepPopulate],
            ..Rules::base()
        },

        // colourless
        "Chromatic Lantern" => Rules {
            statics: vec![Static::Lantern],
            mana: Some(W | U | B | R | G),
            ..Rules::base()
        },
        "Civic Saber" => Rules {
            equip: Some(cost("{1}")),
            ..Rules::base()
        },
        "Street Sweeper" => Rules::base(),
        _ => return None,
    };
    Some(r)
}

impl Eff {
    /// The same pump aimed at each matching creature (overload).
    fn each(self) -> Eff {
        match self {
            Eff::Pump { p, t, kw, aim, untap } => Eff::Pump {
                p,
                t,
                kw,
                aim: match aim {
                    Aim::One(f) => Aim::Each(f),
                    other => other,
                },
                untap,
            },
            other => other,
        }
    }
}

/// The colours a land's unit can be, by its name, or None for a card that
/// is not a land.
pub fn land_unit(name: &str) -> Option<u8> {
    Some(match name {
        "Plains" => W,
        "Island" => U,
        "Swamp" => B,
        "Mountain" => R,
        "Forest" => G,
        "Azorius Guildgate" => W | U,
        "Izzet Guildgate" => U | R,
        "Rakdos Guildgate" => B | R,
        "Golgari Guildgate" => B | G,
        "Selesnya Guildgate" => G | W,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overload_turns_one_target_into_each() {
        let r = for_card("Downsize").unwrap();
        let (c, e) = r.overload.unwrap();
        assert_eq!(c.value(), 3);
        assert!(matches!(
            e[0],
            Eff::Pump {
                aim: Aim::Each(_),
                p: -4,
                ..
            }
        ));
        assert_eq!(r.timing, Timing::Defend);
    }

    #[test]
    fn a_guildgate_gives_either_of_its_guilds_colours() {
        assert_eq!(land_unit("Izzet Guildgate"), Some(U | R));
        assert_eq!(land_unit("Forest"), Some(G));
        assert_eq!(land_unit("Cobblebrute"), None);
    }

    #[test]
    fn an_unknown_card_has_no_rules() {
        assert!(for_card("Worldspine Wurm").is_none());
        assert!(for_card("Niv-Mizzet, Dracogenius").unwrap().kw & kw::FLYING != 0);
    }
}
