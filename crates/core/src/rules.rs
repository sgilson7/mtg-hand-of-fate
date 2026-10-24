//! What each card does in a game of Magic: one definition per card of the
//! first region, written against its Oracle text in `data/cards.json`. A
//! card in a pool with no definition here is refused when the content loads
//! (rule MTG-5), so no card can sit in a deck doing nothing by accident.
//!
//! A definition is data the duel engine (`duel.rs`) reads: keywords, what a
//! spell does and what it targets, triggered abilities (enters, dies,
//! attacks, deals combat damage, upkeep, casting a spell), activated
//! abilities with their costs, mana abilities, statics, auras, equipment,
//! and the guild mechanics (detain, overload, unleash, scavenge, populate).
//! Where a card's text points at something no card of this region can do
//! (casting from a graveyard, auras on lands), the definition still says
//! what the card does, and SECOND-ORDER.md lists it.

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

/// Whose permanents a target may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    /// "you control"
    Mine,
    /// "you don't control", "an opponent controls"
    Theirs,
    Any,
}

/// Which permanents a target or an "each" may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filter {
    pub who: Who,
    /// Creatures only; false means any nonland permanent.
    pub creature: bool,
    pub max_value: Option<usize>,
    pub min_power: Option<i32>,
    /// Some(true): with flying only.
    pub flying: Option<bool>,
    pub mono: bool,
    /// Lands only (Street Sweeper).
    pub land: bool,
    /// Not named Detention Sphere.
    pub not_sphere: bool,
    /// Another one: not the source itself.
    pub other: bool,
}

pub const fn creatures(who: Who) -> Filter {
    Filter {
        who,
        creature: true,
        max_value: None,
        min_power: None,
        flying: None,
        mono: false,
        land: false,
        not_sphere: false,
        other: false,
    }
}

pub const fn permanents(who: Who) -> Filter {
    Filter {
        creature: false,
        ..creatures(who)
    }
}

/// What a spell or ability targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tgt {
    /// One permanent.
    One(Filter),
    /// Up to n permanents (n of them, or fewer, or none: "you may").
    UpTo(u8, Filter),
    /// A creature matching the filter, or a player ("any target").
    Any(Filter),
    /// A player.
    Player,
    /// A spell on the stack; Some(false): a noncreature spell.
    Spell(Option<bool>),
    /// Up to three other creatures on the battlefield and/or creature
    /// cards in graveyards (Angel of Serenity).
    Angel,
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

/// One thing a spell or ability does. Effects that name no target apply to
/// the targets chosen for it, or, for an overloaded spell, to each
/// permanent its filter matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eff {
    Damage(i32),
    Destroy,
    Exile,
    Bounce,
    Detain,
    Tap,
    Untap,
    /// +p/+t and keywords until end of turn.
    Pump(i32, i32, u32),
    /// The source gets +p/+t until end of turn.
    PumpSelf(i32, i32),
    /// +1/+1 counters.
    Counters(i32),
    /// Until end of turn, +1/+1 for each creature you control, and trample (Chorus of Might).
    Chorus,
    /// A creature dealt damage this way that would die this turn is exiled instead.
    ExileIfDies,
    Token(Token, u8),
    Populate,
    /// You draw.
    Draw(u8),
    /// The target player draws.
    TargetDraws(u8),
    Gain(i32),
    /// Damage to each opponent.
    DamageEachOpponent(i32),
    /// The target player discards; they choose.
    TargetDiscards(u8),
    /// The controller of the targeted permanent discards (Assassin's Strike).
    ItsControllerDiscards(u8),
    Counter,
    /// Counter it unless its controller pays this much generic mana.
    CounterUnlessPays(u8),
    /// Draw, then discard.
    Loot(u8, u8),
    /// Each player sacrifices a creature of their choice (Slum Reaper).
    EachSacrifices,
    /// Sacrifice the source unless you return another creature you control (Faerie Impostor).
    ImpostorReturn,
    /// Exile the targets until the source leaves (Angel of Serenity: to hand; Detention Sphere: to the battlefield).
    ExileHeld,
    /// Also exile every other permanent with the target's name (Detention Sphere).
    ExileHeldSameName,
    /// Put the source card on top of its owner's library (Dark Revenant).
    ToLibraryTop,
    /// A regeneration shield on the source.
    Regenerate,
    /// The source keyrune becomes a 2/2 creature until end of turn.
    Animate,
    /// Attach the source equipment to the target.
    Attach,
    /// Destroy all Auras attached to the target (Street Sweeper).
    DestroyAurasOn,
    /// X +1/+1 counters, X being the power of the creature that died (Death's Presence).
    CountersFromDied,
}

/// What a spell does, or one mode of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mode {
    pub target: Option<Tgt>,
    pub effects: Vec<Eff>,
}

/// A triggered or activated ability's effect and target.
pub type Ability = Mode;

/// Hints for the computer player: when an activated ability is worth using.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    Pump,
    Ping,
    Regenerate,
    SacForToken,
    Animate,
    Equip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activated {
    pub cost: Cost,
    pub tap: bool,
    pub sac: bool,
    pub ability: Ability,
    /// Only as a sorcery (equip).
    pub sorcery: bool,
    pub when: Use,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aura {
    /// Can't attack or block, no activated abilities (Arrest).
    Arrest,
    /// Doesn't untap (Paralyzing Grasp).
    Grasp,
    /// -2/-2, its controller loses 2 at their upkeep (Stab Wound).
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
    /// Instant and sorcery cards go to exile instead of a graveyard (Dryad Militant).
    SpellsToExile,
    /// Lands you control tap for any colour (Chromatic Lantern).
    Lantern,
    /// Whenever a player casts a spell from a graveyard, 3 damage to them
    /// (Ash Zealot). No card of this region casts from a graveyard.
    GraveyardCastPunish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnCast {
    /// Whenever you cast an instant or sorcery: 2 damage to each opponent (Guttersnipe).
    SpellBurn,
    /// Whenever you cast a multicoloured spell: 2 damage to any target (Pyroconvergence).
    MulticolourBurn,
    /// Whenever you cast a multicoloured spell, untap this (Lobber Crew).
    MulticolourUntap,
}

/// What happens when the source leaves the battlefield to the cards it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    ToHand,
    ToBattlefield,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub kw: u32,
    /// A spell's modes: one for most, three for a charm.
    pub modes: Vec<Mode>,
    pub overload: Option<Cost>,
    pub etb: Option<Ability>,
    pub dies: Option<Ability>,
    pub attacks: Option<Ability>,
    /// Whenever this deals combat damage to a player.
    pub hits: Option<Ability>,
    /// Whenever this deals any damage to a player ("you may draw": Niv-Mizzet).
    pub damages_player: Option<Ability>,
    /// At the beginning of your upkeep.
    pub upkeep: Option<Ability>,
    /// At the beginning of each opponent's upkeep, if they hold this many cards or fewer: they lose 3 (Shrieking Affliction).
    pub opponent_upkeep: Option<(usize, i32)>,
    /// Whenever a creature you control dies.
    pub ally_dies: Option<Ability>,
    pub release: Option<Release>,
    pub on_cast: Option<OnCast>,
    pub statics: Vec<Static>,
    pub activated: Vec<Activated>,
    /// The colours one tap for mana can be.
    pub mana: Option<u8>,
    pub aura: Option<Aura>,
    pub equip: Option<Cost>,
    pub unleash: bool,
    pub scavenge: Option<Cost>,
    /// An animated keyrune's keywords and colours.
    pub animate: Option<(u32, u8)>,
}

impl Rules {
    fn base() -> Rules {
        Rules {
            kw: 0,
            modes: Vec::new(),
            overload: None,
            etb: None,
            dies: None,
            attacks: None,
            hits: None,
            damages_player: None,
            upkeep: None,
            opponent_upkeep: None,
            ally_dies: None,
            release: None,
            on_cast: None,
            statics: Vec::new(),
            activated: Vec::new(),
            mana: None,
            aura: None,
            equip: None,
            unleash: false,
            scavenge: None,
            animate: None,
        }
    }

    /// Every effect of the card's spell, abilities and triggers, for the
    /// readings and for the computer player.
    pub fn all_effects(&self) -> Vec<&Eff> {
        let mut out: Vec<&Eff> = Vec::new();
        for m in &self.modes {
            out.extend(m.effects.iter());
        }
        for a in [
            &self.etb,
            &self.dies,
            &self.attacks,
            &self.hits,
            &self.damages_player,
            &self.upkeep,
            &self.ally_dies,
        ]
        .into_iter()
        .flatten()
        {
            out.extend(a.effects.iter());
        }
        for a in &self.activated {
            out.extend(a.ability.effects.iter());
        }
        out
    }

    pub fn detains(&self) -> bool {
        self.all_effects().contains(&&Eff::Detain)
    }

    pub fn populates(&self) -> bool {
        self.all_effects().contains(&&Eff::Populate)
    }
}

fn cost(s: &str) -> Cost {
    Cost::parse(s).unwrap_or_default()
}

fn mode(target: Option<Tgt>, effects: Vec<Eff>) -> Mode {
    Mode { target, effects }
}

fn on(t: Tgt, effects: Vec<Eff>) -> Option<Ability> {
    Some(mode(Some(t), effects))
}

fn plain(effects: Vec<Eff>) -> Option<Ability> {
    Some(mode(None, effects))
}

fn spell(target: Option<Tgt>, effects: Vec<Eff>) -> Rules {
    Rules {
        modes: vec![mode(target, effects)],
        ..Rules::base()
    }
}

fn creature(kwf: u32) -> Rules {
    Rules {
        kw: kwf,
        ..Rules::base()
    }
}

fn act(c: &str, tap: bool, sac: bool, ability: Ability, when: Use) -> Activated {
    Activated {
        cost: cost(c),
        tap,
        sac,
        ability,
        sorcery: false,
        when,
    }
}

/// The rules for a card by its exact name, or None for a card this region
/// does not use.
pub fn for_card(name: &str) -> Option<Rules> {
    use kw::*;
    let theirs = creatures(Who::Theirs);
    let any = creatures(Who::Any);
    let mine = creatures(Who::Mine);
    let r = match name {
        // lands: played from the land pile, tapped for mana
        "Plains" | "Island" | "Swamp" | "Mountain" | "Forest" | "Azorius Guildgate" | "Izzet Guildgate"
        | "Rakdos Guildgate" | "Golgari Guildgate" | "Selesnya Guildgate" => Rules {
            mana: land_unit(name),
            ..Rules::base()
        },

        // white
        "Trained Caracal" => creature(LIFELINK),
        "Swift Justice" => spell(
            Some(Tgt::One(any)),
            vec![Eff::Pump(1, 0, FIRST_STRIKE | LIFELINK)],
        ),
        "Azorius Arrester" => Rules {
            etb: on(Tgt::One(theirs), vec![Eff::Detain]),
            ..Rules::base()
        },
        "Concordia Pegasus" => creature(FLYING),
        "Precinct Captain" => Rules {
            hits: plain(vec![Eff::Token(Token::Soldier, 1)]),
            ..creature(FIRST_STRIKE)
        },
        "Fencing Ace" => creature(DOUBLE_STRIKE),
        "Seller of Songbirds" => Rules {
            etb: plain(vec![Eff::Token(Token::Bird, 1)]),
            ..Rules::base()
        },
        "Sunspire Griffin" => creature(FLYING),
        "Eyes in the Skies" => spell(None, vec![Eff::Token(Token::Bird, 1), Eff::Populate]),
        "Arrest" => Rules {
            aura: Some(Aura::Arrest),
            ..spell(Some(Tgt::One(any)), Vec::new())
        },
        "Ethereal Armor" => Rules {
            aura: Some(Aura::EtherealArmor),
            ..spell(Some(Tgt::One(any)), Vec::new())
        },
        "Azorius Justiciar" => Rules {
            etb: on(Tgt::UpTo(2, theirs), vec![Eff::Detain]),
            ..Rules::base()
        },
        "Phantom General" => Rules {
            statics: vec![Static::TokenAnthem],
            ..Rules::base()
        },
        "Trostani's Judgment" => spell(Some(Tgt::One(any)), vec![Eff::Exile, Eff::Populate]),
        "Angel of Serenity" => Rules {
            etb: on(Tgt::Angel, vec![Eff::ExileHeld]),
            release: Some(Release::ToHand),
            ..creature(FLYING)
        },

        // blue
        "Downsize" => Rules {
            overload: Some(cost("{2}{U}")),
            ..spell(Some(Tgt::One(theirs)), vec![Eff::Pump(-4, 0, 0)])
        },
        "Blustersquall" => Rules {
            overload: Some(cost("{3}{U}")),
            ..spell(Some(Tgt::One(theirs)), vec![Eff::Tap])
        },
        "Faerie Impostor" => Rules {
            etb: plain(vec![Eff::ImpostorReturn]),
            ..creature(FLYING)
        },
        "Inaction Injunction" => spell(Some(Tgt::One(theirs)), vec![Eff::Detain, Eff::Draw(1)]),
        "Cyclonic Rift" => Rules {
            overload: Some(cost("{6}{U}")),
            ..spell(Some(Tgt::One(permanents(Who::Theirs))), vec![Eff::Bounce])
        },
        "Stealer of Secrets" => Rules {
            hits: plain(vec![Eff::Draw(1)]),
            ..Rules::base()
        },
        "Cancel" => spell(Some(Tgt::Spell(None)), vec![Eff::Counter]),
        "Hover Barrier" => creature(DEFENDER | FLYING),
        "Runewing" => Rules {
            dies: plain(vec![Eff::Draw(1)]),
            ..creature(FLYING)
        },
        "Inspiration" => spell(Some(Tgt::Player), vec![Eff::TargetDraws(2)]),
        "Soulsworn Spirit" => Rules {
            etb: on(Tgt::One(theirs), vec![Eff::Detain]),
            ..creature(UNBLOCKABLE)
        },
        "Voidwielder" => Rules {
            etb: on(Tgt::UpTo(1, any), vec![Eff::Bounce]),
            ..Rules::base()
        },
        "Isperia's Skywatch" => Rules {
            etb: on(Tgt::One(theirs), vec![Eff::Detain]),
            ..creature(FLYING)
        },
        "Skyline Predator" => creature(FLASH | FLYING),
        "Paralyzing Grasp" => Rules {
            aura: Some(Aura::Grasp),
            ..spell(Some(Tgt::One(any)), Vec::new())
        },

        // black
        "Daggerdrome Imp" => creature(FLYING | LIFELINK),
        "Grim Roustabout" => Rules {
            unleash: true,
            activated: vec![act(
                "{1}{B}",
                false,
                false,
                mode(None, vec![Eff::Regenerate]),
                Use::Regenerate,
            )],
            ..Rules::base()
        },
        "Thrill-Kill Assassin" => Rules {
            unleash: true,
            ..creature(DEATHTOUCH)
        },
        "Ultimate Price" => spell(Some(Tgt::One(Filter { mono: true, ..any })), vec![Eff::Destroy]),
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
            ..spell(Some(Tgt::One(any)), Vec::new())
        },
        "Mind Rot" => spell(Some(Tgt::Player), vec![Eff::TargetDiscards(2)]),
        "Perilous Shadow" => Rules {
            activated: vec![act(
                "{1}{B}",
                false,
                false,
                mode(None, vec![Eff::PumpSelf(2, 2)]),
                Use::Pump,
            )],
            ..Rules::base()
        },
        "Dark Revenant" => Rules {
            dies: plain(vec![Eff::ToLibraryTop]),
            ..creature(FLYING)
        },
        "Slum Reaper" => Rules {
            etb: plain(vec![Eff::EachSacrifices]),
            ..Rules::base()
        },
        "Catacomb Slug" => Rules::base(),
        "Zanikev Locust" => Rules {
            scavenge: Some(cost("{2}{B}{B}")),
            ..creature(FLYING)
        },
        "Shrieking Affliction" => Rules {
            opponent_upkeep: Some((1, 3)),
            ..Rules::base()
        },
        "Assassin's Strike" => spell(
            Some(Tgt::One(any)),
            vec![Eff::Destroy, Eff::ItsControllerDiscards(1)],
        ),

        // red
        "Bellows Lizard" => Rules {
            activated: vec![act(
                "{1}{R}",
                false,
                false,
                mode(None, vec![Eff::PumpSelf(1, 0)]),
                Use::Pump,
            )],
            ..Rules::base()
        },
        "Electrickery" => Rules {
            overload: Some(cost("{1}{R}")),
            ..spell(Some(Tgt::One(theirs)), vec![Eff::Damage(1)])
        },
        "Gore-House Chainwalker" => Rules {
            unleash: true,
            ..Rules::base()
        },
        "Ash Zealot" => Rules {
            statics: vec![Static::GraveyardCastPunish],
            ..creature(FIRST_STRIKE | HASTE)
        },
        "Mizzium Mortars" => Rules {
            overload: Some(cost("{3}{R}{R}{R}")),
            ..spell(Some(Tgt::One(theirs)), vec![Eff::Damage(4)])
        },
        "Lobber Crew" => Rules {
            on_cast: Some(OnCast::MulticolourUntap),
            activated: vec![act(
                "",
                true,
                false,
                mode(None, vec![Eff::DamageEachOpponent(1)]),
                Use::Ping,
            )],
            ..creature(DEFENDER)
        },
        "Splatter Thug" => Rules {
            unleash: true,
            ..creature(FIRST_STRIKE)
        },
        "Annihilating Fire" => spell(Some(Tgt::Any(any)), vec![Eff::Damage(3), Eff::ExileIfDies]),
        "Guttersnipe" => Rules {
            on_cast: Some(OnCast::SpellBurn),
            ..Rules::base()
        },
        "Cobblebrute" => Rules::base(),
        "Bloodfray Giant" => Rules {
            unleash: true,
            ..creature(TRAMPLE)
        },
        "Goblin Rally" => spell(None, vec![Eff::Token(Token::Goblin, 4)]),
        "Tenement Crasher" => creature(HASTE),
        "Explosive Impact" => spell(Some(Tgt::Any(any)), vec![Eff::Damage(5)]),
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
                mode(None, vec![Eff::Token(Token::Centaur, 1)]),
                Use::SacForToken,
            )],
            ..Rules::base()
        },
        "Giant Growth" => spell(Some(Tgt::One(any)), vec![Eff::Pump(3, 3, 0)]),
        "Drudge Beetle" => Rules {
            scavenge: Some(cost("{5}{G}")),
            ..Rules::base()
        },
        "Brushstrider" => creature(VIGILANCE),
        "Savage Surge" => spell(Some(Tgt::One(any)), vec![Eff::Pump(2, 2, 0), Eff::Untap]),
        "Aerial Predation" => spell(
            Some(Tgt::One(Filter {
                flying: Some(true),
                ..any
            })),
            vec![Eff::Destroy, Eff::Gain(2)],
        ),
        "Korozda Monitor" => Rules {
            scavenge: Some(cost("{5}{G}{G}")),
            ..creature(TRAMPLE)
        },
        "Towering Indrik" => creature(REACH),
        "Chorus of Might" => spell(Some(Tgt::One(any)), vec![Eff::Chorus]),
        "Deadbridge Goliath" => Rules {
            scavenge: Some(cost("{4}{G}{G}")),
            ..Rules::base()
        },
        "Rubbleback Rhino" => creature(HEXPROOF),
        "Axebane Stag" => Rules::base(),
        "Archweaver" => creature(REACH | TRAMPLE),
        "Horncaller's Chant" => spell(None, vec![Eff::Token(Token::Rhino, 1), Eff::Populate]),
        "Death's Presence" => Rules {
            ally_dies: on(Tgt::One(mine), vec![Eff::CountersFromDied]),
            ..Rules::base()
        },

        // Azorius
        "Vassal Soul" => creature(FLYING),
        "Lyev Skyknight" => Rules {
            etb: on(Tgt::One(permanents(Who::Theirs)), vec![Eff::Detain]),
            ..creature(FLYING)
        },
        "Detention Sphere" => Rules {
            etb: on(
                Tgt::UpTo(
                    1,
                    Filter {
                        not_sphere: true,
                        ..permanents(Who::Any)
                    },
                ),
                vec![Eff::ExileHeldSameName],
            ),
            release: Some(Release::ToBattlefield),
            ..Rules::base()
        },
        "Azorius Keyrune" => Rules {
            mana: Some(W | U),
            animate: Some((FLYING, W | U)),
            activated: vec![act(
                "{W}{U}",
                false,
                false,
                mode(None, vec![Eff::Animate]),
                Use::Animate,
            )],
            ..Rules::base()
        },

        // Izzet
        "Niv-Mizzet, Dracogenius" => Rules {
            damages_player: plain(vec![Eff::Draw(1)]),
            activated: vec![act(
                "{U}{R}",
                false,
                false,
                mode(Some(Tgt::Any(any)), vec![Eff::Damage(1)]),
                Use::Ping,
            )],
            ..creature(FLYING)
        },
        "Frostburn Weird" => Rules {
            activated: vec![Activated {
                cost: Cost::of(&[Sym::Hybrid(U, R)]),
                tap: false,
                sac: false,
                ability: mode(None, vec![Eff::PumpSelf(1, -1)]),
                sorcery: false,
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
                mode(Some(Tgt::Spell(Some(false))), vec![Eff::CounterUnlessPays(2)]),
                mode(Some(Tgt::One(any)), vec![Eff::Damage(2)]),
                mode(None, vec![Eff::Loot(2, 2)]),
            ],
            ..Rules::base()
        },

        // Rakdos
        "Rakdos Cackler" => Rules {
            unleash: true,
            ..Rules::base()
        },
        "Rakdos Shred-Freak" => creature(HASTE),
        "Dreadbore" => spell(Some(Tgt::One(any)), vec![Eff::Destroy]),
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
            ..spell(
                Some(Tgt::One(Filter {
                    max_value: Some(3),
                    ..permanents(Who::Any)
                })),
                vec![Eff::Destroy],
            )
        },
        "Golgari Keyrune" => Rules {
            mana: Some(B | G),
            animate: Some((DEATHTOUCH, B | G)),
            activated: vec![act(
                "{B}{G}",
                false,
                false,
                mode(None, vec![Eff::Animate]),
                Use::Animate,
            )],
            ..Rules::base()
        },

        // Selesnya
        "Dryad Militant" => Rules {
            statics: vec![Static::SpellsToExile],
            ..Rules::base()
        },
        "Centaur Healer" => Rules {
            etb: plain(vec![Eff::Gain(3)]),
            ..Rules::base()
        },
        "Selesnya Charm" => Rules {
            modes: vec![
                mode(Some(Tgt::One(any)), vec![Eff::Pump(2, 2, TRAMPLE)]),
                mode(
                    Some(Tgt::One(Filter {
                        min_power: Some(5),
                        ..any
                    })),
                    vec![Eff::Exile],
                ),
                mode(None, vec![Eff::Token(Token::Knight, 1)]),
            ],
            ..Rules::base()
        },
        "Growing Ranks" => Rules {
            upkeep: plain(vec![Eff::Populate]),
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
            activated: vec![Activated {
                cost: cost("{1}"),
                tap: false,
                sac: false,
                ability: mode(Some(Tgt::One(mine)), vec![Eff::Attach]),
                sorcery: true,
                when: Use::Equip,
            }],
            ..Rules::base()
        },
        "Street Sweeper" => Rules {
            attacks: on(
                Tgt::One(Filter {
                    land: true,
                    creature: false,
                    ..permanents(Who::Any)
                }),
                vec![Eff::DestroyAurasOn],
            ),
            ..Rules::base()
        },
        _ => return None,
    };
    Some(r)
}

/// The colours a land's mana can be, by its name, or None for a card that
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
    fn overload_is_an_alternative_cost_on_a_targeted_spell() {
        let r = for_card("Downsize").unwrap();
        assert_eq!(r.overload.unwrap().value(), 3);
        assert_eq!(r.modes[0].effects, vec![Eff::Pump(-4, 0, 0)]);
        assert!(matches!(
            r.modes[0].target,
            Some(Tgt::One(Filter { who: Who::Theirs, .. }))
        ));
    }

    #[test]
    fn a_guildgate_gives_either_of_its_guilds_colours() {
        assert_eq!(land_unit("Izzet Guildgate"), Some(U | R));
        assert_eq!(land_unit("Forest"), Some(G));
        assert_eq!(land_unit("Cobblebrute"), None);
        assert_eq!(for_card("Izzet Guildgate").unwrap().mana, Some(U | R));
    }

    #[test]
    fn an_unknown_card_has_no_rules() {
        assert!(for_card("Worldspine Wurm").is_none());
        assert!(for_card("Niv-Mizzet, Dracogenius").unwrap().kw & kw::FLYING != 0);
    }

    #[test]
    fn a_charm_has_three_modes_and_giant_growth_targets_any_creature() {
        assert_eq!(for_card("Izzet Charm").unwrap().modes.len(), 3);
        let g = for_card("Giant Growth").unwrap();
        assert!(matches!(
            g.modes[0].target,
            Some(Tgt::One(Filter { who: Who::Any, .. }))
        ));
    }
}
