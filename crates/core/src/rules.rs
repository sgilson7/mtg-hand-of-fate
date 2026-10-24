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
    /// Can be blocked only by artifact creatures and creatures that share a colour with it.
    pub const INTIMIDATE: u32 = 1 << 16;
    /// Attacks this turn if able (until end of turn, from Chemister's Trick).
    pub const MUST_ATTACK: u32 = 1 << 17;
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
    /// Some(true): with flying only; Some(false): without flying only.
    pub flying: Option<bool>,
    pub mono: bool,
    /// Lands only.
    pub land: bool,
    /// Not named Detention Sphere.
    pub not_sphere: bool,
    /// Another one: not the source itself.
    pub other: bool,
    pub artifact: bool,
    pub enchantment: bool,
    /// An artifact or an enchantment.
    pub art_or_ench: bool,
    /// Attacking or blocking.
    pub combat: bool,
    pub attacking: bool,
    pub blocking: bool,
    /// Dealt damage this turn.
    pub damaged: bool,
    pub max_toughness: Option<i32>,
    pub nontoken: bool,
    /// Creatures and planeswalkers both.
    pub or_walker: bool,
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
        artifact: false,
        enchantment: false,
        art_or_ench: false,
        combat: false,
        attacking: false,
        blocking: false,
        damaged: false,
        max_toughness: None,
        nontoken: false,
        or_walker: false,
    }
}

pub const fn permanents(who: Who) -> Filter {
    Filter {
        creature: false,
        ..creatures(who)
    }
}

pub const fn lands(who: Who) -> Filter {
    Filter {
        land: true,
        ..permanents(who)
    }
}

/// Which spells a counterspell may target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellKind {
    Any,
    OnlyInstants,
    InstantOrSorcery,
    Creature,
    Noncreature,
    /// A spell you don't control.
    NotYours,
}

/// Which cards in a graveyard a target may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraveKind {
    Any,
    Creature,
    Land,
    InstantOrSorcery,
}

/// What a spell or ability targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tgt {
    /// One permanent.
    One(Filter),
    /// Up to n permanents (n of them, or fewer, or none: "you may").
    UpTo(u8, Filter),
    /// A creature (or planeswalker) matching the filter, or a player ("any target").
    Any(Filter),
    /// A player.
    Player,
    /// An opponent.
    Opponent,
    /// A player or a planeswalker.
    PlayerOrWalker,
    /// A spell on the stack.
    Spell(SpellKind),
    /// Up to three other creatures on the battlefield and/or creature
    /// cards in graveyards (Angel of Serenity).
    Angel,
    /// A card in a graveyard: of this kind, in whose graveyard.
    Grave(GraveKind, Who),
    /// Two different permanents, the first matching one filter and the second the other.
    Two(Filter, Filter),
    /// Twice the same kind of target, which may be the same permanent (Common Bond).
    Twice(Filter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Bird,
    Soldier,
    Goblin,
    Centaur,
    Rhino,
    Knight,
    Wurm,
    Dragon,
    Saproling,
    /// "Whenever this deals combat damage to a player, that player loses the game."
    Assassin,
    /// 8/8 green and white with vigilance (Grove of the Guardian).
    Elemental,
    /// An X/X green Ooze (Slime Molding); its size is held on the permanent.
    Ooze,
    /// A copy of a card (Pack Rat): drawn and played as that card.
    Copy,
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
            Token::Wurm => (5, 5, kw::TRAMPLE, G, "Wurm"),
            Token::Dragon => (6, 6, kw::FLYING, R, "Dragon"),
            Token::Saproling => (1, 1, 0, G, "Saproling"),
            Token::Assassin => (1, 1, 0, B, "Assassin"),
            Token::Elemental => (8, 8, kw::VIGILANCE, G | W, "Elemental"),
            Token::Ooze => (0, 0, 0, G, "Ooze"),
            Token::Copy => (0, 0, 0, 0, "copy"),
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
            Token::Wurm => "token_wurm",
            Token::Dragon => "token_dragon",
            Token::Saproling => "token_saproling",
            Token::Assassin => "token_assassin",
            Token::Elemental => "token_elemental",
            Token::Ooze => "token_ooze",
            Token::Copy => "",
        }
    }

    pub const ALL: [Token; 13] = [
        Token::Bird,
        Token::Soldier,
        Token::Goblin,
        Token::Centaur,
        Token::Rhino,
        Token::Knight,
        Token::Wurm,
        Token::Dragon,
        Token::Saproling,
        Token::Assassin,
        Token::Elemental,
        Token::Ooze,
        Token::Copy,
    ];
}

/// One thing a spell or ability does. Effects that name no target apply to
/// the targets chosen for it, or, for an overloaded spell, to each
/// permanent its filter matches. `X` is the spell's X, or the number the
/// ability carries (damage dealt, a creature's power).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eff {
    Damage(i32),
    /// X damage to the targets.
    DamageX,
    Destroy,
    Exile,
    Bounce,
    Detain,
    Tap,
    Untap,
    /// +p/+t and keywords until end of turn.
    Pump(i32, i32, u32),
    /// The source gets +p/+t and keywords until end of turn.
    PumpSelf(i32, i32),
    PumpSelfKw(u32),
    /// +1/+1 counters.
    Counters(i32),
    /// X +1/+1 counters on the source (Necropolis Regent, Cryptborn Horror).
    CountersX,
    /// Until end of turn, +1/+1 for each creature you control, and trample (Chorus of Might).
    Chorus,
    /// A creature dealt damage this way that would die this turn is exiled instead.
    ExileIfDies,
    Token(Token, u8),
    /// An X/X Ooze.
    OozeX,
    Populate,
    /// You draw.
    Draw(u8),
    DrawX,
    /// The target player draws.
    TargetDraws(u8),
    Gain(i32),
    GainX,
    /// The target player gains.
    TargetGains(i32),
    /// Damage to each opponent.
    DamageEachOpponent(i32),
    /// The target player discards; they choose.
    TargetDiscards(u8),
    TargetDiscardsX,
    /// Each opponent discards at random.
    OpponentDiscardsRandom(u8),
    /// The controller of the targeted permanent discards (Assassin's Strike).
    ItsControllerDiscards(u8),
    /// The controller of the targeted permanent loses life (Launch Party).
    ItsControllerLoses(i32),
    /// The target player loses life.
    TargetLoses(i32),
    Counter,
    /// Counter it unless its controller pays this much generic mana.
    CounterUnlessPays(u8),
    /// Counter it unless its controller pays X; exiled if countered so (Syncopate).
    CounterUnlessPaysX,
    /// Counter a creature spell; its power in damage to its controller (Essence Backlash).
    CounterBacklash,
    /// Draw, then discard.
    Loot(u8, u8),
    /// You may discard a card; if you do, draw (Viashino Racketeer).
    Rummage,
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
    /// Put the targeted permanent on top of its owner's library (Azorius Charm).
    TargetToLibraryTop,
    /// A regeneration shield on the source.
    Regenerate,
    /// A regeneration shield on each creature you control.
    RegenerateYours,
    /// The source keyrune becomes a creature until end of turn.
    Animate,
    /// Attach the source equipment to the target.
    Attach,
    /// Destroy all Auras attached to the target (Street Sweeper).
    DestroyAurasOn,
    /// X +1/+1 counters, X being the power of the creature that died (Death's Presence).
    CountersFromDied,
    /// Destroy each permanent the filter matches (Supreme Verdict).
    DestroyAll(Filter),
    /// Each creature gets +p/+t until end of turn (Golgari Charm).
    PumpAll(i32, i32),
    /// Creatures you control gain these keywords until end of turn.
    KeywordYours(u32),
    /// Each other creature you control gets +X/+X, X the source's power (Wild Beastmaster).
    PumpOthersByPower,
    /// The effect on one of the targets only: 0 the first, 1 the second.
    Nth(u8, Box<Eff>),
    /// The target player mills n.
    Mill(u8),
    /// The target player mills X.
    MillX,
    /// Mill X, X the creatures with defender you control (Doorkeeper).
    MillDefenders,
    /// Shuffle your graveyard into your library; the target mills that many (Psychic Spiral).
    SpiralMill,
    /// The targeted permanent's controller mills until a land (Destroy the Evidence).
    MillUntilLand,
    /// The two targets fight (Guild Feud's creatures).
    Fight,
    /// Gain control of the target until end of turn, untap it; +2/+0 and haste (Traitorous Instinct).
    BorrowUntilEnd,
    /// Exchange control of the source and the target (Conjured Currency).
    Exchange,
    /// Prevent all combat damage to you this turn (Druid's Deliverance).
    PreventCombatToYou,
    /// Prevent the next n damage to you this turn (Security Blockade's land).
    Shield(i32),
    /// Exile the target player's graveyard.
    ExileGraveyard,
    /// Exile the targeted graveyard card.
    ExileCard,
    /// Return the targeted graveyard card to its owner's hand.
    CardToHand,
    /// Each creature deals 1 damage to its controller (Rakdos Charm).
    CreaturesHitControllers,
    /// Gain life equal to the cards in the target player's hand (Search Warrant).
    GainPerCardInHand,
    /// Copy the targeted spell (Nivix Guildmage).
    CopySpell,
    /// Put up to n basic lands, of the colours your deck needs, into your land pile (Seek the Horizon, Gatecreeper Vine).
    PileBasics(u8),
    /// Reveal the top five; take a creature card; the rest to the graveyard (Grisly Salvage).
    Salvage,
    /// Two creature cards from your library: one to hand, one to graveyard (Jarad's Orders).
    TwoCreatures,
    /// Instants of mana value 3, 2 and 1 from your library to hand (Firemind's Foresight).
    Foresight,
    /// Exile the top X; cast instants and sorceries of mana value X or less free; the rest to the graveyard.
    Experiment,
    /// Jace's +1: attackers against you get -1/-0 until your next turn.
    JaceShield,
    /// Jace's -2: the top three, split by the opponent; one pile to hand, one to the bottom.
    JacePiles,
    /// Jace's -8: from each library a nonland card, exiled, then cast free.
    JaceUltimate,
    /// Vraska's +1: a creature that deals her combat damage is destroyed, until your next turn.
    VraskaWard,
    /// Flip a coin: on a win, gain 6 life (Tavern Swindler).
    FlipForLife,
    /// Name a nonland card; exile it from the opponent's hand, graveyard and library (Slaughter Games).
    SlaughterGames,
    /// Exile the top five cards of your library, held (Search the City).
    SearchCity,
    /// Put the creature card of each player's top three onto the battlefield; they fight (Guild Feud).
    GuildFeud,
    /// Each creature able to block the source must (Golgari Decoy): a mark until end of turn.
    MustBeBlocked,
    /// The target attacks this turn if able (Chemister's Trick).
    MustAttack,
    /// Take an extra turn after this one.
    ExtraTurn,
    /// The source's controller loses half their life, rounded up (Havoc Festival).
    LoseHalf,
    /// The source's upkeep check: sacrifice the enchanted permanent unless its controller pays its mana value (Soul Tithe).
    TitheCheck,
    /// Put a filibuster counter on the source; at five, its controller wins (Azor's Elocutors).
    Filibuster,
    /// Return the source to its owner's hand when it has no charge counters (Mana Bloom).
    BloomCheck,
    /// Each opponent may sacrifice a creature; if one does, tap the source and put a +1/+1 counter on it (Desecration Demon).
    DemonOffer,
    /// Create a copy of the source as a token (Pack Rat).
    CopySelf,
    /// Damage equal to the discarded card's mana value (Mercurial Chemister).
    DamageDiscarded,
    /// Each opponent loses life equal to the sacrificed creature's power (Jarad).
    LoseSacrificedPower,
    /// X 1/1 Saprolings, X the sacrificed creature's toughness (Korozda Guildmage).
    SaprolingsByToughness,
    /// Draw four cards (Sphinx of the Chimes, after its discard).
    Draw4,
    /// Fling: damage equal to the source's power to the target (Hellhole Flailer, sacrificed).
    DamageSourcePower,
    /// Return this card from your graveyard to your hand (Jarad).
    FromGraveToHand,
    /// You may pay {B}; if you do, the target player discards (Drainpipe Vermin).
    PayToDiscard,
    /// Remove a charge counter from the source (Mana Bloom).
    RemoveCharge,
    /// The permanent an Aura is attached to gains these keywords until end of turn.
    HostKw(u32),
    /// 1 damage to the target and each other creature with its name (Izzet Staticaster).
    DamageSameName(i32),
    /// Flip a coin; on a loss, 4 damage to each creature and each player (Volatile Rig).
    RigBlast,
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
    /// Card draw or digging: at the end of the opponent's turn.
    Draw,
    /// Removal or a hindrance: when there is a good target.
    Removal,
    /// A planeswalker's loyalty ability.
    Loyalty,
    Never,
}

/// A cost beyond mana, tapping and sacrificing the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extra {
    None,
    /// Sacrifice another creature you control.
    SacAnother,
    /// Sacrifice a nontoken creature you control.
    SacNontoken,
    /// Discard a card.
    Discard,
    /// Discard a creature card.
    DiscardCreature,
    /// Discard two nonland cards with the same name.
    DiscardPair,
    PayLife(i32),
    /// Tap two untapped creatures you control.
    TapTwo,
    /// Sacrifice a Swamp and a Forest (Jarad, from the graveyard).
    SwampAndForest,
    /// Exile an instant or sorcery spell you control (Nivmagus Elemental).
    ExileOwnSpell,
    /// Tap the land the Aura enchants (Security Blockade, Racecourse Fury).
    TapHost,
    /// Tap the enchanted land and pay 1 life (Underworld Connections).
    TapHostPayLife,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activated {
    pub cost: Cost,
    pub tap: bool,
    pub sac: bool,
    pub ability: Ability,
    /// Only as a sorcery (equip, loyalty).
    pub sorcery: bool,
    pub when: Use,
    pub extra: Extra,
    /// A loyalty ability: the change in loyalty it costs.
    pub loyalty: Option<i32>,
    /// Activated from the graveyard (Jarad), not the battlefield.
    pub from_grave: bool,
    /// Only once each turn.
    pub once: bool,
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
    /// +2/+2 and vigilance (Knightly Valor).
    KnightlyValor,
    /// +2/+1, and {R}: trample (Deviant Glee).
    DeviantGlee,
    /// +2/+2, and {U}: flying (Pursuit of Flight).
    PursuitOfFlight,
    /// +1/+1 for each card in its controller's hand; they draw an extra card (Righteous Authority).
    RighteousAuthority,
    /// A land's: "{T}: prevent the next 1 damage to you" (Security Blockade).
    Blockade,
    /// A land's: whenever it becomes tapped, its controller mills three (Chronic Flooding).
    Flooding,
    /// A land's: "{T}, pay 1 life: draw a card" (Underworld Connections).
    Connections,
    /// A land's: "{T}: target creature gains haste" (Racecourse Fury).
    Racecourse,
    /// A land's: it untaps during each other player's untap step (Urban Burgeoning).
    Burgeoning,
    /// A nonland permanent's: sacrificed at upkeep unless its mana value is paid (Soul Tithe).
    SoulTithe,
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
    /// Vigilance while you control a Gate (Armory Guard).
    VigilanceWithGate,
    /// Attacks despite defender while you control a Gate (Ogre Jailbreaker).
    AttacksWithGate,
    /// Damage to you and your other permanents is dealt to this instead (Palisade Giant).
    TakesYourDamage,
    /// Cards and tokens go to exile instead of a graveyard (Rest in Peace).
    NoGraveyards,
    /// Creatures attack you only if their controller pays {X} each, X your enchantments (Sphere of Safety).
    AttackTax,
    /// Your tapped creatures have hexproof during other players' turns (Oak Street Innkeeper).
    TappedHexproof,
    /// Creatures you control get +3/+3 (Collective Blessing).
    Blessing,
    /// Power and toughness: the Rats you control (Pack Rat).
    RatCount,
    /// Power and toughness: the creatures you control (Wayfaring Temple).
    CreatureCount,
    /// +1/+1 for each creature card in your graveyard (Jarad).
    GraveCreatures,
    /// Twice as many +1/+1 counters on your creatures (Corpsejack Menace).
    DoubleCounters,
    /// You may cast sorceries as though they had flash (Hypersonic Dragon).
    SorceryFlash,
    /// Creature spells you cast cost {1} less for each life your opponents lost this turn; cast only after they lost life (Rakdos).
    RakdosLord,
    /// Players can't gain life (Havoc Festival).
    NoLifeGain,
    /// Trample while it has a +1/+1 counter (Chaos Imps).
    TrampleWithCounter,
    /// Attacks each combat if able (Volatile Rig).
    MustAttack,
    /// Each creature able to block this must (Golgari Decoy).
    Lure,
    /// Can't be countered (Loxodon Smiter), and a discard forced by an opponent puts it onto the battlefield.
    Smiter,
    /// Put into a graveyard from anywhere: shuffled into its owner's library (Worldspine Wurm).
    ShuffleBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnCast {
    /// Whenever you cast an instant or sorcery: 2 damage to each opponent (Guttersnipe).
    SpellBurn,
    /// Whenever you cast a multicoloured spell: 2 damage to any target (Pyroconvergence).
    MulticolourBurn,
    /// Whenever you cast a multicoloured spell, untap this (Lobber Crew).
    MulticolourUntap,
    /// Whenever you cast an instant or sorcery: +1/+1 until end of turn and untap (Blistercoil Weird).
    SpellGrow,
    /// Whenever you cast a spell: a life for each of the chosen two colours it is (Tablet of the Guilds).
    Tablet,
}

/// What happens when the source leaves the battlefield to the cards it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    ToHand,
    ToBattlefield,
}

/// How a land comes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    Untapped,
    /// Pay 2 life, or it enters tapped (a shock land).
    Shock,
    /// Tapped; then sacrificed unless {1} is paid (Transguild Promenade).
    Promenade,
}

/// A triggered ability beyond the ones every card can have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum When {
    /// Another creature you control enters (Trostani: gain its toughness).
    AllyEnters,
    /// At the beginning of each combat (Desecration Demon).
    EachCombat,
    /// A creature attacks you (Isperia: you may draw).
    Attacked,
    /// A Dragon you control attacks (Utvara Hellkite).
    DragonAttacks,
    /// A creature you control deals combat damage to a player (Necropolis Regent).
    AllyHits,
    /// This is dealt damage (Volatile Rig: flip or sacrifice).
    Damaged,
    /// A creature you don't control dies (Grave Betrayal).
    EnemyDies,
    /// A source deals damage to you (Azor's Elocutors: remove a counter).
    YouAreDamaged,
    /// At the beginning of each player's upkeep (Havoc Festival).
    EachUpkeep,
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
    /// The Aura enchants a land, not a creature.
    pub enchant_land: bool,
    /// The Aura enchants any nonland permanent.
    pub enchant_permanent: bool,
    pub equip: Option<Cost>,
    pub unleash: bool,
    pub scavenge: Option<Cost>,
    /// An animated keyrune's keywords, colours, power and toughness.
    pub animate: Option<(u32, u8)>,
    pub animate_pt: (i32, i32),
    /// A planeswalker's starting loyalty.
    pub loyalty: Option<i32>,
    /// More triggered abilities, each with what it does.
    pub when: Vec<(When, Ability)>,
    /// A spell's added cost: sacrifice a creature (Launch Party).
    pub sac_to_cast: bool,
    /// Exile this spell as it resolves, not the graveyard (Treasured Find).
    pub exile_after: bool,
    pub entry: Entry,
    /// Enters with X charge counters (Mana Bloom) or X +1/+1 counters (Cryptborn Horror).
    pub x_counters: bool,
    /// As it enters, a card name is chosen (Pithing Needle) or two colours (Tablet of the Guilds).
    pub choose_name: bool,
    pub choose_colours: bool,
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
            enchant_land: false,
            enchant_permanent: false,
            equip: None,
            unleash: false,
            scavenge: None,
            animate: None,
            animate_pt: (2, 2),
            loyalty: None,
            when: Vec::new(),
            sac_to_cast: false,
            exile_after: false,
            entry: Entry::Untapped,
            x_counters: false,
            choose_name: false,
            choose_colours: false,
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
        for (_, a) in &self.when {
            out.extend(a.effects.iter());
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
        extra: Extra::None,
        loyalty: None,
        from_grave: false,
        once: false,
    }
}

/// An activated ability with a cost beyond mana and tapping.
fn act_extra(c: &str, tap: bool, sac: bool, extra: Extra, ability: Ability, when: Use) -> Activated {
    Activated {
        extra,
        ..act(c, tap, sac, ability, when)
    }
}

/// A planeswalker's loyalty ability.
fn loyal(change: i32, ability: Ability) -> Activated {
    Activated {
        sorcery: true,
        loyalty: Some(change),
        once: true,
        ..act("", false, false, ability, Use::Loyalty)
    }
}

/// A creature's "{cost}: Regenerate this creature."
fn regen(c: &str) -> Activated {
    act(
        c,
        false,
        false,
        mode(None, vec![Eff::Regenerate]),
        Use::Regenerate,
    )
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
        "Cancel" => spell(Some(Tgt::Spell(SpellKind::Any)), vec![Eff::Counter]),
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
                extra: Extra::None,
                loyalty: None,
                from_grave: false,
                once: false,
            }],
            ..Rules::base()
        },
        "Goblin Electromancer" => Rules {
            statics: vec![Static::SpellsCheaper],
            ..Rules::base()
        },
        "Izzet Charm" => Rules {
            modes: vec![
                mode(
                    Some(Tgt::Spell(SpellKind::Noncreature)),
                    vec![Eff::CounterUnlessPays(2)],
                ),
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
                extra: Extra::None,
                loyalty: None,
                from_grave: false,
                once: false,
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
        // ---------------- the rest of Return to Ravnica ----------------

        // white
        "Armory Guard" => Rules {
            statics: vec![Static::VigilanceWithGate],
            ..Rules::base()
        },
        "Avenging Arrow" => spell(
            Some(Tgt::One(Filter { damaged: true, ..any })),
            vec![Eff::Destroy],
        ),
        "Bazaar Krovod" => Rules {
            attacks: on(
                Tgt::One(Filter {
                    attacking: true,
                    other: true,
                    ..mine
                }),
                vec![Eff::Pump(0, 2, 0), Eff::Untap],
            ),
            ..Rules::base()
        },
        "Keening Apparition" => Rules {
            activated: vec![act(
                "",
                false,
                true,
                mode(
                    Some(Tgt::One(Filter {
                        enchantment: true,
                        ..permanents(Who::Any)
                    })),
                    vec![Eff::Destroy],
                ),
                Use::Removal,
            )],
            ..Rules::base()
        },
        "Knightly Valor" => Rules {
            aura: Some(Aura::KnightlyValor),
            etb: plain(vec![Eff::Token(Token::Knight, 1)]),
            ..spell(Some(Tgt::One(any)), Vec::new())
        },
        "Martial Law" => Rules {
            upkeep: on(Tgt::One(theirs), vec![Eff::Detain]),
            ..Rules::base()
        },
        "Palisade Giant" => Rules {
            statics: vec![Static::TakesYourDamage],
            ..Rules::base()
        },
        "Rest in Peace" => Rules {
            statics: vec![Static::NoGraveyards],
            etb: plain(vec![Eff::ExileGraveyard]),
            ..Rules::base()
        },
        "Rootborn Defenses" => spell(None, vec![Eff::Populate, Eff::KeywordYours(INDESTRUCTIBLE)]),
        "Security Blockade" => Rules {
            aura: Some(Aura::Blockade),
            activated: vec![act_extra(
                "",
                false,
                false,
                Extra::TapHost,
                mode(None, vec![Eff::Shield(1)]),
                Use::Never,
            )],
            enchant_land: true,
            etb: plain(vec![Eff::Token(Token::Knight, 1)]),
            ..spell(Some(Tgt::One(lands(Who::Mine))), Vec::new())
        },
        "Selesnya Sentry" => Rules {
            activated: vec![regen("{5}{G}")],
            ..Rules::base()
        },
        "Soul Tithe" => Rules {
            aura: Some(Aura::SoulTithe),
            enchant_permanent: true,
            ..spell(Some(Tgt::One(permanents(Who::Theirs))), Vec::new())
        },
        "Sphere of Safety" => Rules {
            statics: vec![Static::AttackTax],
            ..Rules::base()
        },

        // blue
        "Aquus Steed" => Rules {
            activated: vec![act(
                "{2}{U}",
                true,
                false,
                mode(Some(Tgt::One(theirs)), vec![Eff::Pump(-2, 0, 0)]),
                Use::Removal,
            )],
            ..Rules::base()
        },
        "Chronic Flooding" => Rules {
            aura: Some(Aura::Flooding),
            enchant_land: true,
            ..spell(Some(Tgt::One(lands(Who::Theirs))), Vec::new())
        },
        "Conjured Currency" => Rules {
            upkeep: on(Tgt::UpTo(1, permanents(Who::Theirs)), vec![Eff::Exchange]),
            ..Rules::base()
        },
        "Crosstown Courier" => Rules {
            hits: plain(vec![Eff::MillX]),
            ..Rules::base()
        },
        "Dispel" => spell(Some(Tgt::Spell(SpellKind::OnlyInstants)), vec![Eff::Counter]),
        "Doorkeeper" => Rules {
            activated: vec![act(
                "{2}{U}",
                true,
                false,
                mode(Some(Tgt::Player), vec![Eff::MillDefenders]),
                Use::Draw,
            )],
            ..creature(DEFENDER)
        },
        "Jace, Architect of Thought" => Rules {
            loyalty: Some(4),
            activated: vec![
                loyal(1, mode(None, vec![Eff::JaceShield])),
                loyal(-2, mode(None, vec![Eff::JacePiles])),
                loyal(-8, mode(None, vec![Eff::JaceUltimate])),
            ],
            ..Rules::base()
        },
        "Mizzium Skin" => Rules {
            overload: Some(cost("{1}{U}")),
            ..spell(Some(Tgt::One(mine)), vec![Eff::Pump(0, 1, HEXPROOF)])
        },
        "Psychic Spiral" => spell(Some(Tgt::Player), vec![Eff::SpiralMill]),
        "Search the City" => Rules {
            etb: plain(vec![Eff::SearchCity]),
            ..Rules::base()
        },
        "Sphinx of the Chimes" => Rules {
            activated: vec![act_extra(
                "",
                false,
                false,
                Extra::DiscardPair,
                mode(None, vec![Eff::Draw4]),
                Use::Draw,
            )],
            ..creature(FLYING)
        },
        "Syncopate" => spell(Some(Tgt::Spell(SpellKind::Any)), vec![Eff::CounterUnlessPaysX]),
        "Tower Drake" => Rules {
            activated: vec![act(
                "{W}",
                false,
                false,
                mode(None, vec![Eff::PumpSelf(0, 1)]),
                Use::Pump,
            )],
            ..creature(FLYING)
        },

        // black
        "Cremate" => spell(
            Some(Tgt::Grave(GraveKind::Any, Who::Any)),
            vec![Eff::ExileCard, Eff::Draw(1)],
        ),
        "Desecration Demon" => Rules {
            when: vec![(When::EachCombat, mode(None, vec![Eff::DemonOffer]))],
            ..creature(FLYING)
        },
        "Destroy the Evidence" => spell(
            Some(Tgt::One(lands(Who::Any))),
            vec![Eff::MillUntilLand, Eff::Destroy],
        ),
        "Deviant Glee" => Rules {
            aura: Some(Aura::DeviantGlee),
            activated: vec![act(
                "{R}",
                false,
                false,
                mode(None, vec![Eff::HostKw(TRAMPLE)]),
                Use::Pump,
            )],
            ..spell(Some(Tgt::One(any)), Vec::new())
        },
        "Drainpipe Vermin" => Rules {
            dies: on(Tgt::Player, vec![Eff::PayToDiscard]),
            ..Rules::base()
        },
        "Grave Betrayal" => Rules {
            when: vec![(When::EnemyDies, mode(None, Vec::new()))],
            ..Rules::base()
        },
        "Launch Party" => Rules {
            sac_to_cast: true,
            ..spell(
                Some(Tgt::One(any)),
                vec![Eff::ItsControllerLoses(2), Eff::Destroy],
            )
        },
        "Necropolis Regent" => Rules {
            when: vec![(When::AllyHits, mode(None, vec![Eff::CountersX]))],
            ..creature(FLYING)
        },
        "Ogre Jailbreaker" => Rules {
            statics: vec![Static::AttacksWithGate],
            ..creature(DEFENDER)
        },
        "Pack Rat" => Rules {
            statics: vec![Static::RatCount],
            activated: vec![act_extra(
                "{2}{B}",
                false,
                false,
                Extra::Discard,
                mode(None, vec![Eff::CopySelf]),
                Use::SacForToken,
            )],
            ..Rules::base()
        },
        "Tavern Swindler" => Rules {
            activated: vec![act_extra(
                "",
                true,
                false,
                Extra::PayLife(3),
                mode(None, vec![Eff::FlipForLife]),
                Use::Never,
            )],
            ..Rules::base()
        },
        "Terrus Wurm" => Rules {
            scavenge: Some(cost("{6}{B}")),
            ..Rules::base()
        },
        "Underworld Connections" => Rules {
            aura: Some(Aura::Connections),
            activated: vec![act_extra(
                "",
                false,
                false,
                Extra::TapHostPayLife,
                mode(None, vec![Eff::Draw(1)]),
                Use::Draw,
            )],
            enchant_land: true,
            ..spell(Some(Tgt::One(lands(Who::Mine))), Vec::new())
        },

        // red
        "Batterhorn" => Rules {
            etb: on(
                Tgt::UpTo(
                    1,
                    Filter {
                        artifact: true,
                        ..permanents(Who::Any)
                    },
                ),
                vec![Eff::Destroy],
            ),
            ..Rules::base()
        },
        "Chaos Imps" => Rules {
            unleash: true,
            statics: vec![Static::TrampleWithCounter],
            ..creature(FLYING)
        },
        "Dynacharge" => Rules {
            overload: Some(cost("{2}{R}")),
            ..spell(Some(Tgt::One(mine)), vec![Eff::Pump(2, 0, 0)])
        },
        "Guild Feud" => Rules {
            upkeep: plain(vec![Eff::GuildFeud]),
            ..Rules::base()
        },
        "Minotaur Aggressor" => creature(FIRST_STRIKE | HASTE),
        "Pursuit of Flight" => Rules {
            aura: Some(Aura::PursuitOfFlight),
            activated: vec![act(
                "{U}",
                false,
                false,
                mode(None, vec![Eff::HostKw(FLYING)]),
                Use::Pump,
            )],
            ..spell(Some(Tgt::One(any)), Vec::new())
        },
        "Racecourse Fury" => Rules {
            aura: Some(Aura::Racecourse),
            activated: vec![act_extra(
                "",
                false,
                false,
                Extra::TapHost,
                mode(Some(Tgt::One(mine)), vec![Eff::Pump(0, 0, HASTE)]),
                Use::Pump,
            )],
            enchant_land: true,
            ..spell(Some(Tgt::One(lands(Who::Mine))), Vec::new())
        },
        "Street Spasm" => Rules {
            overload: Some(cost("{X}{X}{R}{R}")),
            ..spell(
                Some(Tgt::One(Filter {
                    flying: Some(false),
                    ..theirs
                })),
                vec![Eff::DamageX],
            )
        },
        "Survey the Wreckage" => spell(
            Some(Tgt::One(lands(Who::Any))),
            vec![Eff::Destroy, Eff::Token(Token::Goblin, 1)],
        ),
        "Traitorous Instinct" => spell(Some(Tgt::One(any)), vec![Eff::BorrowUntilEnd]),
        "Utvara Hellkite" => Rules {
            when: vec![(
                When::DragonAttacks,
                plain(vec![Eff::Token(Token::Dragon, 1)]).unwrap(),
            )],
            ..creature(FLYING)
        },
        "Vandalblast" => Rules {
            overload: Some(cost("{4}{R}")),
            ..spell(
                Some(Tgt::One(Filter {
                    artifact: true,
                    ..permanents(Who::Theirs)
                })),
                vec![Eff::Destroy],
            )
        },
        "Viashino Racketeer" => Rules {
            etb: plain(vec![Eff::Rummage]),
            ..Rules::base()
        },

        // green
        "Axebane Guardian" => Rules {
            mana: Some(W | U | B | R | G),
            ..creature(DEFENDER)
        },
        "Druid's Deliverance" => spell(None, vec![Eff::PreventCombatToYou, Eff::Populate]),
        "Gatecreeper Vine" => Rules {
            etb: plain(vec![Eff::PileBasics(1)]),
            ..creature(DEFENDER)
        },
        "Gobbling Ooze" => Rules {
            activated: vec![act_extra(
                "{G}",
                false,
                false,
                Extra::SacAnother,
                mode(None, vec![Eff::Counters(1)]),
                Use::Never,
            )],
            ..Rules::base()
        },
        "Golgari Decoy" => Rules {
            statics: vec![Static::Lure],
            scavenge: Some(cost("{3}{G}{G}")),
            ..Rules::base()
        },
        "Mana Bloom" => Rules {
            x_counters: true,
            mana: Some(W | U | B | R | G),
            upkeep: plain(vec![Eff::BloomCheck]),
            ..Rules::base()
        },
        "Oak Street Innkeeper" => Rules {
            statics: vec![Static::TappedHexproof],
            ..Rules::base()
        },
        "Seek the Horizon" => spell(None, vec![Eff::PileBasics(3)]),
        "Slime Molding" => spell(None, vec![Eff::OozeX]),
        "Stonefare Crocodile" => Rules {
            activated: vec![act(
                "{2}{B}",
                false,
                false,
                mode(None, vec![Eff::PumpSelfKw(LIFELINK)]),
                Use::Pump,
            )],
            ..Rules::base()
        },
        "Urban Burgeoning" => Rules {
            aura: Some(Aura::Burgeoning),
            enchant_land: true,
            ..spell(Some(Tgt::One(lands(Who::Mine))), Vec::new())
        },
        "Wild Beastmaster" => Rules {
            attacks: plain(vec![Eff::PumpOthersByPower]),
            ..Rules::base()
        },
        "Worldspine Wurm" => Rules {
            statics: vec![Static::ShuffleBack],
            dies: plain(vec![Eff::Token(Token::Wurm, 3)]),
            ..creature(TRAMPLE)
        },

        // Azorius
        "Archon of the Triumvirate" => Rules {
            attacks: on(Tgt::UpTo(2, permanents(Who::Theirs)), vec![Eff::Detain]),
            ..creature(FLYING)
        },
        "Azorius Charm" => Rules {
            modes: vec![
                mode(None, vec![Eff::KeywordYours(LIFELINK)]),
                mode(None, vec![Eff::Draw(1)]),
                mode(
                    Some(Tgt::One(Filter { combat: true, ..any })),
                    vec![Eff::TargetToLibraryTop],
                ),
            ],
            ..Rules::base()
        },
        "Dramatic Rescue" => spell(Some(Tgt::One(any)), vec![Eff::Bounce, Eff::Gain(2)]),
        "Fall of the Gavel" => spell(Some(Tgt::Spell(SpellKind::Any)), vec![Eff::Counter, Eff::Gain(5)]),
        "Hussar Patrol" => creature(FLASH | VIGILANCE),
        "Isperia, Supreme Judge" => Rules {
            when: vec![(When::Attacked, plain(vec![Eff::Draw(1)]).unwrap())],
            ..creature(FLYING)
        },
        "New Prahv Guildmage" => Rules {
            activated: vec![
                act(
                    "{W}{U}",
                    false,
                    false,
                    mode(Some(Tgt::One(mine)), vec![Eff::Pump(0, 0, FLYING)]),
                    Use::Pump,
                ),
                act(
                    "{3}{W}{U}",
                    false,
                    false,
                    mode(Some(Tgt::One(permanents(Who::Theirs))), vec![Eff::Detain]),
                    Use::Removal,
                ),
            ],
            ..Rules::base()
        },
        "Righteous Authority" => Rules {
            aura: Some(Aura::RighteousAuthority),
            ..spell(Some(Tgt::One(mine)), Vec::new())
        },
        "Search Warrant" => spell(Some(Tgt::Player), vec![Eff::GainPerCardInHand]),
        "Skymark Roc" => Rules {
            attacks: on(
                Tgt::UpTo(
                    1,
                    Filter {
                        max_toughness: Some(2),
                        ..theirs
                    },
                ),
                vec![Eff::Bounce],
            ),
            ..creature(FLYING)
        },
        "Sphinx's Revelation" => spell(None, vec![Eff::GainX, Eff::DrawX]),
        "Supreme Verdict" => Rules {
            kw: CANT_BE_COUNTERED,
            ..spell(None, vec![Eff::DestroyAll(creatures(Who::Any))])
        },
        "Azor's Elocutors" => Rules {
            upkeep: plain(vec![Eff::Filibuster]),
            when: vec![(When::YouAreDamaged, mode(None, Vec::new()))],
            ..Rules::base()
        },
        "Judge's Familiar" => Rules {
            activated: vec![act(
                "",
                false,
                true,
                mode(
                    Some(Tgt::Spell(SpellKind::InstantOrSorcery)),
                    vec![Eff::CounterUnlessPays(1)],
                ),
                Use::Never,
            )],
            ..creature(FLYING)
        },

        // Izzet
        "Chemister's Trick" => Rules {
            overload: Some(cost("{3}{U}{R}")),
            ..spell(Some(Tgt::One(theirs)), vec![Eff::Pump(-2, 0, MUST_ATTACK)])
        },
        "Counterflux" => Rules {
            kw: CANT_BE_COUNTERED,
            overload: Some(cost("{1}{U}{U}{R}")),
            ..spell(Some(Tgt::Spell(SpellKind::NotYours)), vec![Eff::Counter])
        },
        "Epic Experiment" => spell(None, vec![Eff::Experiment]),
        "Essence Backlash" => spell(Some(Tgt::Spell(SpellKind::Creature)), vec![Eff::CounterBacklash]),
        "Firemind's Foresight" => spell(None, vec![Eff::Foresight]),
        "Hypersonic Dragon" => Rules {
            statics: vec![Static::SorceryFlash],
            ..creature(FLYING | HASTE)
        },
        "Izzet Staticaster" => Rules {
            activated: vec![act(
                "",
                true,
                false,
                mode(Some(Tgt::One(any)), vec![Eff::DamageSameName(1)]),
                Use::Ping,
            )],
            ..creature(FLASH | HASTE)
        },
        "Mercurial Chemister" => Rules {
            activated: vec![
                act("{U}", true, false, mode(None, vec![Eff::Draw(2)]), Use::Draw),
                act_extra(
                    "{R}",
                    true,
                    false,
                    Extra::Discard,
                    mode(Some(Tgt::One(theirs)), vec![Eff::DamageDiscarded]),
                    Use::Removal,
                ),
            ],
            ..Rules::base()
        },
        "Nivix Guildmage" => Rules {
            activated: vec![
                act(
                    "{1}{U}{R}",
                    false,
                    false,
                    mode(None, vec![Eff::Loot(1, 1)]),
                    Use::Draw,
                ),
                act(
                    "{2}{U}{R}",
                    false,
                    false,
                    mode(
                        Some(Tgt::Spell(SpellKind::InstantOrSorcery)),
                        vec![Eff::CopySpell],
                    ),
                    Use::Never,
                ),
            ],
            ..Rules::base()
        },
        "Teleportal" => Rules {
            overload: Some(cost("{3}{U}{R}")),
            ..spell(Some(Tgt::One(mine)), vec![Eff::Pump(1, 0, UNBLOCKABLE)])
        },
        "Thoughtflare" => spell(None, vec![Eff::Loot(4, 2)]),
        "Blistercoil Weird" => Rules {
            on_cast: Some(OnCast::SpellGrow),
            ..Rules::base()
        },
        "Nivmagus Elemental" => Rules {
            activated: vec![act_extra(
                "",
                false,
                false,
                Extra::ExileOwnSpell,
                mode(None, vec![Eff::Counters(2)]),
                Use::Never,
            )],
            ..Rules::base()
        },

        // Rakdos
        "Auger Spree" => spell(Some(Tgt::One(any)), vec![Eff::Pump(4, -4, 0)]),
        "Havoc Festival" => Rules {
            statics: vec![Static::NoLifeGain],
            when: vec![(When::EachUpkeep, plain(vec![Eff::LoseHalf]).unwrap())],
            ..Rules::base()
        },
        "Hellhole Flailer" => Rules {
            unleash: true,
            activated: vec![act(
                "{2}{B}{R}",
                false,
                true,
                mode(Some(Tgt::PlayerOrWalker), vec![Eff::DamageSourcePower]),
                Use::Ping,
            )],
            ..Rules::base()
        },
        "Rakdos Charm" => Rules {
            modes: vec![
                mode(Some(Tgt::Player), vec![Eff::ExileGraveyard]),
                mode(
                    Some(Tgt::One(Filter {
                        artifact: true,
                        ..permanents(Who::Any)
                    })),
                    vec![Eff::Destroy],
                ),
                mode(None, vec![Eff::CreaturesHitControllers]),
            ],
            ..Rules::base()
        },
        "Rakdos Ragemutt" => creature(LIFELINK | HASTE),
        "Rakdos Ringleader" => Rules {
            hits: plain(vec![Eff::OpponentDiscardsRandom(1)]),
            activated: vec![regen("{B}")],
            ..creature(FIRST_STRIKE)
        },
        "Rakdos, Lord of Riots" => Rules {
            statics: vec![Static::RakdosLord],
            ..creature(FLYING | TRAMPLE)
        },
        "Rakdos's Return" => spell(Some(Tgt::Opponent), vec![Eff::DamageX, Eff::TargetDiscardsX]),
        "Rix Maadi Guildmage" => Rules {
            activated: vec![
                act(
                    "{B}{R}",
                    false,
                    false,
                    mode(
                        Some(Tgt::One(Filter {
                            blocking: true,
                            ..any
                        })),
                        vec![Eff::Pump(-1, -1, 0)],
                    ),
                    Use::Removal,
                ),
                act(
                    "{B}{R}",
                    false,
                    false,
                    mode(Some(Tgt::Opponent), vec![Eff::TargetLoses(1)]),
                    Use::Ping,
                ),
            ],
            ..Rules::base()
        },
        "Skull Rend" => spell(
            None,
            vec![Eff::DamageEachOpponent(2), Eff::OpponentDiscardsRandom(2)],
        ),
        "Slaughter Games" => Rules {
            kw: CANT_BE_COUNTERED,
            ..spell(None, vec![Eff::SlaughterGames])
        },
        "Spawn of Rix Maadi" => Rules {
            unleash: true,
            ..Rules::base()
        },
        "Cryptborn Horror" => Rules {
            x_counters: true,
            ..creature(TRAMPLE)
        },

        // Golgari
        "Corpsejack Menace" => Rules {
            statics: vec![Static::DoubleCounters],
            ..Rules::base()
        },
        "Golgari Charm" => Rules {
            modes: vec![
                mode(None, vec![Eff::PumpAll(-1, -1)]),
                mode(
                    Some(Tgt::One(Filter {
                        enchantment: true,
                        ..permanents(Who::Any)
                    })),
                    vec![Eff::Destroy],
                ),
                mode(None, vec![Eff::RegenerateYours]),
            ],
            ..Rules::base()
        },
        "Grisly Salvage" => spell(None, vec![Eff::Salvage]),
        "Jarad, Golgari Lich Lord" => Rules {
            statics: vec![Static::GraveCreatures],
            activated: vec![
                act_extra(
                    "{1}{B}{G}",
                    false,
                    false,
                    Extra::SacAnother,
                    mode(None, vec![Eff::LoseSacrificedPower]),
                    Use::Never,
                ),
                Activated {
                    from_grave: true,
                    extra: Extra::SwampAndForest,
                    ..act(
                        "",
                        false,
                        false,
                        mode(None, vec![Eff::FromGraveToHand]),
                        Use::Never,
                    )
                },
            ],
            ..Rules::base()
        },
        "Jarad's Orders" => spell(None, vec![Eff::TwoCreatures]),
        "Korozda Guildmage" => Rules {
            activated: vec![
                act(
                    "{1}{B}{G}",
                    false,
                    false,
                    mode(Some(Tgt::One(mine)), vec![Eff::Pump(1, 1, INTIMIDATE)]),
                    Use::Pump,
                ),
                act_extra(
                    "{2}{B}{G}",
                    false,
                    false,
                    Extra::SacNontoken,
                    mode(None, vec![Eff::SaprolingsByToughness]),
                    Use::Never,
                ),
            ],
            ..Rules::base()
        },
        "Lotleth Troll" => Rules {
            activated: vec![
                act_extra(
                    "",
                    false,
                    false,
                    Extra::DiscardCreature,
                    mode(None, vec![Eff::Counters(1)]),
                    Use::Never,
                ),
                regen("{B}"),
            ],
            ..creature(TRAMPLE)
        },
        "Rites of Reaping" => spell(
            Some(Tgt::Two(mine, theirs)),
            vec![
                Eff::Nth(0, Box::new(Eff::Pump(3, 3, 0))),
                Eff::Nth(1, Box::new(Eff::Pump(-3, -3, 0))),
            ],
        ),
        "Treasured Find" => Rules {
            exile_after: true,
            ..spell(Some(Tgt::Grave(GraveKind::Any, Who::Mine)), vec![Eff::CardToHand])
        },
        "Trestle Troll" => Rules {
            activated: vec![regen("{1}{B}{G}")],
            ..creature(DEFENDER | REACH)
        },
        "Vraska the Unseen" => Rules {
            loyalty: Some(5),
            activated: vec![
                loyal(1, mode(None, vec![Eff::VraskaWard])),
                loyal(
                    -3,
                    mode(Some(Tgt::One(permanents(Who::Theirs))), vec![Eff::Destroy]),
                ),
                loyal(-7, mode(None, vec![Eff::Token(Token::Assassin, 3)])),
            ],
            ..Rules::base()
        },
        "Deathrite Shaman" => Rules {
            activated: vec![
                act(
                    "",
                    true,
                    false,
                    mode(Some(Tgt::Grave(GraveKind::Land, Who::Any)), vec![Eff::ExileCard]),
                    Use::Never,
                ),
                act(
                    "{B}",
                    true,
                    false,
                    mode(
                        Some(Tgt::Grave(GraveKind::InstantOrSorcery, Who::Any)),
                        vec![Eff::ExileCard, Eff::DamageEachOpponent(2)],
                    ),
                    Use::Ping,
                ),
                act(
                    "{G}",
                    true,
                    false,
                    mode(
                        Some(Tgt::Grave(GraveKind::Creature, Who::Any)),
                        vec![Eff::ExileCard, Eff::Gain(2)],
                    ),
                    Use::Ping,
                ),
            ],
            ..Rules::base()
        },
        "Golgari Longlegs" => Rules::base(),
        "Slitherhead" => Rules {
            scavenge: Some(cost("{0}")),
            ..Rules::base()
        },

        // Selesnya
        "Armada Wurm" => Rules {
            etb: plain(vec![Eff::Token(Token::Wurm, 1)]),
            ..creature(TRAMPLE)
        },
        "Call of the Conclave" => spell(None, vec![Eff::Token(Token::Centaur, 1)]),
        "Collective Blessing" => Rules {
            statics: vec![Static::Blessing],
            ..Rules::base()
        },
        "Common Bond" => spell(Some(Tgt::Twice(any)), vec![Eff::Counters(1)]),
        "Coursers' Accord" => spell(None, vec![Eff::Token(Token::Centaur, 1), Eff::Populate]),
        "Heroes' Reunion" => spell(Some(Tgt::Player), vec![Eff::TargetGains(7)]),
        "Loxodon Smiter" => Rules {
            kw: CANT_BE_COUNTERED,
            statics: vec![Static::Smiter],
            ..Rules::base()
        },
        "Risen Sanctuary" => creature(VIGILANCE),
        "Trostani, Selesnya's Voice" => Rules {
            when: vec![(When::AllyEnters, mode(None, Vec::new()))],
            activated: vec![act(
                "{1}{G}{W}",
                true,
                false,
                mode(None, vec![Eff::Populate]),
                Use::SacForToken,
            )],
            ..Rules::base()
        },
        "Vitu-Ghazi Guildmage" => Rules {
            activated: vec![
                act(
                    "{4}{G}{W}",
                    false,
                    false,
                    mode(None, vec![Eff::Token(Token::Centaur, 1)]),
                    Use::SacForToken,
                ),
                act(
                    "{2}{G}{W}",
                    false,
                    false,
                    mode(None, vec![Eff::Populate]),
                    Use::SacForToken,
                ),
            ],
            ..Rules::base()
        },
        "Wayfaring Temple" => Rules {
            statics: vec![Static::CreatureCount],
            hits: plain(vec![Eff::Populate]),
            ..Rules::base()
        },
        "Sundering Growth" => spell(
            Some(Tgt::One(Filter {
                art_or_ench: true,
                ..permanents(Who::Any)
            })),
            vec![Eff::Destroy, Eff::Populate],
        ),

        // colourless
        "Codex Shredder" => Rules {
            activated: vec![
                act(
                    "",
                    true,
                    false,
                    mode(Some(Tgt::Player), vec![Eff::Mill(1)]),
                    Use::Draw,
                ),
                act(
                    "{5}",
                    true,
                    true,
                    mode(Some(Tgt::Grave(GraveKind::Any, Who::Mine)), vec![Eff::CardToHand]),
                    Use::Never,
                ),
            ],
            ..Rules::base()
        },
        "Izzet Keyrune" => Rules {
            mana: Some(U | R),
            animate: Some((0, U | R)),
            animate_pt: (2, 1),
            hits: plain(vec![Eff::Loot(1, 1)]),
            activated: vec![act(
                "{U}{R}",
                false,
                false,
                mode(None, vec![Eff::Animate]),
                Use::Animate,
            )],
            ..Rules::base()
        },
        "Rakdos Keyrune" => Rules {
            mana: Some(B | R),
            animate: Some((FIRST_STRIKE, B | R)),
            animate_pt: (3, 1),
            activated: vec![act(
                "{B}{R}",
                false,
                false,
                mode(None, vec![Eff::Animate]),
                Use::Animate,
            )],
            ..Rules::base()
        },
        "Selesnya Keyrune" => Rules {
            mana: Some(G | W),
            animate: Some((0, G | W)),
            animate_pt: (3, 3),
            activated: vec![act(
                "{G}{W}",
                false,
                false,
                mode(None, vec![Eff::Animate]),
                Use::Animate,
            )],
            ..Rules::base()
        },
        "Pithing Needle" => Rules {
            choose_name: true,
            ..Rules::base()
        },
        "Tablet of the Guilds" => Rules {
            choose_colours: true,
            on_cast: Some(OnCast::Tablet),
            ..Rules::base()
        },
        "Volatile Rig" => Rules {
            statics: vec![Static::MustAttack],
            when: vec![(When::Damaged, mode(None, Vec::new()))],
            dies: plain(vec![Eff::RigBlast]),
            ..creature(TRAMPLE)
        },

        // lands
        "Blood Crypt" | "Hallowed Fountain" | "Overgrown Tomb" | "Steam Vents" | "Temple Garden" => Rules {
            mana: land_unit(name),
            entry: Entry::Shock,
            ..Rules::base()
        },
        "Grove of the Guardian" => Rules {
            mana: Some(crate::mana::C),
            activated: vec![act_extra(
                "{3}{G}{W}",
                true,
                true,
                Extra::TapTwo,
                mode(None, vec![Eff::Token(Token::Elemental, 1)]),
                Use::SacForToken,
            )],
            ..Rules::base()
        },
        "Rogue's Passage" => Rules {
            mana: Some(crate::mana::C),
            activated: vec![act(
                "{4}",
                true,
                false,
                mode(Some(Tgt::One(mine)), vec![Eff::Pump(0, 0, UNBLOCKABLE)]),
                Use::Pump,
            )],
            ..Rules::base()
        },
        "Transguild Promenade" => Rules {
            mana: Some(W | U | B | R | G),
            entry: Entry::Promenade,
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
        "Blood Crypt" => B | R,
        "Hallowed Fountain" => W | U,
        "Overgrown Tomb" => B | G,
        "Steam Vents" => U | R,
        "Temple Garden" => G | W,
        "Grove of the Guardian" | "Rogue's Passage" => crate::mana::C,
        "Transguild Promenade" => W | U | B | R | G,
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
        assert!(for_card("Grizzly Bears").is_none());
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
