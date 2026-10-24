//! The game: one `Session` the shim holds. It loads the content, keeps the
//! state of play, takes every input through `act(name, value)`, advances
//! animation and battle playback in `frame`, and describes the screen in
//! `ui()` (view.rs) and the 3D view in the draw list (scene.rs). The page
//! decides nothing (Builder R6).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::content::{self, Card, Kind};
use crate::duel::{Answer, Ask, Duel, Ev, Event, Outcome, Pending, Setup, P};
use crate::encounter;
use crate::look::Look;
use crate::player::{self, Armoury, Player};
use crate::quest::{self, Happened, Quest};
use crate::render::{FrameOut, StaticScene};
use crate::rng::Rng;
use crate::story::{self, Effect, Face, Outcome as StoryOutcome, Story};
use crate::world::{City, Feature, Guild, Pos, World};

/// Microseconds a fight's event stays on screen at speed one.
pub const EVENT_US: i64 = 450_000;
/// Microseconds for a step from tile to tile.
pub const STEP_US: i64 = 260_000;
pub const SAVE_VERSION: u32 = 1;
/// Supplies lost when you flee a creature.
pub const FLEE_SUPPLIES: i32 = 2;

pub type Strings = BTreeMap<String, String>;

/// What the player is looking at.
#[derive(Debug, Clone)]
pub enum Screen {
    Title,
    NewGame,
    Board,
    /// A card turned up on a tile.
    Draw {
        at: Pos,
        card: usize,
    },
    Story {
        at: Pos,
        card: usize,
    },
    /// A fate draw: the faces, face up until shuffled, then one picked.
    Fate {
        at: Pos,
        card: usize,
        choice: usize,
        faces: Vec<Face>,
        shuffled: bool,
        picked: Option<usize>,
    },
    /// What happened, and where to go next.
    Result {
        lines: Vec<String>,
        next: Next,
    },
    Land {
        card: usize,
        from: Pos,
    },
    Battle(Box<Fight>),
    City {
        city: City,
        room: Room,
    },
    Deck {
        back: Box<Screen>,
    },
    Lair,
    Victory,
    Models {
        page: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Room {
    Menu,
    Armourer,
    Cards,
    Hall,
    Inn,
    Trainer,
}

impl Room {
    pub fn key(self) -> &'static str {
        match self {
            Room::Menu => "menu",
            Room::Armourer => "armourer",
            Room::Cards => "cards",
            Room::Hall => "hall",
            Room::Inn => "inn",
            Room::Trainer => "trainer",
        }
    }
    fn from_key(s: &str) -> Option<Room> {
        [
            Room::Menu,
            Room::Armourer,
            Room::Cards,
            Room::Hall,
            Room::Inn,
            Room::Trainer,
        ]
        .into_iter()
        .find(|r| r.key() == s)
    }
}

#[derive(Debug, Clone)]
pub enum Next {
    Board,
    Fight {
        at: Pos,
        creature: usize,
        prize: Option<usize>,
    },
    City(City),
    Victory,
}

#[derive(Debug, Clone)]
pub struct Fight {
    pub at: Pos,
    pub creature: usize,
    /// The story card won with the fight, when a story sent you to it.
    pub prize: Option<usize>,
    pub guardian: bool,
    pub duel: Duel,
    /// The event on screen: playback runs behind the game.
    pub shown: usize,
    /// When the event on screen came up: its animation runs from here.
    pub shown_at: i64,
    pub next_at: i64,
    pub speed: i64,
}

impl Fight {
    pub fn caught_up(&self) -> bool {
        self.shown + 1 >= self.duel.events.len()
    }

    pub fn event(&self) -> &Event {
        &self.duel.events[self.shown.min(self.duel.events.len().saturating_sub(1))]
    }

    /// Whether the game waits on the player, with the table shown up to now.
    pub fn asking(&self) -> Option<&Ask> {
        match &self.duel.pending {
            Some(Pending { who: P, ask }) if self.caught_up() && self.duel.outcome.is_none() => Some(ask),
            _ => None,
        }
    }

    pub fn done(&self) -> bool {
        self.duel.outcome.is_some() && self.caught_up()
    }
}

/// What is kept between visits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Save {
    pub version: u32,
    pub seed: u64,
    pub guild: Guild,
    pub chunks: Vec<Pos>,
    pub tiles: Vec<(Pos, crate::world::Tile)>,
    pub player: Player,
}

pub struct Session {
    pub cards: Vec<Card>,
    pub stories: BTreeMap<String, Story>,
    pub quests: Vec<Quest>,
    pub armoury: Armoury,
    pub look: Look,
    pub strings: Strings,
    pub hints: Vec<(String, String)>,
    pub world: Option<World>,
    pub player: Option<Player>,
    pub guild: Guild,
    pub screen: Screen,
    pub rng: Rng,
    pub seed: u64,
    pub now: i64,
    /// How far the fight's table is turned from its home view (turns).
    pub view_yaw: i32,
    /// The step under way: from, to, when it began.
    pub walk: Option<(Pos, Pos, i64)>,
    /// The view's size in CSS pixels, and in device pixels.
    pub size: (i32, i32),
    pub pixels: (i32, i32),
    pub frame_out: FrameOut,
    pub statics: StaticScene,
    pub statics_dirty: bool,
    pub ui_version: u64,
    pub save_out: Option<String>,
    pub saved: Option<Save>,
    pub message: String,
    /// The piece shown on the armourer's stand.
    pub showing: String,
    /// Cards bought or sold this visit to a city's merchant.
    pub stock: BTreeMap<City, Vec<usize>>,
}

fn bundle_get<'a>(b: &'a BTreeMap<String, String>, k: &str) -> Result<&'a str, String> {
    b.get(k)
        .map(|s| s.as_str())
        .ok_or_else(|| format!("The data file \"{k}\" is missing."))
}

impl Session {
    /// `bundle_json` maps each data file's name to its text (data/manifest.json lists them).
    pub fn new(bundle_json: &str, save: &str, seed: u64) -> Result<Session, String> {
        let bundle: BTreeMap<String, String> =
            serde_json::from_str(bundle_json).map_err(|e| format!("The data could not be read: {e}"))?;
        let cards = content::parse(bundle_get(&bundle, "cards.json")?)?;
        let story_files: Vec<&str> = bundle
            .iter()
            .filter(|(k, _)| k.starts_with("stories/"))
            .map(|(_, v)| v.as_str())
            .collect();
        let stories = story::parse(&story_files)?;
        for c in &cards {
            if !stories.contains_key(&c.number) {
                return Err(format!("The card \"{}\" has no story.", c.name));
            }
        }
        let quests = quest::parse(bundle_get(&bundle, "quests.json")?)?;
        let armoury = player::parse_armour(bundle_get(&bundle, "armour.json")?)?;
        let look = Look::parse(
            bundle_get(&bundle, "palette.json")?,
            bundle_get(&bundle, "models.json")?,
        )?;
        let strings: Strings = serde_json::from_str(bundle_get(&bundle, "strings.json")?)
            .map_err(|e| format!("The strings could not be read: {e}"))?;
        let hints = crate::glossary::parse(bundle.get("glossary.md").map(|s| s.as_str()).unwrap_or(""))
            .map_err(|e| e.message())?
            .into_iter()
            .map(|e| {
                (
                    e.term,
                    e.definition.replace("{pile}", &encounter::PILE_MIN.to_string()),
                )
            })
            .collect();
        let saved = serde_json::from_str::<Save>(save)
            .ok()
            .filter(|s| s.version == SAVE_VERSION);
        Ok(Session {
            view_yaw: 0,
            cards,
            stories,
            quests,
            armoury,
            look,
            strings,
            hints,
            world: None,
            player: None,
            guild: Guild::Izzet,
            screen: Screen::Title,
            rng: Rng::new(seed),
            seed,
            now: 0,
            walk: None,
            size: (1280, 720),
            pixels: (1280, 720),
            frame_out: FrameOut::default(),
            statics: StaticScene::default(),
            statics_dirty: true,
            ui_version: 1,
            save_out: None,
            saved,
            message: String::new(),
            showing: String::new(),
            stock: BTreeMap::new(),
        })
    }

    // ---------- words ----------

    pub fn s(&self, key: &str) -> String {
        self.strings.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    /// A string with its {placeholders} filled.
    pub fn f(&self, key: &str, args: &[(&str, String)]) -> String {
        let mut t = self.s(key);
        for (k, v) in args {
            t = t.replace(&format!("{{{k}}}"), v);
        }
        t
    }

    // ---------- the state of play ----------

    pub fn world(&self) -> &World {
        self.world.as_ref().expect("a game is under way")
    }

    pub fn player(&self) -> &Player {
        self.player.as_ref().expect("a game is under way")
    }

    fn pw(&mut self) -> (&mut Player, &mut World) {
        (
            self.player.as_mut().expect("player"),
            self.world.as_mut().expect("world"),
        )
    }

    fn changed(&mut self) {
        self.ui_version += 1;
    }

    pub fn new_game(&mut self, guild: Guild) {
        let seed = self.rng.next_u64();
        self.seed = seed;
        let mut world = World::new(seed, &self.cards);
        let mut p = Player::new(&self.armoury);
        p.deck = encounter::starter_deck(&self.cards, guild);
        p.collection = p.deck.clone();
        if let Some(t) = world.tiles.get_mut(&(0, 0)) {
            t.lands = encounter::starter_lands(&self.cards, guild);
        }
        p.quests.insert("dracogenius".into(), 0);
        self.guild = guild;
        self.world = Some(world);
        self.player = Some(p);
        self.stock.clear();
        let lines = self.happened(Happened::AtCity(City::Tenth));
        let mut all = vec![self.s("story.opening")];
        all.extend(lines);
        self.screen = Screen::Result {
            lines: all,
            next: Next::City(City::Tenth),
        };
        self.statics_dirty = true;
        self.write_save();
    }

    fn continue_game(&mut self) -> Result<(), String> {
        let s = self.saved.clone().ok_or_else(|| self.s("error.no_save"))?;
        let world = World {
            seed: s.seed,
            chunks: s.chunks.into_iter().collect(),
            tiles: s.tiles.into_iter().collect(),
        };
        self.seed = s.seed;
        self.guild = s.guild;
        self.world = Some(world);
        self.player = Some(s.player);
        // a game saved in a city or at the lair opens there
        self.screen = match self.world().tile(self.player().pos).map(|t| t.feature) {
            Some(Feature::City(c)) => Screen::City {
                city: c,
                room: Room::Menu,
            },
            Some(Feature::Lair) => Screen::Lair,
            _ => Screen::Board,
        };
        self.statics_dirty = true;
        Ok(())
    }

    fn write_save(&mut self) {
        let (Some(w), Some(p)) = (&self.world, &self.player) else {
            return;
        };
        let save = Save {
            version: SAVE_VERSION,
            seed: w.seed,
            guild: self.guild,
            chunks: w.chunks.iter().copied().collect(),
            tiles: w.tiles.iter().map(|(k, v)| (*k, v.clone())).collect(),
            player: p.clone(),
        };
        self.save_out = serde_json::to_string(&save).ok();
        self.saved = Some(save);
    }

    pub fn take_save(&mut self) -> String {
        self.save_out.take().unwrap_or_default()
    }

    // ---------- quests ----------

    /// Moves any quest whose next step this meets; returns what to tell the player.
    fn happened(&mut self, h: Happened) -> Vec<String> {
        let mut lines = Vec::new();
        let quests = self.quests.clone();
        for q in &quests {
            let Some(&done) = self.player().quests.get(&q.id) else {
                continue;
            };
            let done = done as usize;
            if done >= q.steps.len() || !q.steps[done].met_by(h) {
                continue;
            }
            // the lair's step is the guardian beaten, not the arrival
            if q.steps[done].at == "lair" && h == Happened::AtLair && !self.player().guardian_beaten {
                continue;
            }
            let p = self.player.as_mut().expect("player");
            p.quests.insert(q.id.clone(), done as u8 + 1);
            let name = self.s(&format!("quest.{}", q.id));
            if done + 1 == q.steps.len() {
                lines.push(self.f("quest.finished", &[("quest", name)]));
                lines.extend(self.reward(q));
            } else {
                let next = self.s(&format!("quest.{}.{}", q.id, done + 1));
                lines.push(self.f("quest.advanced", &[("quest", name), ("next", next)]));
            }
        }
        lines
    }

    fn reward(&mut self, q: &Quest) -> Vec<String> {
        let mut lines = Vec::new();
        let r = q.reward.clone();
        let p = self.player.as_mut().expect("player");
        if r.gold > 0 {
            p.gold += r.gold;
        }
        if !r.armour.is_empty() && !p.owned.contains(&r.armour) {
            p.owned.push(r.armour.clone());
        }
        if let Some((g, n)) = &r.style {
            *p.standing.entry(g.clone()).or_insert(0) += n;
        }
        if r.gold > 0 {
            lines.push(self.f("effect.gold", &[("n", r.gold.to_string())]));
        }
        if !r.armour.is_empty() {
            let name = self.s(&format!("armour.{}", r.armour));
            lines.push(self.f("quest.armour", &[("piece", name)]));
        }
        if let Some((g, n)) = r.style {
            lines.push(self.f("effect.style", &[("guild", g), ("n", n.to_string())]));
        }
        lines
    }

    // ---------- moving ----------

    pub fn step(&mut self, dir: &str) -> Result<(), String> {
        if !matches!(self.screen, Screen::Board) || self.walk.is_some() {
            return Ok(());
        }
        let d = match dir {
            "n" => (0, -1),
            "s" => (0, 1),
            "e" => (1, 0),
            "w" => (-1, 0),
            _ => return Err(self.f("error.no_direction", &[("dir", dir.to_string())])),
        };
        let from = self.player().pos;
        let to = (from.0 + d.0, from.1 + d.1);
        self.go(to)
    }

    /// Steps onto a neighbouring tile.
    pub fn go(&mut self, to: Pos) -> Result<(), String> {
        let from = self.player().pos;
        if crate::world::distance(from, to) != 1 {
            return Err(self.s("error.not_next_to"));
        }
        let Some(tile) = self.world().tile(to).cloned() else {
            return Err(self.s("error.region_edge"));
        };
        if !tile.passable() {
            return Err(self.s(match tile.feature {
                Feature::Canal => "error.canal",
                _ => "error.peak",
            }));
        }
        let armoury = self.armoury.clone();
        let cards = self.cards.clone();
        let (p, w) = self.pw();
        p.step(&armoury, tile.terrain);
        p.pos = to;
        let grew = w.grow_from(to, &cards);
        if let Some(t) = w.tiles.get_mut(&to) {
            t.visited = true;
        }
        if !grew.is_empty() {
            self.message = self.s("board.grew");
        }
        self.walk = Some((from, to, self.now));
        self.statics_dirty = true;
        if self.player().health <= 0 {
            self.defeat(vec![self.s("board.starved")]);
            return Ok(());
        }
        self.arrive(to);
        Ok(())
    }

    fn arrive(&mut self, at: Pos) {
        let tile = self.world().tile(at).cloned().expect("tile");
        let mut lines = Vec::new();
        if let Some(g) = tile.guild {
            lines.extend(self.happened(Happened::InDistrict(g)));
        }
        match tile.feature {
            Feature::City(c) => {
                self.player.as_mut().expect("player").last_city = at;
                lines.extend(self.happened(Happened::AtCity(c)));
                self.write_save();
                if lines.is_empty() {
                    self.screen = Screen::City {
                        city: c,
                        room: Room::Menu,
                    };
                } else {
                    self.screen = Screen::Result {
                        lines,
                        next: Next::City(c),
                    };
                }
            }
            Feature::Lair => {
                self.write_save();
                self.screen = Screen::Lair;
            }
            Feature::Cards if !tile.cleared && !tile.pool.is_empty() => {
                let k = (self.rng.next_u64() % tile.pool.len() as u64) as usize;
                self.screen = Screen::Draw {
                    at,
                    card: tile.pool[k],
                };
                if !lines.is_empty() {
                    self.message = lines.join(" ");
                }
            }
            _ => {
                if !lines.is_empty() {
                    self.screen = Screen::Result {
                        lines,
                        next: Next::Board,
                    };
                }
                self.write_save();
            }
        }
    }

    fn defeat(&mut self, mut lines: Vec<String>) {
        let armoury = self.armoury.clone();
        let p = self.player.as_mut().expect("player");
        let lost = p.gold / player::DEFEAT_GOLD_DIVISOR;
        p.gold -= lost;
        p.health = (p.max_health(&armoury) / 2).max(1);
        p.supplies = p.supplies.max(player::START_SUPPLIES / 2);
        p.pos = p.last_city;
        let city = match self.world().tile(self.player().pos).map(|t| t.feature) {
            Some(Feature::City(c)) => c,
            _ => City::Tenth,
        };
        lines.push(self.f(
            "defeat.carried",
            &[
                ("gold", lost.to_string()),
                ("city", self.s(&format!("city.{}", city.id()))),
            ],
        ));
        self.statics_dirty = true;
        self.screen = Screen::Result {
            lines,
            next: Next::City(city),
        };
        self.write_save();
    }

    // ---------- cards and outcomes ----------

    fn gain_card(&mut self, c: usize) -> String {
        let p = self.player.as_mut().expect("player");
        p.collection.push(c);
        let into_deck = p.deck.len() < player::DECK_MAX;
        if into_deck {
            p.deck.push(c);
        }
        let name = self.cards[c].name.clone();
        self.f(
            if into_deck {
                "effect.card_deck"
            } else {
                "effect.card"
            },
            &[("card", name)],
        )
    }

    fn clear(&mut self, at: Pos) {
        if let Some(t) = self.world.as_mut().and_then(|w| w.tiles.get_mut(&at)) {
            t.cleared = true;
        }
        self.statics_dirty = true;
    }

    /// Applies an outcome's effects. Returns the lines to show and a fight to start.
    fn apply(&mut self, at: Pos, card: usize, o: &StoryOutcome) -> (Vec<String>, Option<Next>) {
        let mut lines = vec![o.text.clone()];
        let mut settled = false;
        let mut next = None;
        for e in &o.effects {
            match e {
                Effect::Take(_) => {
                    lines.push(self.gain_card(card));
                    self.clear(at);
                    settled = true;
                }
                Effect::Fight(_) => {
                    let creature = self.fight_for(at, card);
                    next = Some(Next::Fight {
                        at,
                        creature,
                        prize: Some(card),
                    });
                    settled = true;
                }
                Effect::Leave(_) => settled = true,
                Effect::Gold(n) => {
                    let p = self.player.as_mut().expect("player");
                    let n = (*n).max(-p.gold);
                    p.gold += n;
                    lines.push(self.f(
                        if n >= 0 { "effect.gold" } else { "effect.gold_lost" },
                        &[("n", n.abs().to_string())],
                    ));
                }
                Effect::Supplies(n) => {
                    let p = self.player.as_mut().expect("player");
                    let max = p.max_supplies();
                    let n = (*n).max(-p.supplies).min(max - p.supplies);
                    p.supplies += n;
                    lines.push(self.f(
                        if n >= 0 {
                            "effect.supplies"
                        } else {
                            "effect.supplies_lost"
                        },
                        &[("n", n.abs().to_string())],
                    ));
                }
                Effect::Health(n) => {
                    let armoury = self.armoury.clone();
                    let p = self.player.as_mut().expect("player");
                    let max = p.max_health(&armoury);
                    let n = (*n).min(max - p.health);
                    p.health += n;
                    lines.push(self.f(
                        if n >= 0 {
                            "effect.health"
                        } else {
                            "effect.health_lost"
                        },
                        &[("n", n.abs().to_string())],
                    ));
                }
                Effect::PoolCard(_) => {
                    let pool: Vec<usize> = self
                        .world()
                        .tile(at)
                        .map(|t| {
                            t.pool
                                .iter()
                                .copied()
                                .filter(|&c| c != card && !self.cards[c].is(content::ty::LAND))
                                .collect()
                        })
                        .unwrap_or_default();
                    if !pool.is_empty() {
                        let k = (self.rng.next_u64() % pool.len() as u64) as usize;
                        lines.push(self.gain_card(pool[k]));
                    }
                }
                Effect::Style((g, n)) => {
                    if let Some(gg) = Guild::from_name(g) {
                        self.player.as_mut().expect("player").guild_standing(gg, *n);
                        lines.push(self.f("effect.style", &[("guild", g.clone()), ("n", n.to_string())]));
                    }
                }
                Effect::QuestStart(id) => {
                    let p = self.player.as_mut().expect("player");
                    if !p.quests.contains_key(id) {
                        p.quests.insert(id.clone(), 0);
                        let name = self.s(&format!("quest.{id}"));
                        let first = self.s(&format!("quest.{id}.0"));
                        lines.push(self.f("quest.started", &[("quest", name), ("next", first)]));
                    }
                }
            }
        }
        if !settled {
            // a failed gamble: the moment passes and the tile is spent
            self.clear(at);
        }
        if self.player().health <= 0 {
            return (lines, None);
        }
        (lines, next)
    }

    /// The creature a story's fight is with: the card itself if it is one,
    /// else the strongest creature in the tile's pool, else one of its colours.
    fn fight_for(&mut self, at: Pos, card: usize) -> usize {
        if self.cards[card].kind() == Kind::Creature {
            return card;
        }
        let pool: Vec<usize> = self.world().tile(at).map(|t| t.pool.clone()).unwrap_or_default();
        if let Some(c) = pool
            .iter()
            .copied()
            .filter(|&c| self.cards[c].kind() == Kind::Creature)
            .max_by_key(|&c| (self.cards[c].value, c))
        {
            return c;
        }
        let colours = self
            .world()
            .tile(at)
            .map(|t| t.colours())
            .unwrap_or(crate::mana::ANY);
        let options: Vec<usize> = (0..self.cards.len())
            .filter(|&c| {
                self.cards[c].kind() == Kind::Creature
                    && self.cards[c].identity & !colours == 0
                    && self.cards[c].identity != 0
                    && self.cards[c].value <= 4
            })
            .collect();
        options
            .get((self.rng.next_u64() % options.len().max(1) as u64) as usize)
            .copied()
            .unwrap_or(card)
    }

    fn start_fight(&mut self, at: Pos, creature: usize, prize: Option<usize>, guardian: bool) {
        let mut foe = if guardian {
            encounter::guardian(&self.cards)
        } else {
            let pool = self.world().tile(at).map(|t| t.pool.clone()).unwrap_or_default();
            encounter::foe(&self.cards, creature, &pool, &mut self.rng)
        };
        if !guardian {
            foe.name = self.s("battle.foe_side");
        }
        let me = encounter::player_side(
            &self.cards,
            &self.armoury,
            self.world(),
            self.player(),
            at,
            &self.s("battle.your_side"),
        );
        let seed = self.rng.next_u64();
        self.view_yaw = 0;
        let mut duel = Duel::new(
            std::rc::Rc::new(self.cards.clone()),
            &Setup {
                sides: [me, foe],
                seed,
            },
            [true, false],
        );
        duel.run();
        self.screen = Screen::Battle(Box::new(Fight {
            at,
            creature,
            prize,
            guardian,
            duel,
            shown: 0,
            shown_at: self.now,
            next_at: self.now,
            speed: 1,
        }));
    }

    fn end_fight(&mut self) {
        let Screen::Battle(f) = self.screen.clone() else {
            return;
        };
        let mut lines = Vec::new();
        let story = self.stories.get(&self.cards[f.creature].number).cloned();
        let life = f.duel.s[P].life;
        let armoury = self.armoury.clone();
        let Some(outcome) = f.duel.outcome else {
            return;
        };
        match outcome {
            Outcome::Won => {
                {
                    let p = self.player.as_mut().expect("player");
                    p.health = life.min(p.max_health(&armoury)).max(1);
                    p.fights_won += 1;
                }
                if let Some(s) = &story {
                    lines.push(s.win.clone());
                }
                if f.guardian {
                    self.player.as_mut().expect("player").guardian_beaten = true;
                    lines.push(self.gain_card(f.creature));
                    lines.extend(self.happened(Happened::AtLair));
                    self.screen = Screen::Result {
                        lines,
                        next: Next::Victory,
                    };
                    self.write_save();
                    return;
                }
                let tier = self.cards[f.creature].tier() as i32;
                let gold = tier * encounter::GOLD_PER_TIER;
                self.player.as_mut().expect("player").gold += gold;
                lines.push(self.f("effect.gold", &[("n", gold.to_string())]));
                let won = f.prize.unwrap_or(f.creature);
                lines.push(self.gain_card(won));
                if f.prize.is_some()
                    && f.prize != Some(f.creature)
                    && self.cards[f.creature].kind() == Kind::Creature
                {
                    // the creature a story sent you to fight stays in its pool
                }
                if self.player().wears_effect(&armoury, "heal_after_win") {
                    let p = self.player.as_mut().expect("player");
                    p.health = (p.health + player::HEAL_AFTER_WIN).min(p.max_health(&armoury));
                    lines.push(self.f("effect.health", &[("n", player::HEAL_AFTER_WIN.to_string())]));
                }
                self.clear(f.at);
                if let Some(g) = self.world().tile(f.at).and_then(|t| t.guild) {
                    lines.extend(self.happened(Happened::WonInDistrict(g)));
                }
                self.screen = Screen::Result {
                    lines,
                    next: Next::Board,
                };
            }
            Outcome::Lost => {
                if let Some(s) = &story {
                    lines.push(s.lose.clone());
                }
                self.defeat(lines);
                return;
            }
            Outcome::Withdrew => {
                let p = self.player.as_mut().expect("player");
                p.health = life.max(1);
                lines.push(self.s("battle.withdrew"));
                self.screen = Screen::Result {
                    lines,
                    next: Next::Board,
                };
            }
        }
        self.write_save();
    }

    // ---------- the one door ----------

    /// Every input: a dotted name and a value. Errors carry the message the page shows.
    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.message.clear();
        let r = self.dispatch(name, value);
        self.changed();
        r
    }

    fn dispatch(&mut self, name: &str, value: &str) -> Result<(), String> {
        let num = |v: &str| {
            v.parse::<usize>()
                .map_err(|_| format!("\"{v}\" is not a number."))
        };
        match name {
            "game.new" => self.screen = Screen::NewGame,
            // Enter: the screen's main action
            "first" => {
                let next = match &self.screen {
                    Screen::Draw { card, .. } => match self.cards[*card].kind() {
                        Kind::Creature => "draw.fight",
                        Kind::Land => "draw.claim",
                        _ => "draw.read",
                    },
                    Screen::Result { .. } => "result.next",
                    _ => return Ok(()),
                };
                return self.dispatch(next, "");
            }
            "game.guild" => {
                let g = Guild::from_name(value)
                    .ok_or_else(|| self.f("error.no_guild", &[("guild", value.to_string())]))?;
                self.new_game(g);
            }
            "game.continue" => self.continue_game()?,
            "screen.show" => match value {
                "title" => self.screen = Screen::Title,
                "models" => self.screen = Screen::Models { page: 0 },
                "board" if self.world.is_some() => self.screen = Screen::Board,
                other => return Err(format!("There is no screen \"{other}\".")),
            },
            "models.page" => {
                if let Screen::Models { page } = &mut self.screen {
                    *page = num(value)?;
                }
            }
            "move" => self.step(value)?,
            "tile.click" => {
                let p = parse_pos(value)?;
                match &self.screen {
                    Screen::Land { .. } => return self.dispatch("land.place", value),
                    Screen::Board if crate::world::distance(self.player().pos, p) == 1 => self.go(p)?,
                    _ => {}
                }
            }
            "deck.open" => {
                if self.world.is_some() {
                    let back = Box::new(self.screen.clone());
                    self.screen = Screen::Deck { back };
                }
            }
            "deck.close" => {
                if let Screen::Deck { back } = &self.screen {
                    self.screen = (**back).clone();
                }
            }
            "deck.add" => {
                let c = num(value)?;
                let p = self.player.as_mut().expect("player");
                let owned = p.collection.iter().filter(|&&x| x == c).count();
                let used = p.deck.iter().filter(|&&x| x == c).count();
                if p.deck.len() >= player::DECK_MAX {
                    return Err(self.f("error.deck_full", &[("n", player::DECK_MAX.to_string())]));
                }
                if used >= owned {
                    return Err(self.s("error.no_more_copies"));
                }
                p.deck.push(c);
                self.write_save();
            }
            "deck.remove" => {
                let c = num(value)?;
                let p = self.player.as_mut().expect("player");
                if let Some(i) = p.deck.iter().rposition(|&x| x == c) {
                    p.deck.remove(i);
                }
                self.write_save();
            }
            // a drawn card
            "draw.fight" => {
                if let Screen::Draw { at, card } = self.screen.clone() {
                    self.start_fight(at, card, None, false);
                }
            }
            "draw.flee" => {
                if let Screen::Draw { at, card } = self.screen.clone() {
                    let flee = self
                        .stories
                        .get(&self.cards[card].number)
                        .map(|s| s.flee.clone())
                        .unwrap_or_default();
                    let p = self.player.as_mut().expect("player");
                    p.supplies = (p.supplies - FLEE_SUPPLIES).max(0);
                    let back = self.walk.map(|w| w.0).unwrap_or(p.last_city);
                    p.pos = back;
                    let _ = at;
                    self.statics_dirty = true;
                    let n = FLEE_SUPPLIES.to_string();
                    self.screen = Screen::Result {
                        lines: vec![flee, self.f("effect.supplies_lost", &[("n", n)])],
                        next: Next::Board,
                    };
                }
            }
            "draw.read" => {
                if let Screen::Draw { at, card } = self.screen.clone() {
                    self.screen = Screen::Story { at, card };
                }
            }
            "draw.claim" => {
                if let Screen::Draw { at, card } = self.screen.clone() {
                    self.screen = Screen::Land { card, from: at };
                }
            }
            "land.place" => {
                let Screen::Land { card, from } = self.screen.clone() else {
                    return Ok(());
                };
                let p = parse_pos(value)?;
                let t = self
                    .world()
                    .tile(p)
                    .cloned()
                    .ok_or_else(|| self.s("error.no_tile"))?;
                if !t.visited {
                    return Err(self.s("error.land_unvisited"));
                }
                if t.lands.len() >= t.land_room() {
                    return Err(self.s("error.land_full"));
                }
                if let Some(tt) = self.world.as_mut().and_then(|w| w.tiles.get_mut(&p)) {
                    tt.lands.push(card);
                }
                self.clear(from);
                let name = self.cards[card].name.clone();
                self.screen = Screen::Result {
                    lines: vec![self.f("land.placed", &[("card", name)])],
                    next: Next::Board,
                };
                self.write_save();
            }
            "story.choose" => {
                let Screen::Story { at, card } = self.screen.clone() else {
                    return Ok(());
                };
                let i = num(value)?;
                let st = self
                    .stories
                    .get(&self.cards[card].number)
                    .cloned()
                    .ok_or_else(|| self.s("error.no_story"))?;
                let ch = st
                    .choices
                    .get(i)
                    .cloned()
                    .ok_or_else(|| self.s("error.no_choice"))?;
                if !self.can_choose(&ch) {
                    return Err(self.s("error.choice_locked"));
                }
                {
                    let p = self.player.as_mut().expect("player");
                    match &ch.needs {
                        Some(story::Need::Gold(n)) => p.gold -= n,
                        Some(story::Need::Supplies(n)) => p.supplies -= n,
                        _ => {}
                    }
                }
                if let Some(fate) = &ch.outcome.fate {
                    let extra = self.player().wears_effect(&self.armoury, "fate_success");
                    self.screen = Screen::Fate {
                        at,
                        card,
                        choice: i,
                        faces: fate.cards(extra),
                        shuffled: false,
                        picked: None,
                    };
                } else {
                    let (lines, next) = self.apply(at, card, &ch.outcome);
                    self.after(lines, next);
                }
            }
            "fate.shuffle" => {
                if let Screen::Fate { faces, shuffled, .. } = &mut self.screen {
                    let mut f = faces.clone();
                    story::shuffle(&mut f, &mut self.rng);
                    *faces = f;
                    *shuffled = true;
                }
            }
            "fate.pick" => {
                let Screen::Fate {
                    at,
                    card,
                    choice,
                    faces,
                    shuffled,
                    picked,
                } = self.screen.clone()
                else {
                    return Ok(());
                };
                if !shuffled || picked.is_some() {
                    return Ok(());
                }
                let i = num(value)?;
                let face = *faces.get(i).ok_or_else(|| self.s("error.no_choice"))?;
                if let Screen::Fate { picked, .. } = &mut self.screen {
                    *picked = Some(i);
                }
                let st = self
                    .stories
                    .get(&self.cards[card].number)
                    .cloned()
                    .ok_or_else(|| self.s("error.no_story"))?;
                let fate = st.choices[choice]
                    .outcome
                    .fate
                    .clone()
                    .ok_or_else(|| self.s("error.no_story"))?;
                let o = fate.branch(face).cloned().unwrap_or(StoryOutcome {
                    text: self.s(face.key()),
                    effects: Vec::new(),
                    fate: None,
                });
                let (mut lines, next) = self.apply(at, card, &o);
                lines.insert(0, self.f("fate.drew", &[("face", self.s(face.key()))]));
                self.after(lines, next);
            }
            "result.next" => {
                let Screen::Result { next, .. } = self.screen.clone() else {
                    return Ok(());
                };
                match next {
                    Next::Board => self.screen = Screen::Board,
                    Next::City(c) => {
                        self.screen = Screen::City {
                            city: c,
                            room: Room::Menu,
                        }
                    }
                    Next::Fight { at, creature, prize } => self.start_fight(at, creature, prize, false),
                    Next::Victory => self.screen = Screen::Victory,
                }
                self.statics_dirty = true;
            }
            "battle.speed" => {
                if let Screen::Battle(f) = &mut self.screen {
                    f.speed = if f.speed >= 4 { 1 } else { f.speed * 2 };
                }
            }
            "battle.skip" => {
                if let Screen::Battle(f) = &mut self.screen {
                    f.shown = f.duel.events.len().saturating_sub(1);
                }
            }
            "battle.done" => {
                if let Screen::Battle(f) = &self.screen {
                    if f.done() {
                        self.end_fight();
                    }
                }
            }
            n if n.starts_with("duel.") => return self.duel_act(n, value),
            "lair.climb" => {
                if !matches!(self.screen, Screen::Lair) {
                    return Ok(());
                }
                if self.player().quests.get("dracogenius").copied().unwrap_or(0) < 2 {
                    return Err(self.s("lair.sealed"));
                }
                let niv = self
                    .cards
                    .iter()
                    .position(|c| c.name == "Niv-Mizzet, Dracogenius")
                    .unwrap_or(0);
                let at = self.player().pos;
                self.start_fight(at, niv, None, true);
            }
            "lair.leave" | "victory.continue" => self.screen = Screen::Board,
            // cities
            "city.room" => {
                if let Screen::City { city, .. } = self.screen.clone() {
                    let room =
                        Room::from_key(value).ok_or_else(|| format!("There is no room \"{value}\"."))?;
                    self.screen = Screen::City { city, room };
                    if room == Room::Armourer {
                        self.showing = encounter::shop_armour(&self.armoury, city)
                            .into_iter()
                            .next()
                            .unwrap_or_default();
                    }
                }
            }
            "city.leave" => {
                self.screen = Screen::Board;
                self.write_save();
            }
            "armour.show" => self.showing = value.to_string(),
            "armour.buy" => {
                let piece = self
                    .armoury
                    .pieces
                    .iter()
                    .find(|p| p.id == value)
                    .cloned()
                    .ok_or_else(|| self.s("error.no_piece"))?;
                let armoury = self.armoury.clone();
                let p = self.player.as_mut().expect("player");
                if p.owned.contains(&piece.id) {
                    return Err(self.s("error.owned"));
                }
                if p.style_with(&armoury, &piece.guild) < piece.style_needed {
                    let g = piece.guild.clone();
                    return Err(self.f(
                        "error.style_needed",
                        &[("guild", g), ("n", piece.style_needed.to_string())],
                    ));
                }
                let price = p.price(&armoury, piece.price);
                if p.gold < price {
                    return Err(self.s("error.gold"));
                }
                p.gold -= price;
                p.owned.push(piece.id.clone());
                p.wear(&armoury, &piece.id)?;
                self.showing = piece.id;
                self.write_save();
            }
            "armour.wear" => {
                let armoury = self.armoury.clone();
                self.player.as_mut().expect("player").wear(&armoury, value)?;
                self.showing = value.to_string();
                self.write_save();
            }
            "cards.buy" => {
                let Screen::City { city, .. } = self.screen.clone() else {
                    return Ok(());
                };
                let c = num(value)?;
                let stock = self.stock_of(city);
                if !stock.contains(&c) {
                    return Err(self.s("error.not_for_sale"));
                }
                let armoury = self.armoury.clone();
                let price = self
                    .player()
                    .price(&armoury, encounter::card_price(&self.cards[c]));
                if self.player().gold < price {
                    return Err(self.s("error.gold"));
                }
                self.player.as_mut().expect("player").gold -= price;
                let mut s = stock;
                if let Some(i) = s.iter().position(|&x| x == c) {
                    s.remove(i);
                }
                self.stock.insert(city, s);
                self.message = self.gain_card(c);
                self.write_save();
            }
            "cards.sell" => {
                let c = num(value)?;
                let p = self.player.as_mut().expect("player");
                let Some(i) = p.collection.iter().position(|&x| x == c) else {
                    return Err(self.s("error.not_owned"));
                };
                p.collection.remove(i);
                let owned = p.collection.iter().filter(|&&x| x == c).count();
                while p.deck.iter().filter(|&&x| x == c).count() > owned {
                    if let Some(j) = p.deck.iter().rposition(|&x| x == c) {
                        p.deck.remove(j);
                    }
                }
                let gain = encounter::card_price(&self.cards[c]) / encounter::SELL_DIVISOR;
                self.player.as_mut().expect("player").gold += gain;
                self.write_save();
            }
            "inn.rest" => {
                let armoury = self.armoury.clone();
                let p = self.player.as_mut().expect("player");
                let price = p.price(&armoury, encounter::INN_REST);
                if p.gold < price {
                    return Err(self.s("error.gold"));
                }
                p.gold -= price;
                p.health = p.max_health(&armoury);
                self.write_save();
            }
            "inn.supplies" => {
                let armoury = self.armoury.clone();
                let p = self.player.as_mut().expect("player");
                let price = p.price(&armoury, encounter::INN_SUPPLIES_PRICE);
                if p.gold < price {
                    return Err(self.s("error.gold"));
                }
                if p.supplies >= p.max_supplies() {
                    return Err(self.s("error.supplies_full"));
                }
                p.gold -= price;
                p.supplies = (p.supplies + encounter::INN_SUPPLIES).min(p.max_supplies());
                self.write_save();
            }
            "trainer.learn" => {
                let (id, base, max) = player::ABILITIES
                    .iter()
                    .find(|a| a.0 == value)
                    .copied()
                    .ok_or_else(|| self.s("error.no_ability"))?;
                let armoury = self.armoury.clone();
                let p = self.player.as_mut().expect("player");
                let have = p.abilities.get(id).copied().unwrap_or(0);
                if have >= max {
                    return Err(self.s("error.learned"));
                }
                let price = p.price(&armoury, base * (have as i32 + 1));
                if p.gold < price {
                    return Err(self.s("error.gold"));
                }
                p.gold -= price;
                p.abilities.insert(id.to_string(), have + 1);
                self.write_save();
            }
            "view.turn" => {
                let step = crate::fx::TURN / 16;
                self.view_yaw = match value {
                    "left" => self.view_yaw - step,
                    "right" => self.view_yaw + step,
                    _ => 0,
                } & (crate::fx::TURN - 1);
            }
            "view.drag" => {
                // a drag across the whole view turns the table half round
                let dx = value
                    .parse::<i32>()
                    .map_err(|_| format!("\"{value}\" is not a number."))?;
                let w = self.size.0.max(1);
                self.view_yaw = (self.view_yaw + dx * (crate::fx::TURN / 2) / w) & (crate::fx::TURN - 1);
            }
            "resize" => {
                // "css width,css height,device pixel ratio": the ratio may be a decimal ("1.5")
                let parts: Vec<&str> = value.split(',').map(|x| x.trim()).collect();
                if let [w, h, r] = parts.as_slice() {
                    let w: i32 = w.split('.').next().unwrap_or("0").parse().unwrap_or(1);
                    let h: i32 = h.split('.').next().unwrap_or("0").parse().unwrap_or(1);
                    let milli = per_mille(r);
                    self.size = (w.max(1), h.max(1));
                    self.pixels = ((w * milli / 1000).max(1), (h * milli / 1000).max(1));
                }
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    fn after(&mut self, lines: Vec<String>, next: Option<Next>) {
        if self.player().health <= 0 {
            self.defeat(lines);
            return;
        }
        self.screen = Screen::Result {
            lines,
            next: next.unwrap_or(Next::Board),
        };
        self.write_save();
    }

    pub fn can_choose(&self, ch: &story::Choice) -> bool {
        let p = self.player();
        match &ch.needs {
            None => true,
            Some(story::Need::Charisma(n)) => p.charisma(&self.armoury) >= *n,
            Some(story::Need::Style((g, n))) => p.style_with(&self.armoury, g) >= *n,
            Some(story::Need::Gold(n)) => p.gold >= *n,
            Some(story::Need::Supplies(n)) => p.supplies >= *n,
        }
    }

    pub fn stock_of(&mut self, city: City) -> Vec<usize> {
        if !self.stock.contains_key(&city) {
            let s = encounter::shop_cards(&self.cards, city, self.seed);
            self.stock.insert(city, s);
        }
        self.stock[&city].clone()
    }

    /// A click on the 3D view at pixel (x, y): the tile or door under it.
    pub fn pick(&mut self, x: i32, y: i32) -> String {
        let m = crate::scene::camera_for(self)
            .matrix((self.size.0 as i64 * crate::fx::ONE as i64 / self.size.1.max(1) as i64) as i32);
        let to_px = |q: (i32, i32), s: &Session| (q.0 * s.size.0 / 1000, q.1 * s.size.1 / 1000);
        let near = |pt: crate::geom::V3, s: &Session| -> Option<i64> {
            let q = crate::geom::project(&m, pt)?;
            let (px, py) = to_px(q, s);
            Some(((px - x) as i64).pow(2) + ((py - y) as i64).pow(2))
        };
        let reach = (self.size.1 / 14).max(1) as i64;
        match self.screen {
            Screen::City { room: Room::Menu, .. } => {
                let best = crate::scene::city_doors(self)
                    .into_iter()
                    .filter_map(|(pt, k)| {
                        near(pt.sub(crate::geom::v3(0, 2 * crate::fx::ONE, 0)), self).map(|d| (d, k))
                    })
                    .min();
                match best {
                    Some((d, k)) if d <= (reach * 3).pow(2) => self.act_json("city.room", k),
                    _ => serde_json::json!({"ok": true}).to_string(),
                }
            }
            Screen::Board | Screen::Land { .. } if self.world.is_some() => {
                let best = self
                    .world()
                    .tiles
                    .keys()
                    .filter_map(|&p| near(crate::scene::tile_centre(p), self).map(|d| (d, p)))
                    .min();
                match best {
                    Some((d, p)) if d <= (reach * 2).pow(2) => {
                        self.act_json("tile.click", &format!("{},{}", p.0, p.1))
                    }
                    _ => serde_json::json!({"ok": true}).to_string(),
                }
            }
            Screen::Battle(ref f) => {
                let snap = f.event().snap.clone();
                let best = crate::scene::battle_spots(self, &snap)
                    .into_iter()
                    .filter_map(|(pt, p)| {
                        near(pt.add(crate::geom::v3(0, crate::fx::ONE, 0)), self).map(|d| (d, p.id))
                    })
                    .min();
                match best {
                    Some((d, id)) if d <= (reach * 2).pow(2) => self.act_json("duel.click", &id.to_string()),
                    _ => serde_json::json!({"ok": true}).to_string(),
                }
            }
            _ => serde_json::json!({"ok": true}).to_string(),
        }
    }

    /// A move in a game of Magic. While the table is still being played back,
    /// any move first shows it up to now.
    fn duel_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        let now = self.now;
        let Screen::Battle(f) = &mut self.screen else {
            return Err("There is no fight on.".to_string());
        };
        if !f.caught_up() {
            f.shown = f.duel.events.len().saturating_sub(1);
            return Ok(());
        }
        let num = |v: &str| {
            v.parse::<usize>()
                .map_err(|_| format!("\"{v}\" is not a number."))
        };
        let before = f.duel.events.len();
        let d = &mut f.duel;
        let r = match name {
            "duel.do" => {
                let acts = d.actions(P);
                let a = *acts.get(num(value)?).ok_or("That can't be done now.")?;
                d.answer(P, Answer::Act(a))
            }
            "duel.ok" => d.confirm(P),
            "duel.cancel" => d.cancel(P),
            "duel.all" => d.toggle_all(P),
            "duel.yes" => d.answer(P, Answer::Yes(true)),
            "duel.no" => d.answer(P, Answer::Yes(false)),
            "duel.none" => d.answer(P, Answer::Pick(None)),
            "duel.pick" => {
                let i = num(value)?;
                match &d.pending {
                    Some(Pending {
                        ask: Ask::Targets { .. } | Ask::Attackers { .. },
                        ..
                    }) => {
                        if d.toggle(P, i)? {
                            d.confirm(P)
                        } else {
                            Ok(())
                        }
                    }
                    _ => d.answer(P, Answer::Pick(Some(i))),
                }
            }
            "duel.block" => {
                let id = value
                    .parse::<u32>()
                    .map_err(|_| format!("\"{value}\" is not a number."))?;
                d.cycle_block(P, id)
            }
            "duel.click" => {
                let id = value
                    .parse::<u32>()
                    .map_err(|_| format!("\"{value}\" is not a number."))?;
                if d.click(P, id)? {
                    d.confirm(P)
                } else {
                    Ok(())
                }
            }
            _ => Err(format!("There is no move \"{name}\".")),
        };
        // the player's own move shows at once; what follows plays back
        if f.duel.events.len() > before {
            f.shown = before.saturating_sub(1).max(f.shown);
            f.shown_at = now;
            f.next_at = now;
        }
        r
    }

    pub fn act_json(&mut self, name: &str, value: &str) -> String {
        match self.act(name, value) {
            Ok(()) => serde_json::json!({"ok": true}).to_string(),
            Err(e) => {
                self.message = e.clone();
                serde_json::json!({"error": e}).to_string()
            }
        }
    }

    // ---------- time ----------

    pub fn frame(&mut self, now_us: i64) {
        self.now = now_us;
        if let Some((_, _, t0)) = self.walk {
            if now_us - t0 >= STEP_US {
                self.walk = None;
                self.changed();
            }
        }
        if let Screen::Battle(f) = &mut self.screen {
            if !f.caught_up() && now_us >= f.next_at {
                f.shown += 1;
                f.shown_at = now_us;
                let ev = f.duel.events[f.shown].ev;
                // the computer's moves at reading pace; the player's own at a glance
                let slow = matches!(ev, Ev::Turn | Ev::Attack | Ev::Cast | Ev::End)
                    && f.duel.events[f.shown].side != P as u8;
                let quick = f.duel.events[f.shown].side == P as u8 && !matches!(ev, Ev::Turn);
                let k = if slow { 2 } else { 1 };
                let k_div = if quick { 2 } else { 1 };
                f.next_at = now_us + EVENT_US * k / k_div / f.speed;
                self.ui_version += 1;
            }
        }
        crate::scene::build(self);
    }

    pub fn ui_json(&mut self) -> String {
        crate::view::ui(self).to_string()
    }

    pub fn ui_version(&self) -> u32 {
        (self.ui_version % (u32::MAX as u64)) as u32
    }

    pub fn value_of(v: &Value) -> String {
        v.to_string()
    }
}

/// The seed: `?seed=123` in the address picks one (a world to share or to
/// screenshot again); otherwise the page's random one.
pub fn seed_from(query: &str, random: u64) -> u64 {
    query
        .trim_start_matches('?')
        .split('&')
        .find_map(|kv| kv.strip_prefix("seed="))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(random)
}

/// "1.5" as 1500: a decimal read without floats.
pub fn per_mille(v: &str) -> i32 {
    let (whole, frac) = v.split_once('.').unwrap_or((v, ""));
    let w: i32 = whole.parse().unwrap_or(1);
    let digits: String = frac.chars().take(3).collect();
    let pad = format!("{digits:0<3}");
    (w * 1000 + pad.parse::<i32>().unwrap_or(0)).clamp(500, 4000)
}

pub fn parse_pos(v: &str) -> Result<Pos, String> {
    let parts: Vec<i32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    match parts.as_slice() {
        [x, y] => Ok((*x, *y)),
        _ => Err(format!("\"{v}\" is not a tile.")),
    }
}

#[cfg(test)]
pub fn for_tests() -> Session {
    let mut b: BTreeMap<String, String> = BTreeMap::new();
    b.insert(
        "cards.json".into(),
        include_str!("../../../data/cards.json").into(),
    );
    b.insert(
        "quests.json".into(),
        include_str!("../../../data/quests.json").into(),
    );
    b.insert(
        "armour.json".into(),
        include_str!("../../../data/armour.json").into(),
    );
    b.insert(
        "palette.json".into(),
        include_str!("../../../data/palette.json").into(),
    );
    b.insert(
        "models.json".into(),
        include_str!("../../../data/models.json").into(),
    );
    b.insert(
        "strings.json".into(),
        include_str!("../../../data/strings.json").into(),
    );
    b.insert("glossary.md".into(), include_str!("../../../GLOSSARY.md").into());
    for (k, v) in [
        ("W", include_str!("../../../data/stories/W.json")),
        ("U", include_str!("../../../data/stories/U.json")),
        ("B", include_str!("../../../data/stories/B.json")),
        ("R", include_str!("../../../data/stories/R.json")),
        ("G", include_str!("../../../data/stories/G.json")),
        ("Azorius", include_str!("../../../data/stories/Azorius.json")),
        ("Izzet", include_str!("../../../data/stories/Izzet.json")),
        ("Rakdos", include_str!("../../../data/stories/Rakdos.json")),
        ("Golgari", include_str!("../../../data/stories/Golgari.json")),
        ("Selesnya", include_str!("../../../data/stories/Selesnya.json")),
        (
            "colourless",
            include_str!("../../../data/stories/colourless.json"),
        ),
    ] {
        b.insert(format!("stories/{k}.json"), v.into());
    }
    Session::new(&serde_json::to_string(&b).unwrap(), "", 42).expect("session")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started() -> Session {
        let mut s = for_tests();
        s.act("game.new", "").unwrap();
        s.act("game.guild", "Izzet").unwrap();
        s
    }

    #[test]
    fn a_seed_in_the_address_wins_over_the_random_one() {
        assert_eq!(seed_from("?seed=77", 5), 77);
        assert_eq!(seed_from("?x=1&seed=8", 5), 8);
        assert_eq!(seed_from("", 5), 5);
        assert_eq!(seed_from("?seed=abc", 5), 5);
    }

    #[test]
    fn a_resize_reads_a_decimal_pixel_ratio() {
        let mut s = for_tests();
        s.act("resize", "1200,800,1.5").unwrap();
        assert_eq!(s.size, (1200, 800));
        assert_eq!(s.pixels, (1800, 1200));
        assert_eq!(per_mille("2"), 2000);
        assert_eq!(per_mille("1.25"), 1250);
    }

    #[test]
    fn a_new_game_starts_in_the_tenth_district_with_lands_and_a_deck() {
        let s = started();
        let p = s.player();
        assert_eq!(p.pos, (0, 0));
        assert_eq!(p.deck.len(), encounter::STARTER_DECK);
        assert_eq!(s.world().tile((0, 0)).unwrap().lands.len(), 3);
        // the main quest's first step is the Tenth District, done at the start
        assert_eq!(p.quests["dracogenius"], 1);
        assert!(matches!(s.screen, Screen::Result { .. }));
    }

    #[test]
    fn a_step_spends_a_supply_and_an_edge_step_grows_the_map() {
        let mut s = started();
        s.act("result.next", "").unwrap();
        s.act("city.leave", "").unwrap();
        // walk east to the edge, skipping whatever each tile draws
        let mut moved = 0;
        for _ in 0..2 {
            s.walk = None;
            let before = s.player().supplies;
            let to = (s.player().pos.0 + 1, 0);
            if !s.world().tile(to).map(|t| t.passable()).unwrap_or(false) {
                break;
            }
            s.act("move", "e").unwrap();
            assert_eq!(s.player().supplies, before - 1);
            s.screen = Screen::Board;
            moved += 1;
        }
        if moved == 2 {
            assert!(s.world().chunks.contains(&(1, 0)));
        }
    }

    #[test]
    fn a_land_goes_only_on_a_visited_tile_with_room() {
        let mut s = started();
        let plains = s.cards.iter().position(|c| c.name == "Plains").unwrap();
        s.screen = Screen::Land {
            card: plains,
            from: (1, 0),
        };
        // (2, 2) is unvisited
        assert_eq!(
            s.act("land.place", "2,2").unwrap_err(),
            s.s("error.land_unvisited")
        );
        // the city holds three, and has three already
        assert_eq!(s.act("land.place", "0,0").unwrap_err(), s.s("error.land_full"));
        s.world.as_mut().unwrap().tiles.get_mut(&(1, 0)).unwrap().visited = true;
        s.act("land.place", "1,0").unwrap();
        assert_eq!(s.world().tile((1, 0)).unwrap().lands, vec![plains]);
    }

    #[test]
    fn a_save_round_trips_through_continue() {
        let mut s = started();
        let save = s.take_save();
        assert!(!save.is_empty());
        let mut b = for_tests();
        b.saved = serde_json::from_str(&save).ok();
        b.act("game.continue", "").unwrap();
        assert_eq!(b.player().deck, s.player().deck);
        assert_eq!(b.world().tiles, s.world().tiles);
        // an unreadable save is not taken up: the title offers no Continue
        assert!(serde_json::from_str::<Save>("{not json").is_err());
    }

    #[test]
    fn the_lair_is_sealed_until_new_prahv_has_been_visited() {
        let mut s = started();
        s.screen = Screen::Lair;
        assert_eq!(s.act("lair.climb", "").unwrap_err(), s.s("lair.sealed"));
        s.player.as_mut().unwrap().quests.insert("dracogenius".into(), 2);
        s.act("lair.climb", "").unwrap();
        assert!(matches!(&s.screen, Screen::Battle(f) if f.guardian));
    }

    #[test]
    fn a_fight_plays_back_and_its_end_settles_the_tile() {
        let mut s = started();
        let caracal = s.cards.iter().position(|c| c.name == "Trained Caracal").unwrap();
        s.world.as_mut().unwrap().tiles.get_mut(&(1, 0)).unwrap().cleared = false;
        s.screen = Screen::Draw {
            at: (1, 0),
            card: caracal,
        };
        s.act("draw.fight", "").unwrap();
        // the game waits on the player's first move
        let Screen::Battle(f) = &mut s.screen else {
            panic!("no fight")
        };
        assert!(f.duel.pending.as_ref().map(|p| p.who == P).unwrap_or(false));
        assert!(s.act("battle.done", "").is_ok());
        assert!(
            matches!(s.screen, Screen::Battle(_)),
            "a fight in play does not end on done"
        );
        // the computer plays the player's side to the end
        if let Screen::Battle(f) = &mut s.screen {
            f.duel.human = [false, false];
            f.duel.run();
        }
        s.act("battle.skip", "").unwrap();
        s.act("battle.done", "").unwrap();
        match &s.screen {
            Screen::Result { .. } => {}
            other => panic!("{:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn buying_armour_needs_gold_and_standing() {
        let mut s = started();
        s.screen = Screen::City {
            city: City::Prahv,
            room: Room::Armourer,
        };
        s.player.as_mut().unwrap().gold = 100;
        // the Azorius circlet needs standing 2 with the Azorius
        assert!(s
            .act("armour.buy", "azorius_head")
            .unwrap_err()
            .contains("Azorius"));
        s.act("armour.buy", "azorius_chest").unwrap();
        let p = s.player();
        assert_eq!(p.worn["chest"], "azorius_chest");
        assert_eq!(p.gold, 60);
    }
}
