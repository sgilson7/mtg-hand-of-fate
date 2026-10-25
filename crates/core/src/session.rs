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
use crate::riot::{self, Ending, Place, Riot};
use crate::rng::Rng;
use crate::story::{self, Effect, Face, Outcome as StoryOutcome, Story};
use crate::world::{City, Feature, Guild, MarkKind, Pos, World};

/// The two missions a new game can be (T32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mission {
    /// Niv-Mizzet in Nivix, with the writ from New Prahv.
    Dracogenius,
    /// Stop Rakdos before he razes New Prahv.
    Riot,
}

impl Mission {
    pub const ALL: [Mission; 2] = [Mission::Dracogenius, Mission::Riot];
    pub fn id(self) -> &'static str {
        match self {
            Mission::Dracogenius => "dracogenius",
            Mission::Riot => "riot",
        }
    }
    pub fn from_id(s: &str) -> Option<Mission> {
        Mission::ALL.into_iter().find(|m| m.id() == s)
    }
    /// What a quest's `starts` says to be begun with this mission's new game.
    fn starts(self) -> &'static str {
        match self {
            Mission::Dracogenius => "new_game",
            Mission::Riot => "mission:riot",
        }
    }
}

/// The key `quest_marks` gives Rakdos's own mark: no quest has it.
pub const RAKDOS_MARK: &str = "rakdos";
/// The key `quest_marks` gives a contract's tile (T37).
pub const CONTRACT_MARK: &str = "contract";
/// A contract's tile is this many steps away, nearest and farthest.
pub const CONTRACT_NEAR: (i32, i32) = (3, 6);
/// Steps a contract runs, and what it pays: gold and Golgari standing.
pub const CONTRACT_STEPS: u32 = 15;
pub const CONTRACT_GOLD: i32 = 15;
pub const CONTRACT_STANDING: i32 = 1;
/// A lapsed contract: health lost (never the last of it) and loyalty lost.
pub const CONTRACT_HARM: i32 = 3;
pub const CONTRACT_LOYALTY: i32 = 2;
/// How far Vraska's gaze reaches from you.
pub const PETRIFY_REACH: i32 = 2;
/// The cards Jace splits into two piles.
pub const PILE_CARDS: usize = 3;
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
    /// Which mission a new game is (T32).
    Mission,
    NewGame,
    /// The guild's starting decks, one of them shown (T25).
    Starter {
        guild: Guild,
        pick: usize,
    },
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
    /// Face to face with Rakdos on the map (T32).
    Rakdos,
    /// Jace's two piles (T37): the first face up, the second face down.
    Piles {
        card: usize,
        piles: [Vec<usize>; 2],
    },
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
    Piles {
        card: usize,
        piles: [Vec<usize>; 2],
    },
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
    #[serde(default)]
    pub riot: Option<Riot>,
}

/// The orders the deck screen offers, the default first.
pub const DECK_SORTS: [&str; 5] = ["cost", "colour", "type", "rarity", "name"];

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
    /// The mission the new game being set up will be.
    pub mission: Mission,
    pub screen: Screen,
    pub rng: Rng,
    pub seed: u64,
    pub now: i64,
    /// How far the fight's table is turned from its home view (turns).
    pub view_yaw: i32,
    /// The fight's whole log is open over the table (T28).
    pub log_open: bool,
    /// How the deck screen orders the collection: one of `DECK_SORTS`.
    pub deck_sort: &'static str,
    /// The permanent under the pointer in a fight, shown full size.
    pub hovered: Option<u32>,
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
                    e.definition
                        .replace("{pile}", &encounter::PILE_MIN.to_string())
                        .replace("{steps}", &CONTRACT_STEPS.to_string()),
                )
            })
            .collect();
        let saved = serde_json::from_str::<Save>(save)
            .ok()
            .filter(|s| s.version == SAVE_VERSION);
        Ok(Session {
            view_yaw: 0,
            log_open: false,
            deck_sort: DECK_SORTS[0],
            hovered: None,
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
            mission: Mission::Dracogenius,
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

    pub fn new_game(&mut self, guild: Guild, starter: &str) {
        let seed = self.rng.next_u64();
        self.seed = seed;
        let mut world = World::new(seed, &self.cards);
        let mut p = Player::new(&self.armoury);
        p.deck = encounter::starter_named(&self.cards, guild, starter);
        p.collection = p.deck.clone();
        if let Some(t) = world.tiles.get_mut(&(0, 0)) {
            t.lands = encounter::starter_lands(&self.cards, guild);
        }
        for q in self.quests.iter().filter(|q| q.starts == self.mission.starts()) {
            p.quests.insert(q.id.clone(), 0);
        }
        if self.mission == Mission::Riot {
            world.riot = Some(Riot::new(seed));
        }
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
        let mut world = World {
            seed: s.seed,
            chunks: s.chunks.into_iter().collect(),
            tiles: s.tiles.into_iter().collect(),
            riot: s.riot,
        };
        world.settle_cities();
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
            riot: w.riot.clone(),
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
        if let Some(riot) = self.world.as_mut().and_then(|w| w.riot.as_mut()) {
            match r.rakdos.as_str() {
                "slow" => {
                    riot.slowed = true;
                    lines.push(self.f("riot.slowed", &[("n", riot::EVERY_SLOWED.to_string())]));
                }
                "weaken" => {
                    riot.weakened = true;
                    lines.push(self.f("riot.weakened", &[("n", encounter::RAKDOS_WEAKENED.to_string())]));
                }
                _ => {}
            }
        }
        lines
    }

    /// Where each quest under way goes next, and Rakdos while he roams
    /// (T33): the map shows each, greyed, before its tile is found.
    pub fn quest_marks(&self) -> Vec<(Pos, String)> {
        let (w, p) = (self.world(), self.player());
        let mut out = Vec::new();
        if let Some(r) = w.riot.as_ref().filter(|r| r.roaming()) {
            // a key no quest has: Rakdos himself, not the Riot quest's next place
            out.push((r.pos(), RAKDOS_MARK.to_string()));
        }
        if let Some(c) = &p.contract {
            out.push((c.at, CONTRACT_MARK.to_string()));
        }
        for q in &self.quests {
            let Some(step) = p.quests.get(&q.id).and_then(|&d| q.steps.get(d as usize)) else {
                continue;
            };
            let at = match step.at.as_str() {
                "city" => City::from_id(&step.city).and_then(|c| crate::world::spot_of(Feature::City(c))),
                "lair" => crate::world::spot_of(Feature::Lair),
                "place" => Place::from_id(&step.place).map(|x| x.spot()),
                "district" | "win_in_district" => Guild::from_name(&step.guild)
                    .and_then(|g| self.nearest_district(g, step.at == "win_in_district")),
                _ => None,
            };
            if let Some(at) = at {
                out.push((at, q.id.clone()));
            }
        }
        out
    }

    /// The district of a guild nearest the player, found or not; for a fight,
    /// one that still has a fight in it.
    fn nearest_district(&self, g: Guild, fight: bool) -> Option<Pos> {
        let w = self.world();
        let at = self.player().pos;
        let r = crate::world::REGION;
        let mut tiles: Vec<(Pos, crate::world::Tile)> = Vec::new();
        for c in (-r..=r).flat_map(|x| (-r..=r).map(move |y| (x, y))) {
            if w.chunks.contains(&c) {
                tiles.extend(
                    w.tiles
                        .iter()
                        .filter(|(q, _)| crate::world::chunk_of(**q) == c)
                        .map(|(q, t)| (*q, t.clone())),
                );
            } else {
                tiles.extend(w.made(c, &self.cards));
            }
        }
        tiles
            .into_iter()
            .filter(|(_, t)| t.guild == Some(g))
            .filter(|(_, t)| !fight || (!t.cleared && (!t.pool.is_empty() || t.minion.is_some())))
            .min_by_key(|(q, _)| (crate::world::distance(*q, at), q.0, q.1))
            .map(|(q, _)| q)
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
        p.steps += 1;
        let grew = w.grow_from(to, &cards);
        if let Some(t) = w.tiles.get_mut(&to) {
            t.visited = true;
        }
        let moved = w.riot_step(&cards);
        // a shrine with nothing to heal, or a hex with nothing to take, keeps its use
        let idle = match w.tile(to).and_then(|t| t.mark).map(|m| m.kind) {
            Some(MarkKind::Shrine) => p.health >= p.max_health(&armoury),
            Some(MarkKind::Hex) => p.health <= 1,
            _ => false,
        };
        let used = if idle { None } else { w.use_mark(to) };
        if !grew.is_empty() {
            self.message = self.s("board.grew");
        }
        if let Some(kind) = used {
            let m = self.use_mark(kind);
            self.note(m);
        }
        let lapsed = self
            .player()
            .contract
            .as_ref()
            .filter(|c| self.player().steps > c.by)
            .cloned();
        if let Some(c) = lapsed {
            let name = self.cards[self.patron_card(&c.from)].name.clone();
            let p = self.player.as_mut().expect("player");
            p.contract = None;
            let harm = CONTRACT_HARM.min(p.health - 1).max(0);
            p.health -= harm;
            let l = p.loyalty.entry(c.from.clone()).or_insert(0);
            *l = (*l - CONTRACT_LOYALTY).max(0);
            let m = self.f(
                "contract.lapsed",
                &[
                    ("patron", name),
                    ("n", harm.to_string()),
                    ("loyalty", CONTRACT_LOYALTY.to_string()),
                ],
            );
            self.note(m);
        }
        self.walk = Some((from, to, self.now));
        self.statics_dirty = true;
        if self.player().health <= 0 {
            self.defeat(vec![self.s("board.starved")]);
            return Ok(());
        }
        if let Some(r) = self.world().riot.clone() {
            if r.ended == Some(Ending::Fell) {
                self.screen = Screen::Result {
                    lines: vec![self.s("riot.fell")],
                    next: Next::Victory,
                };
                self.write_save();
                return Ok(());
            }
            if moved.is_some() {
                let m = self.f("riot.moved", &[("n", r.left().to_string())]);
                self.message = [self.message.clone(), m]
                    .into_iter()
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            if r.roaming() && r.pos() == to {
                self.write_save();
                self.screen = Screen::Rakdos;
                return Ok(());
            }
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
            Feature::Cards if !tile.razed && self.player().contract.as_ref().map(|c| c.at) == Some(at) => {
                let card = self.player().contract.as_ref().expect("contract").creature;
                self.screen = Screen::Draw { at, card };
                if !lines.is_empty() {
                    self.message = lines.join(" ");
                }
            }
            Feature::Cards if tile.minion.is_some() => {
                self.screen = Screen::Draw {
                    at,
                    card: tile.minion.expect("minion"),
                };
                if !lines.is_empty() {
                    self.message = lines.join(" ");
                }
            }
            Feature::Place(place) => {
                let mut all = vec![self.s(&format!("place.{}.text", place.id()))];
                all.extend(lines);
                all.extend(self.happened(Happened::AtPlace(place)));
                self.screen = Screen::Result {
                    lines: all,
                    next: Next::Board,
                };
                self.write_save();
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

    /// Adds a sentence to the board's message.
    fn note(&mut self, m: String) {
        self.message = [self.message.clone(), m]
            .into_iter()
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
    }

    /// A mark acting as you step onto its tile (T39).
    fn use_mark(&mut self, kind: MarkKind) -> String {
        let armoury = self.armoury.clone();
        let p = self.player.as_mut().expect("player");
        let n = match kind {
            MarkKind::Shrine => {
                let n = crate::world::SHRINE_HEAL
                    .min(p.max_health(&armoury) - p.health)
                    .max(0);
                p.health += n;
                n
            }
            // a hex hurts and never kills
            MarkKind::Hex => {
                let n = crate::world::HEX_HARM.min(p.health - 1).max(0);
                p.health -= n;
                n
            }
            MarkKind::Device => {
                p.gold += crate::world::DEVICE_GOLD;
                crate::world::DEVICE_GOLD
            }
        };
        self.f(&format!("mark.{}.used", kind.id()), &[("n", n.to_string())])
    }

    /// A story card leaves a mark on its tile (T39).
    fn put_mark(&mut self, at: Pos, kind: MarkKind) -> String {
        self.world.as_mut().expect("world").mark(at, kind);
        self.statics_dirty = true;
        self.f(
            &format!("mark.{}.made", kind.id()),
            &[("n", kind.uses().to_string())],
        )
    }

    /// A patron's card by number.
    fn patron_card(&self, number: &str) -> usize {
        self.cards.iter().position(|c| c.number == number).unwrap_or(0)
    }

    /// A patron's loyalty: what it was left at, or the card's own to begin.
    pub fn loyalty_of(&self, card: usize) -> i32 {
        let k = &self.cards[card];
        self.player()
            .loyalty
            .get(&k.number)
            .copied()
            .unwrap_or(k.rules.loyalty.unwrap_or(0))
    }

    /// The change in loyalty of a patron's `i`th loyalty ability, from the card's rules.
    pub fn ability_change(&self, card: usize, i: usize) -> Option<i32> {
        self.cards[card]
            .rules
            .activated
            .iter()
            .filter_map(|a| a.loyalty)
            .nth(i)
    }

    /// The strongest creature within Vraska's gaze, not on your own tile:
    /// a minion or a creature in a pool not yet drawn.
    pub fn petrify_target(&self) -> Option<(Pos, usize)> {
        let you = self.player().pos;
        let mut best: Option<(usize, i32, Pos, usize)> = None;
        for (&q, t) in &self.world().tiles {
            let d = crate::world::distance(q, you);
            if d == 0 || d > PETRIFY_REACH || t.feature != Feature::Cards {
                continue;
            }
            let options: Vec<usize> = match t.minion {
                Some(m) => vec![m],
                None if !t.cleared => t
                    .pool
                    .iter()
                    .copied()
                    .filter(|&c| self.cards[c].kind() == Kind::Creature)
                    .collect(),
                None => Vec::new(),
            };
            for c in options {
                let key = (self.cards[c].value, -d, q, c);
                if best.map(|b| key > b).unwrap_or(true) {
                    best = Some(key);
                }
            }
        }
        best.map(|(_, _, q, c)| (q, c))
    }

    /// Tiles a contract can name: not yet drawn, with a pool, at a fair walk.
    fn contract_tiles(&self) -> Vec<Pos> {
        let you = self.player().pos;
        self.world()
            .tiles
            .iter()
            .filter(|(&q, t)| {
                let d = crate::world::distance(q, you);
                (CONTRACT_NEAR.0..=CONTRACT_NEAR.1).contains(&d)
                    && t.feature == Feature::Cards
                    && !t.cleared
                    && !t.razed
                    && !t.pool.is_empty()
            })
            .map(|(&q, _)| q)
            .collect()
    }

    /// Jace's split of three cards of the tile's colours: the most valued
    /// card alone and face up, the other two face down.
    fn piles_for(&mut self, at: Pos) -> [Vec<usize>; 2] {
        let colours = self
            .world()
            .tile(at)
            .map(|t| t.colours())
            .unwrap_or(crate::mana::ANY);
        let mut options: Vec<usize> = (0..self.cards.len())
            .filter(|&c| {
                let k = &self.cards[c];
                !k.is(content::ty::LAND) && k.identity != 0 && k.identity & !colours == 0
            })
            .collect();
        let mut picked = Vec::new();
        while picked.len() < PILE_CARDS && !options.is_empty() {
            let i = (self.rng.next_u64() % options.len() as u64) as usize;
            picked.push(options.remove(i));
        }
        let worth = |c: usize| {
            let k = &self.cards[c];
            let r = match k.rarity {
                'm' => 3,
                'r' => 2,
                'u' => 1,
                _ => 0,
            };
            (r, k.value, c)
        };
        picked.sort_by_key(|&c| std::cmp::Reverse(worth(c)));
        let rest = picked.split_off(picked.len().min(1));
        [picked, rest]
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
        // a card won goes to the collection; the player puts it in the deck (T22)
        p.collection.push(c);
        let name = self.cards[c].name.clone();
        self.f("effect.card", &[("card", name)])
    }

    fn clear(&mut self, at: Pos) {
        if let Some(t) = self.world.as_mut().and_then(|w| w.tiles.get_mut(&at)) {
            t.cleared = true;
            t.minion = None;
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
                Effect::Piles(_) => {
                    let piles = self.piles_for(at);
                    next = Some(Next::Piles { card, piles });
                }
                Effect::Contract(_) => {
                    let options = self.contract_tiles();
                    if !options.is_empty() {
                        let q = options[(self.rng.next_u64() % options.len() as u64) as usize];
                        let creature = self.creature_near(q, card);
                        let by = self.player().steps + CONTRACT_STEPS;
                        let number = self.cards[card].number.clone();
                        self.player.as_mut().expect("player").contract = Some(player::Contract {
                            at: q,
                            creature,
                            by,
                            from: number,
                        });
                        let away = crate::world::distance(q, self.player().pos);
                        lines.push(self.f(
                            "contract.set",
                            &[
                                ("card", self.cards[creature].name.clone()),
                                ("away", away.to_string()),
                                ("n", CONTRACT_STEPS.to_string()),
                            ],
                        ));
                        self.statics_dirty = true;
                    }
                }
                Effect::Petrify(_) => {
                    if let Some((q, c)) = self.petrify_target() {
                        self.clear(q);
                        let line = self.gain_card(c);
                        lines.push(self.f("patron.petrified", &[("card", self.cards[c].name.clone())]));
                        lines.push(line);
                    }
                }
                Effect::Assassins(n) => {
                    self.player.as_mut().expect("player").assassins += (*n).max(0) as u32;
                    lines.push(self.f("patron.assassins", &[("n", n.to_string())]));
                }
                Effect::Rares(n) => {
                    let colours = self.guild.colours();
                    let mut options: Vec<usize> = (0..self.cards.len())
                        .filter(|&c| {
                            let k = &self.cards[c];
                            matches!(k.rarity, 'r' | 'm')
                                && !k.is(content::ty::LAND)
                                && k.identity != 0
                                && k.identity & !colours == 0
                        })
                        .collect();
                    for _ in 0..*n {
                        if options.is_empty() {
                            break;
                        }
                        let i = (self.rng.next_u64() % options.len() as u64) as usize;
                        let c = options.remove(i);
                        lines.push(self.gain_card(c));
                    }
                }
                Effect::Enchant(id) => {
                    if let Some(kind) = MarkKind::from_id(id) {
                        lines.push(self.put_mark(at, kind));
                    }
                }
                Effect::Install(_) => lines.push(self.put_mark(at, MarkKind::Device)),
                Effect::Open(n) => {
                    let opened = self
                        .world
                        .as_mut()
                        .expect("world")
                        .open_around(at, (*n).max(0) as usize);
                    lines.push(if opened == 0 {
                        self.s("open.none")
                    } else {
                        self.f("open.done", &[("n", opened.to_string())])
                    });
                    self.statics_dirty = true;
                }
                Effect::Rebuild(_) => {
                    let n = self.world.as_mut().expect("world").rebuild_around(at);
                    lines.push(if n == 0 {
                        self.s("rebuild.none")
                    } else {
                        self.f("rebuild.done", &[("n", n.to_string())])
                    });
                    self.statics_dirty = true;
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
        if matches!(self.cards[card].kind(), Kind::Creature | Kind::Patron) {
            return card;
        }
        self.creature_near(at, card)
    }

    /// The strongest creature in a tile's pool, else one of its colours.
    fn creature_near(&mut self, at: Pos, card: usize) -> usize {
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
        let weakened = self.world().riot.as_ref().map(|r| r.weakened).unwrap_or(false);
        let mut foe = if guardian && self.cards[creature].name == "Rakdos, Lord of Riots" {
            encounter::rakdos(&self.cards, weakened)
        } else if guardian {
            encounter::guardian(&self.cards)
        } else if self.world().tile(at).and_then(|t| t.minion) == Some(creature) {
            encounter::minion(&self.cards, creature, &mut self.rng)
        } else {
            let pool = self.world().tile(at).map(|t| t.pool.clone()).unwrap_or_default();
            encounter::foe(&self.cards, creature, &pool, &mut self.rng)
        };
        if !guardian {
            foe.name = self.s("battle.foe_side");
        }
        let p = self.player.as_mut().expect("player");
        if p.assassins > 0 {
            // Vraska's assassins went ahead of you
            p.assassins -= 1;
            foe.life = (foe.life + 1) / 2;
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
        self.log_open = false;
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
                if f.guardian && self.cards[f.creature].name == "Rakdos, Lord of Riots" {
                    let steps = self.player().steps;
                    if let Some(r) = self.world.as_mut().and_then(|w| w.riot.as_mut()) {
                        r.ended = Some(Ending::Beaten { steps });
                    }
                    lines.push(self.gain_card(f.creature));
                    lines.extend(self.happened(Happened::BeatRakdos));
                    // the mission ends with him, whatever step its quest was at
                    if let Some(n) = self.quests.iter().find(|q| q.id == "riot").map(|q| q.steps.len()) {
                        self.player
                            .as_mut()
                            .expect("player")
                            .quests
                            .insert("riot".into(), n as u8);
                    }
                    let grade = self.s(&format!("riot.grade.{}", riot::grade(steps)));
                    lines.push(self.f("riot.beaten", &[("steps", steps.to_string()), ("grade", grade)]));
                    self.statics_dirty = true;
                    self.screen = Screen::Result {
                        lines,
                        next: Next::Victory,
                    };
                    self.write_save();
                    return;
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
                let paid = self
                    .player()
                    .contract
                    .as_ref()
                    .filter(|c| c.at == f.at && c.creature == f.creature)
                    .cloned();
                if let Some(c) = paid {
                    let name = self.cards[self.patron_card(&c.from)].name.clone();
                    let p = self.player.as_mut().expect("player");
                    p.contract = None;
                    p.gold += CONTRACT_GOLD;
                    p.guild_standing(Guild::Golgari, CONTRACT_STANDING);
                    lines.push(self.f(
                        "contract.paid",
                        &[("patron", name), ("n", CONTRACT_GOLD.to_string())],
                    ));
                    lines.push(self.f(
                        "effect.style",
                        &[
                            ("guild", Guild::Golgari.name().to_string()),
                            ("n", CONTRACT_STANDING.to_string()),
                        ],
                    ));
                    self.statics_dirty = true;
                }
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
            "game.new" => self.screen = Screen::Mission,
            "mission.choose" => {
                if !value.is_empty() {
                    self.mission =
                        Mission::from_id(value).ok_or_else(|| format!("There is no mission \"{value}\"."))?;
                }
                self.screen = Screen::NewGame;
            }
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
                self.screen = Screen::Starter { guild: g, pick: 0 };
            }
            "hall.take" => {
                let Screen::City { city, .. } = self.screen else {
                    return Err(self.s("error.not_in_hall"));
                };
                let posted = self
                    .quests
                    .iter()
                    .any(|q| q.id == value && q.starts == format!("hall:{}", city.id()));
                if !posted {
                    return Err(self.s("error.not_in_hall"));
                }
                let p = self.player.as_mut().expect("player");
                if !p.quests.contains_key(value) {
                    p.quests.insert(value.to_string(), 0);
                    let name = self.s(&format!("quest.{value}"));
                    let first = self.s(&format!("quest.{value}.0"));
                    self.message = self.f("quest.started", &[("quest", name), ("next", first)]);
                    self.write_save();
                }
            }
            "starter.look" => {
                let n = num(value)?;
                if let Screen::Starter { guild, .. } = self.screen {
                    if n >= encounter::starter_ids(guild).len() {
                        return Err(format!("There is no deck {n}."));
                    }
                    self.screen = Screen::Starter { guild, pick: n };
                }
            }
            "starter.start" => {
                if let Screen::Starter { guild, pick } = self.screen {
                    self.new_game(guild, encounter::starter_ids(guild)[pick]);
                }
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
            "deck.sort" => {
                self.deck_sort = DECK_SORTS
                    .iter()
                    .find(|k| **k == value)
                    .ok_or_else(|| format!("\"{value}\" is not a way to sort."))?;
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
                if !self.can_choose(card, &ch) {
                    return Err(self.s("error.choice_locked"));
                }
                let mut said = Vec::new();
                if let Some(change) = ch.ability.and_then(|a| self.ability_change(card, a)) {
                    // a patron's loyalty moves with each ability, and is kept
                    let l = self.loyalty_of(card) + change;
                    let number = self.cards[card].number.clone();
                    self.player.as_mut().expect("player").loyalty.insert(number, l);
                    said.push(self.f(
                        "patron.loyalty",
                        &[("patron", self.cards[card].name.clone()), ("n", l.to_string())],
                    ));
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
                    let (mut lines, next) = self.apply(at, card, &ch.outcome);
                    lines.extend(said);
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
                if !face.good() && self.player().held.is_some() {
                    // the held instant can answer it (T38): wait for the player
                    return Ok(());
                }
                self.settle_fate(at, card, choice, face, face)?;
            }
            "fate.accept" | "fate.answer" => {
                let Screen::Fate {
                    at,
                    card,
                    choice,
                    faces,
                    picked: Some(i),
                    ..
                } = self.screen.clone()
                else {
                    return Ok(());
                };
                let face = faces[i];
                let Some(held) = self.player().held.filter(|_| !face.good()) else {
                    return Ok(());
                };
                if name == "fate.accept" {
                    self.settle_fate(at, card, choice, face, face)?;
                } else {
                    self.player.as_mut().expect("player").held = None;
                    self.settle_fate(at, card, choice, face, Face::Success)?;
                    let line = self.f(
                        "held.answered",
                        &[
                            ("card", self.cards[held].name.clone()),
                            ("face", self.s(face.key())),
                        ],
                    );
                    if let Screen::Result { lines, .. } = &mut self.screen {
                        lines.insert(1, line);
                    }
                }
            }
            "piles.pick" => {
                let Screen::Piles { piles, .. } = self.screen.clone() else {
                    return Ok(());
                };
                let pile = piles
                    .get(num(value)?)
                    .cloned()
                    .ok_or_else(|| self.s("error.no_choice"))?;
                let lines: Vec<String> = pile.iter().map(|&c| self.gain_card(c)).collect();
                self.after(lines, None);
            }
            "draw.keep" => {
                let Screen::Draw { at, card } = self.screen.clone() else {
                    return Ok(());
                };
                if !self.cards[card].is(content::ty::INSTANT) {
                    return Ok(());
                }
                let old = self.player.as_mut().expect("player").held.replace(card);
                self.clear(at);
                let mut lines = vec![self.f("held.kept", &[("card", self.cards[card].name.clone())])];
                if let Some(o) = old {
                    lines.push(self.f("held.dropped", &[("card", self.cards[o].name.clone())]));
                }
                self.after(lines, None);
            }
            "draw.slip" => {
                let Screen::Draw { card, .. } = self.screen.clone() else {
                    return Ok(());
                };
                let Some(held) = self.player().held else {
                    return Ok(());
                };
                if self.cards[card].kind() != Kind::Creature {
                    return Ok(());
                }
                // the card stays where it is; you go on past it
                self.player.as_mut().expect("player").held = None;
                let line = self.f(
                    "held.slipped",
                    &[
                        ("card", self.cards[held].name.clone()),
                        ("foe", self.cards[card].name.clone()),
                    ],
                );
                self.after(vec![line], None);
            }
            "riot.detain" => {
                if !matches!(self.screen, Screen::Rakdos) {
                    return Ok(());
                }
                let Some(held) = self.player().held else {
                    return Ok(());
                };
                self.player.as_mut().expect("player").held = None;
                if let Some(r) = self.world.as_mut().and_then(|w| w.riot.as_mut()) {
                    r.detained = riot::DETAIN_STEPS;
                }
                let line = self.f(
                    "held.detained",
                    &[
                        ("card", self.cards[held].name.clone()),
                        ("n", riot::DETAIN_STEPS.to_string()),
                    ],
                );
                self.after(vec![line], None);
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
                    Next::Piles { card, piles } => self.screen = Screen::Piles { card, piles },
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
            "riot.fight" => {
                if !matches!(self.screen, Screen::Rakdos) {
                    return Ok(());
                }
                let me = self
                    .cards
                    .iter()
                    .position(|c| c.name == "Rakdos, Lord of Riots")
                    .unwrap_or(0);
                let at = self.player().pos;
                self.start_fight(at, me, None, true);
            }
            "lair.leave" | "victory.continue" | "riot.leave" => self.screen = Screen::Board,
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
            "view.log" => self.log_open = value == "open",
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

    /// Settles a fate draw: `drew` is the face picked, `face` the one that
    /// acts (a held instant turns a bad face to a Success).
    fn settle_fate(
        &mut self,
        at: Pos,
        card: usize,
        choice: usize,
        drew: Face,
        face: Face,
    ) -> Result<(), String> {
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
        lines.insert(0, self.f("fate.drew", &[("face", self.s(drew.key()))]));
        self.after(lines, next);
        Ok(())
    }

    pub fn can_choose(&self, card: usize, ch: &story::Choice) -> bool {
        let p = self.player();
        if let Some(change) = ch.ability.and_then(|a| self.ability_change(card, a)) {
            // loyalty never goes below nothing
            if self.loyalty_of(card) + change < 0 {
                return false;
            }
        }
        let wants = |e: &Effect| ch.outcome.effects.contains(e);
        if wants(&Effect::Petrify(true)) && self.petrify_target().is_none() {
            return false;
        }
        if wants(&Effect::Contract(true)) && (p.contract.is_some() || self.contract_tiles().is_empty()) {
            return false;
        }
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
            Screen::Battle(_) => match self.battle_under(x, y) {
                Some(id) => self.act_json("duel.click", &id.to_string()),
                None => serde_json::json!({"ok": true}).to_string(),
            },
            _ => serde_json::json!({"ok": true}).to_string(),
        }
    }

    /// The permanent drawn nearest pixel (x, y) in a fight, if one is near.
    fn battle_under(&self, x: i32, y: i32) -> Option<u32> {
        let Screen::Battle(f) = &self.screen else {
            return None;
        };
        let m = crate::scene::camera_for(self)
            .matrix((self.size.0 as i64 * crate::fx::ONE as i64 / self.size.1.max(1) as i64) as i32);
        let reach = (self.size.1 / 14).max(1) as i64;
        let snap = &f.event().snap;
        crate::scene::battle_spots(self, snap)
            .into_iter()
            .filter_map(|(pt, p)| {
                let q = crate::geom::project(&m, pt.add(crate::geom::v3(0, crate::fx::ONE, 0)))?;
                let (px, py) = (q.0 * self.size.0 / 1000, q.1 * self.size.1 / 1000);
                Some((((px - x) as i64).pow(2) + ((py - y) as i64).pow(2), p.id))
            })
            .filter(|(d, _)| *d <= (reach * 2).pow(2))
            .min()
            .map(|(_, id)| id)
    }

    /// The pointer moved over the 3D view: what lies under it is shown
    /// full size. The view changes only when that changes.
    pub fn hover(&mut self, x: i32, y: i32) {
        let h = self.battle_under(x, y);
        if h != self.hovered {
            self.hovered = h;
            self.changed();
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
        s.act("starter.start", "").unwrap();
        s
    }

    #[test]
    fn a_new_game_starts_with_the_starting_deck_looked_at_last() {
        let mut s = for_tests();
        s.act("game.new", "").unwrap();
        s.act("game.guild", "Golgari").unwrap();
        assert!(s.world.is_none(), "choosing a guild shows its decks first");
        s.act("starter.look", "1").unwrap();
        s.act("starter.look", "5").unwrap();
        assert!(s.act("starter.look", "6").is_err());
        s.act("starter.start", "").unwrap();
        let vraska = s
            .cards
            .iter()
            .position(|c| c.name == "Vraska the Unseen")
            .unwrap();
        assert_eq!(s.player().deck.len(), encounter::STARTER_DECK);
        assert!(s.player().deck.contains(&vraska));
        assert_eq!(s.player().collection, s.player().deck);
    }

    #[test]
    fn the_whole_log_opens_over_the_table_with_every_line_so_far() {
        let mut s = started();
        let caracal = s.cards.iter().position(|c| c.name == "Trained Caracal").unwrap();
        s.world.as_mut().unwrap().tiles.get_mut(&(1, 0)).unwrap().cleared = false;
        s.screen = Screen::Draw {
            at: (1, 0),
            card: caracal,
        };
        s.act("draw.fight", "").unwrap();
        // pass until the fight has said enough
        for _ in 0..12 {
            let _ = s.act("duel.ok", "");
            s.act("battle.skip", "").unwrap();
        }
        let shown = match &s.screen {
            Screen::Battle(f) => f.shown,
            _ => panic!("no fight"),
        };
        assert!(shown > 8, "enough lines for the small log to drop some");
        let small = crate::view::ui(&mut s);
        let log = small["panels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == "log")
            .unwrap();
        assert_eq!(log["log"].as_array().unwrap().len(), shown + 1);
        s.act("view.log", "open").unwrap();
        let v = crate::view::ui(&mut s);
        assert_eq!(v["screen"], "battlelog");
        let panels = v["panels"].as_array().unwrap();
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0]["log"].as_array().unwrap().len(), shown + 1);
        s.act("view.log", "close").unwrap();
        assert_eq!(crate::view::ui(&mut s)["screen"], "battle");
    }

    #[test]
    fn rix_maadi_posts_its_own_quests_and_sells_its_own_gear() {
        let mut s = started();
        s.screen = Screen::City {
            city: City::Rix,
            room: Room::Hall,
        };
        assert!(
            s.act("hall.take", "writ").is_err(),
            "a quest begun elsewhere is not posted here"
        );
        s.act("hall.take", "encore").unwrap();
        assert_eq!(s.player().quests.get("encore"), Some(&0));
        let rix = encounter::shop_armour(&s.armoury, City::Rix);
        let tenth = encounter::shop_armour(&s.armoury, City::Tenth);
        assert!(rix.contains(&"rix_head".to_string()) && rix.contains(&"rakdos_head".to_string()));
        assert!(!tenth.contains(&"rix_head".to_string()) && tenth.contains(&"rakdos_head".to_string()));
        assert!(!encounter::shop_armour(&s.armoury, City::Svogthos).contains(&"rix_head".to_string()));
        s.screen = Screen::City {
            city: City::Tenth,
            room: Room::Hall,
        };
        assert!(
            s.act("hall.take", "harvest").is_err(),
            "Svogthos's quest is taken up at Svogthos"
        );
    }

    #[test]
    fn the_inn_sells_six_supplies_for_two_gold() {
        let mut s = started();
        let p = s.player.as_mut().unwrap();
        p.gold = 10;
        p.supplies = 2;
        s.act("inn.supplies", "").unwrap();
        assert_eq!((s.player().supplies, s.player().gold), (8, 8));
    }

    #[test]
    fn a_card_won_joins_the_collection_and_not_the_deck() {
        let mut s = started();
        let caracal = s.cards.iter().position(|c| c.name == "Trained Caracal").unwrap();
        let deck = s.player().deck.clone();
        let had = s.player().collection.len();
        let line = s.gain_card(caracal);
        assert_eq!(s.player().deck, deck);
        assert_eq!(s.player().collection.len(), had + 1);
        assert!(line.contains("Trained Caracal"));
    }

    #[test]
    fn the_deck_screen_sorts_by_colour_type_rarity_name_or_mana_value() {
        let mut s = started();
        let id = |s: &Session, n: &str| s.cards.iter().position(|c| c.name == n).unwrap();
        let (caracal, island_spell) = (id(&s, "Trained Caracal"), id(&s, "Cancel"));
        // Trained Caracal is white and costs 1; Cancel is blue and costs 3
        s.act("deck.sort", "colour").unwrap();
        assert!(crate::view::deck_order(&s, caracal) < crate::view::deck_order(&s, island_spell));
        s.act("deck.sort", "type").unwrap();
        assert!(
            crate::view::deck_order(&s, caracal) < crate::view::deck_order(&s, island_spell),
            "a creature before an instant"
        );
        s.act("deck.sort", "name").unwrap();
        assert!(
            crate::view::deck_order(&s, island_spell) < crate::view::deck_order(&s, caracal),
            "C before T"
        );
        assert!(s.act("deck.sort", "weight").is_err());
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
    fn hovering_a_creature_in_a_fight_shows_its_card_and_moving_off_hides_it() {
        let mut s = started();
        s.act("resize", "1280,720,1").unwrap();
        let caracal = s.cards.iter().position(|c| c.name == "Trained Caracal").unwrap();
        s.world.as_mut().unwrap().tiles.get_mut(&(1, 0)).unwrap().cleared = false;
        s.screen = Screen::Draw {
            at: (1, 0),
            card: caracal,
        };
        s.act("draw.fight", "").unwrap();
        s.act("battle.skip", "").unwrap();
        let Screen::Battle(f) = &s.screen else {
            panic!("no fight")
        };
        let snap = f.event().snap.clone();
        let (pt, p) = crate::scene::battle_spots(&s, &snap)
            .into_iter()
            .find(|(_, p)| p.creature)
            .map(|(a, p)| (a, p.clone()))
            .unwrap();
        let m = crate::scene::camera_for(&s).matrix(1280 * crate::fx::ONE / 720);
        let q = crate::geom::project(&m, pt.add(crate::geom::v3(0, crate::fx::ONE, 0))).unwrap();
        s.hover(q.0 * 1280 / 1000, q.1 * 720 / 1000);
        assert_eq!(s.hovered, Some(p.id));
        let ui = s.ui_json();
        assert!(ui.contains("\"id\":\"peek\""), "no peek panel");
        assert!(ui.contains("Trained Caracal"));
        s.hover(-100_000, -100_000);
        assert_eq!(s.hovered, None);
        assert!(!s.ui_json().contains("\"id\":\"peek\""));
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
    fn a_fight_shows_its_phase_its_main_button_and_damage_as_numbers() {
        let mut s = started();
        let caracal = s.cards.iter().position(|c| c.name == "Trained Caracal").unwrap();
        s.world.as_mut().unwrap().tiles.get_mut(&(1, 0)).unwrap().cleared = false;
        s.screen = Screen::Draw {
            at: (1, 0),
            card: caracal,
        };
        s.act("draw.fight", "").unwrap();
        let panel = |v: &serde_json::Value, id: &str| {
            v["panels"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["id"] == id)
                .cloned()
                .unwrap_or_default()
        };
        // waiting on the player's first move, played back to now: the main
        // button only waits, and each card in the hand is playable or dimmed
        if let Screen::Battle(f) = &mut s.screen {
            f.shown = f.duel.events.len() - 1;
        }
        let v = crate::view::ui(&mut s);
        assert_eq!(panel(&v, "controls")["actions"][0]["main"], "optional");
        // play lands and pass until a card in the hand can be cast: it glows,
        // and a card that cannot be cast is dimmed
        let mut seen = false;
        for _ in 0..60 {
            let v = crate::view::ui(&mut s);
            let hand = panel(&v, "hand")["cards"].as_array().cloned().unwrap_or_default();
            if panel(&v, "controls")["heading"] == s.s("duel.target") {
                break;
            }
            // the game waits on the player's move (a discard asks otherwise)
            let priority = panel(&v, "controls")["actions"][0]["main"] == "optional";
            if priority
                && hand
                    .iter()
                    .any(|c| !c["act"].is_null() || !c["actions"].is_null())
            {
                for c in &hand {
                    let castable = !c["act"].is_null() || !c["actions"].is_null();
                    assert_eq!(c["playable"] == true, castable, "{}", c["name"]);
                    assert_ne!(c["playable"] == true, c["dim"] == true, "{}", c["name"]);
                }
                seen = true;
                break;
            }
            let land = panel(&v, "controls")["list"].as_array().and_then(|l| {
                l.iter()
                    .find(|b| b["label"].as_str().unwrap_or("").starts_with("Play"))
                    .cloned()
            });
            match land {
                Some(b) => s.act("duel.do", b["value"].as_str().unwrap()).unwrap(),
                None => {
                    let _ = s.act("duel.ok", "");
                }
            }
            if let Screen::Battle(f) = &mut s.screen {
                f.shown = f.duel.events.len() - 1;
            } else {
                break;
            }
        }
        assert!(seen, "no card in the hand could be cast");
        if let Screen::Battle(f) = &mut s.screen {
            f.duel.human = [false, false];
            f.duel.run();
        }
        let n = match &s.screen {
            Screen::Battle(f) => f.duel.events.len(),
            _ => panic!("no fight"),
        };
        let mut floats = 0;
        for i in 0..n {
            if let Screen::Battle(f) = &mut s.screen {
                f.shown = i;
            }
            let (step, ev, dmg) = match &s.screen {
                Screen::Battle(f) => {
                    let e = &f.duel.events[i];
                    let n = e.args.iter().find(|(k, _)| *k == "n").map(|(_, v)| v.clone());
                    (e.snap.step, e.ev, n)
                }
                _ => unreachable!(),
            };
            let v = crate::view::ui(&mut s);
            // one phase is marked, the one the step belongs to
            let phases = panel(&v, "sides")["phases"].as_array().unwrap().clone();
            assert_eq!(phases.len(), 5);
            let now: Vec<usize> = (0..5).filter(|&k| phases[k]["now"] == true).collect();
            // worked out by hand from Magic's turn: beginning, first main,
            // combat, second main, end
            use crate::duel::Step as St;
            let want = match step {
                St::Upkeep | St::Draw => 0,
                St::Main1 => 1,
                St::Combat | St::Attackers | St::Blockers | St::FirstDamage | St::Damage | St::EndCombat => 2,
                St::Main2 => 3,
                St::End | St::Cleanup => 4,
            };
            if i + 1 < n {
                // the last event is the game caught up, whose step is the game's own
                assert_eq!(now, vec![want], "event {i}");
                // played back, nothing is asked of the player: no main button
                assert!(panel(&v, "controls")["actions"][0]["main"].is_null(), "event {i}");
            }
            let labels: Vec<&serde_json::Value> = v["labels"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|l| l["float"] == "damage")
                .collect();
            // this event's number, under its own key
            let own: Vec<&&serde_json::Value> = labels.iter().filter(|l| l["key"] == i).collect();
            if matches!(ev, Ev::Face) {
                // a player's damage rises from their side, which is always on screen
                assert_eq!(own.len(), 1, "event {i}");
            }
            if let Some(l) = own.first() {
                assert!(matches!(ev, Ev::Hit | Ev::Face));
                assert_eq!(l["text"], format!("−{}", dmg.unwrap()));
                floats += 1;
            } else {
                assert!(!matches!(ev, Ev::Face));
            }
            // a number stays four events, then goes
            for l in &labels {
                let k = l["key"].as_u64().unwrap() as usize;
                assert!(k <= i && i - k < 4, "event {i} shows {k}'s number");
            }
            if i >= 3 {
                if let Screen::Battle(f) = &s.screen {
                    let back = &f.duel.events[i - 3];
                    if matches!(back.ev, Ev::Face) {
                        assert!(
                            labels.iter().any(|l| l["key"] == i - 3),
                            "event {i}: {}'s number went early",
                            i - 3
                        );
                    }
                }
            }
        }
        assert!(floats > 0, "no damage was dealt in the whole fight");
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

    fn riot() -> Session {
        let mut s = for_tests();
        s.act("game.new", "").unwrap();
        assert!(matches!(s.screen, Screen::Mission));
        s.act("mission.choose", "riot").unwrap();
        s.act("game.guild", "Izzet").unwrap();
        s.act("starter.start", "").unwrap();
        s.screen = Screen::Board;
        s
    }

    /// Rakdos's road made short for a test: from `first`, then each of `rest`.
    fn road(s: &mut Session, first: Pos, rest: &[Pos]) {
        let r = s.world.as_mut().unwrap().riot.as_mut().unwrap();
        r.route = std::iter::once(first).chain(rest.iter().copied()).collect();
        r.at = 0;
        r.clock = 0;
    }

    fn win_the_fight(s: &mut Session) {
        let Screen::Battle(f) = &mut s.screen else {
            panic!("no fight")
        };
        f.duel.outcome = Some(Outcome::Won);
        f.shown = f.duel.events.len().saturating_sub(1);
        s.act("battle.done", "").unwrap();
    }

    #[test]
    fn the_riot_begins_with_rakdos_at_rix_maadi_and_its_own_three_quests() {
        let s = riot();
        let r = s.world().riot.as_ref().unwrap();
        assert_eq!(r.pos(), (0, 5));
        assert!(r.roaming());
        let q = &s.player().quests;
        // the Tenth District step is met as the game begins
        assert_eq!(q.get("riot"), Some(&1));
        assert_eq!(q.get("calliope"), Some(&0));
        assert_eq!(q.get("column"), Some(&0));
        assert!(!q.contains_key("dracogenius"));
        let d = started();
        assert!(d.world().riot.is_none());
        assert_eq!(d.player().quests.get("dracogenius"), Some(&1));
        assert!(!d.player().quests.contains_key("riot"));
    }

    #[test]
    fn stepping_onto_rakdos_faces_him_and_beating_him_grades_the_steps() {
        let mut s = riot();
        road(&mut s, (1, 0), &[(2, 0), (3, 0), (0, -5)]);
        s.act("move", "e").unwrap();
        assert!(matches!(s.screen, Screen::Rakdos));
        assert!(s.ui_json().contains("Fight Rakdos"));
        s.act("riot.fight", "").unwrap();
        let Screen::Battle(f) = &s.screen else {
            panic!("no fight")
        };
        assert!(f.guardian);
        assert_eq!(s.cards[f.creature].name, "Rakdos, Lord of Riots");
        assert_eq!(f.duel.s[1].life, 12);
        win_the_fight(&mut s);
        // one step taken: grade A
        let Screen::Result { lines, next } = &s.screen else {
            panic!("no result")
        };
        assert!(matches!(next, Next::Victory));
        assert!(
            lines.iter().any(|l| l == "Rakdos is beaten in 1 steps: grade A."),
            "{lines:?}"
        );
        assert_eq!(
            s.world().riot.as_ref().unwrap().ended,
            Some(Ending::Beaten { steps: 1 })
        );
        assert_eq!(s.player().quests.get("riot"), Some(&3));
        s.act("result.next", "").unwrap();
        assert!(s.ui_json().contains("Rakdos falls: grade A"));
        // he is gone from the map: no more moves, no mark
        s.act("victory.continue", "").unwrap();
        assert!(s.quest_marks().iter().all(|(_, q)| q != RAKDOS_MARK));
    }

    #[test]
    fn leaving_rakdos_keeps_the_game_on_the_board() {
        let mut s = riot();
        road(&mut s, (1, 0), &[(2, 0), (0, -5)]);
        s.act("move", "e").unwrap();
        s.act("riot.leave", "").unwrap();
        assert!(matches!(s.screen, Screen::Board));
    }

    #[test]
    fn rakdos_reaching_new_prahv_loses_the_riot() {
        let mut s = riot();
        road(&mut s, (0, 4), &[(0, -5)]);
        s.world.as_mut().unwrap().riot.as_mut().unwrap().clock = 2;
        s.act("move", "e").unwrap();
        let Screen::Result { lines, next } = &s.screen else {
            panic!("no result")
        };
        assert_eq!(lines, &vec![s.s("riot.fell")]);
        assert!(matches!(next, Next::Victory));
        s.act("result.next", "").unwrap();
        assert!(s.ui_json().contains("New Prahv has fallen"));
        assert_eq!(s.world().riot.as_ref().unwrap().ended, Some(Ending::Fell));
    }

    #[test]
    fn the_lyev_columns_mark_brings_rakdos_to_eight_life() {
        let mut s = riot();
        road(&mut s, (1, 0), &[(0, -5)]);
        s.world.as_mut().unwrap().riot.as_mut().unwrap().weakened = true;
        s.act("move", "e").unwrap();
        s.act("riot.fight", "").unwrap();
        let Screen::Battle(f) = &s.screen else {
            panic!("no fight")
        };
        assert_eq!(f.duel.s[1].life, 8);
    }

    #[test]
    fn jamming_the_calliope_slows_rakdos_to_one_move_in_four() {
        let mut s = riot();
        s.world.as_mut().unwrap().add_chunk((1, 1), &s.cards.clone());
        assert_eq!(
            s.world().tile((6, 4)).unwrap().feature,
            Feature::Place(Place::Calliope)
        );
        s.player.as_mut().unwrap().quests.insert("calliope".into(), 2);
        s.player.as_mut().unwrap().pos = (5, 4);
        s.go((6, 4)).unwrap();
        let Screen::Result { lines, .. } = &s.screen else {
            panic!("no result")
        };
        assert_eq!(lines[0], s.s("place.calliope.text"));
        assert!(
            lines.iter().any(|l| l.contains("once every 4 of your steps")),
            "{lines:?}"
        );
        let r = s.world().riot.as_ref().unwrap();
        assert!(r.slowed);
        assert_eq!(r.every(), 4);
        // the first mission has no calliope
        let mut d = started();
        d.world.as_mut().unwrap().add_chunk((1, 1), &d.cards.clone());
        assert_ne!(
            d.world().tile((6, 4)).unwrap().feature,
            Feature::Place(Place::Calliope)
        );
    }

    #[test]
    fn a_minion_on_a_razed_tile_fights_with_eighteen_life_and_beating_it_frees_the_tile() {
        let mut s = riot();
        road(&mut s, (0, 7), &[(1, 7), (0, -5)]);
        let thug = s.cards.iter().position(|c| c.name == "Splatter Thug").unwrap();
        {
            let t = s.world.as_mut().unwrap().tiles.get_mut(&(1, 0)).unwrap();
            t.razed = true;
            t.minion = Some(thug);
            t.pool.clear();
            t.cleared = false;
        }
        s.act("move", "e").unwrap();
        assert!(matches!(s.screen, Screen::Draw { card, .. } if card == thug));
        assert!(s
            .ui_json()
            .contains("Rakdos's minion: it brings 18 life and 7 lands"));
        s.act("draw.fight", "").unwrap();
        let Screen::Battle(f) = &s.screen else {
            panic!("no fight")
        };
        assert_eq!(f.duel.s[1].life, 18);
        win_the_fight(&mut s);
        let t = s.world().tile((1, 0)).unwrap();
        assert_eq!(t.minion, None);
        assert!(t.cleared && t.razed);
        assert!(s.player().collection.contains(&thug));
    }

    #[test]
    fn a_quests_next_place_is_marked_before_it_is_found_and_nothing_else_is_shown() {
        let mut s = started();
        s.screen = Screen::Board;
        let tiles = s.world().tiles.len();
        // the Dracogenius's next step is New Prahv, two chunks north
        let marks = s.quest_marks();
        assert!(marks.contains(&((0, -5), "dracogenius".to_string())), "{marks:?}");
        assert!(s.world().tile((0, -5)).is_none());
        assert_eq!(s.world().tiles.len(), tiles);
        s.act("resize", "1280,720,1").unwrap();
        let ui: Value = serde_json::from_str(&s.ui_json()).unwrap();
        let label = ui["labels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["text"] == "New Prahv")
            .expect("a label for New Prahv")
            .clone();
        // it lies past the top of the view, so its label is held on screen,
        // and says how far it is: five steps north
        let y = label["y"].as_i64().unwrap();
        assert!((80..=940).contains(&y), "{label}");
        assert_eq!(label["sub"], "The Dracogenius · 5 steps away");
        // a district step points at a district of that guild, as it will be made
        s.player.as_mut().unwrap().quests.insert("lantern".into(), 0);
        let (at, _) = s
            .quest_marks()
            .into_iter()
            .find(|(_, q)| q == "lantern")
            .expect("a mark for the lantern");
        let c = crate::world::chunk_of(at);
        s.world.as_mut().unwrap().add_chunk(c, &s.cards.clone());
        assert_eq!(s.world().tile(at).unwrap().guild, Some(Guild::Izzet));
        // the Riot marks Rakdos wherever he is
        let r = riot();
        let marks = r.quest_marks();
        assert!(marks.contains(&((0, 5), RAKDOS_MARK.to_string())), "{marks:?}");
        // and the Riot quest's own next place, New Prahv, apart from him
        assert!(marks.contains(&((0, -5), "riot".to_string())), "{marks:?}");
        assert!(r.quests.iter().all(|q| q.id != RAKDOS_MARK));
        // the Riot and the Lyev Column both go to New Prahv next, five steps
        // north of the start: one label
        let mut r = r;
        r.act("resize", "1280,720,1").unwrap();
        let ui: Value = serde_json::from_str(&r.ui_json()).unwrap();
        let prahv: Vec<&Value> = ui["labels"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|l| l["text"] == "New Prahv")
            .collect();
        assert_eq!(prahv.len(), 1, "{prahv:?}");
        assert_eq!(prahv[0]["sub"], "The Riot, The Lyev Column · 5 steps away");
    }

    // ---------- T37, T38, T39: patrons, held instants, tile marks ----------

    fn card(s: &Session, name: &str) -> usize {
        s.cards.iter().position(|c| c.name == name).unwrap()
    }

    /// Stands the player on a card tile with all four neighbours open,
    /// cleared ground, and nothing else within three steps to draw.
    fn quiet_tile(s: &mut Session) -> Pos {
        let at = (3, 2);
        let cards = s.cards.clone();
        let w = s.world.as_mut().unwrap();
        for c in [(0, 0), (1, 0), (0, 1), (1, 1), (1, -1)] {
            w.add_chunk(c, &cards);
        }
        for (q, t) in w.tiles.iter_mut() {
            if crate::world::distance(*q, at) <= 6 {
                t.feature = Feature::Cards;
                t.pool.clear();
                t.cleared = true;
                t.razed = false;
                t.minion = None;
                t.mark = None;
            }
        }
        s.player.as_mut().unwrap().pos = at;
        s.screen = Screen::Board;
        at
    }

    fn meet(s: &mut Session, name: &str) -> usize {
        let c = card(s, name);
        let at = s.player().pos;
        s.screen = Screen::Draw { at, card: c };
        s.act("draw.read", "").unwrap();
        assert!(matches!(s.screen, Screen::Story { .. }));
        c
    }

    fn result_lines(s: &Session) -> Vec<String> {
        match &s.screen {
            Screen::Result { lines, .. } => lines.clone(),
            other => panic!("no result: {other:?}"),
        }
    }

    /// A fed step: supplies are topped up so hunger never acts.
    fn step_to(s: &mut Session, p: Pos) {
        s.player.as_mut().unwrap().supplies = player::START_SUPPLIES;
        s.walk = None;
        s.screen = Screen::Board;
        s.go(p).unwrap();
    }

    #[test]
    fn a_patron_offers_its_loyalty_abilities_and_keeps_its_loyalty_between_meetings() {
        let mut s = started();
        let at = quiet_tile(&mut s);
        let jace = card(&s, "Jace, Architect of Thought");
        s.screen = Screen::Draw { at, card: jace };
        let ui = s.ui_json();
        assert!(
            ui.contains("Meet the patron"),
            "a patron is met, not fought on sight"
        );
        assert!(!ui.contains("\"Fight\""));
        meet(&mut s, "Jace, Architect of Thought");
        let ui = s.ui_json();
        // Jace starts at four: +1, −2 and −8
        assert!(ui.contains("Patron · loyalty 4"));
        assert!(ui.contains("+1 loyalty") && ui.contains("−2 loyalty") && ui.contains("−8 loyalty"));
        let st = s.stories["44"].clone();
        assert!(s.can_choose(jace, &st.choices[1]), "four can pay two");
        assert!(!s.can_choose(jace, &st.choices[2]), "four cannot pay eight");
        assert!(s.act("story.choose", "2").is_err());
        s.act("story.choose", "0").unwrap();
        assert_eq!(s.player().loyalty.get("44"), Some(&5));
        assert!(result_lines(&s)
            .iter()
            .any(|l| l == "Jace, Architect of Thought's loyalty is now 5."));
        // the next meeting begins where the last left off
        meet(&mut s, "Jace, Architect of Thought");
        assert!(s.ui_json().contains("Patron · loyalty 5"));
        s.player.as_mut().unwrap().loyalty.insert("44".into(), 8);
        let had = s.player().collection.len();
        s.act("story.choose", "2").unwrap();
        assert_eq!(s.player().loyalty.get("44"), Some(&0));
        let won: Vec<usize> = s.player().collection[had..].to_vec();
        assert_eq!(won.len(), 2, "the ultimate gives two rares");
        for c in won {
            let k = &s.cards[c];
            assert!(matches!(k.rarity, 'r' | 'm'), "{}", k.name);
            assert_eq!(k.identity & !Guild::Izzet.colours(), 0, "{}", k.name);
        }
        // at nothing, only the +1 and the challenge are open
        meet(&mut s, "Jace, Architect of Thought");
        assert!(!s.can_choose(jace, &st.choices[1]));
        s.act("story.choose", "3").unwrap();
        let Screen::Result {
            next: Next::Fight { creature, .. },
            ..
        } = &s.screen
        else {
            panic!("the challenge is a fight")
        };
        assert_eq!(*creature, jace);
    }

    #[test]
    fn jaces_piles_put_the_most_valued_card_alone_face_up() {
        let mut s = started();
        quiet_tile(&mut s);
        let jace = meet(&mut s, "Jace, Architect of Thought");
        s.act("story.choose", "1").unwrap();
        assert_eq!(s.player().loyalty.get("44"), Some(&2), "four less two");
        s.act("result.next", "").unwrap();
        let Screen::Piles { card, piles } = s.screen.clone() else {
            panic!("no piles")
        };
        assert_eq!(card, jace);
        assert_eq!((piles[0].len(), piles[1].len()), (1, 2));
        let rank = |c: usize| match s.cards[c].rarity {
            'm' => 3,
            'r' => 2,
            'u' => 1,
            _ => 0,
        };
        let up = piles[0][0];
        for &c in &piles[1] {
            assert!((rank(up), s.cards[up].value) >= (rank(c), s.cards[c].value));
        }
        let ui = s.ui_json();
        assert!(ui.contains("Take the face-up pile") && ui.contains("2 cards, face down"));
        assert!(ui.contains(&s.cards[up].name));
        let had = s.player().collection.len();
        s.act("piles.pick", "1").unwrap();
        assert_eq!(s.player().collection[had..], piles[1][..]);
    }

    #[test]
    fn vraskas_contract_pays_when_its_creature_is_beaten_in_time_and_hurts_when_it_lapses() {
        let mut s = started();
        let at = quiet_tile(&mut s);
        // one tile four steps off has a pool: the contract can only name it
        let far = (at.0 + 4, at.1);
        let caracal = card(&s, "Trained Caracal");
        // and one two steps off, too near to be named
        let near = (at.0, at.1 + 2);
        for q in [far, near] {
            let t = s.world.as_mut().unwrap().tiles.get_mut(&q).unwrap();
            t.pool = vec![caracal];
            t.cleared = false;
        }
        assert_eq!(s.contract_tiles(), vec![far]);
        let vraska = meet(&mut s, "Vraska the Unseen");
        let steps = s.player().steps;
        s.act("story.choose", "0").unwrap();
        let c = s.player().contract.clone().expect("a contract");
        assert_eq!((c.at, c.creature, c.by), (far, caracal, steps + 15));
        assert_eq!(s.player().loyalty.get("208"), Some(&6));
        assert!(result_lines(&s)
            .iter()
            .any(|l| l == "The mark is Trained Caracal, 4 steps from here. Beat it within 15 steps."));
        assert!(s.quest_marks().contains(&(far, CONTRACT_MARK.to_string())));
        s.screen = Screen::Board;
        assert!(s.ui_json().contains("beat Trained Caracal: 15 steps left"));
        // a second contract waits for the first
        meet(&mut s, "Vraska the Unseen");
        assert!(!s.can_choose(vraska, &s.stories["208"].choices[0].clone()));
        // arriving there turns up the mark, whatever the pool holds
        s.world.as_mut().unwrap().tiles.get_mut(&far).unwrap().pool = vec![card(&s, "Cancel")];
        s.player.as_mut().unwrap().pos = far;
        s.arrive(far);
        assert!(matches!(s.screen, Screen::Draw { card, .. } if card == caracal));
        assert!(s.ui_json().contains("Vraska's mark: Trained Caracal."));
        let gold = s.player().gold;
        let golgari = s.player().standing.get("Golgari").copied().unwrap_or(0);
        s.act("draw.fight", "").unwrap();
        win_the_fight(&mut s);
        // tier one: three gold for the fight, fifteen for the contract
        assert_eq!(s.player().gold, gold + 3 + 15);
        assert_eq!(
            s.player().standing.get("Golgari").copied().unwrap_or(0),
            golgari + 1
        );
        assert!(s.player().contract.is_none());
        // a lapsed contract: three health and two loyalty
        s.player.as_mut().unwrap().contract = Some(player::Contract {
            at: far,
            creature: caracal,
            by: s.player().steps,
            from: "208".into(),
        });
        let health = s.player().health;
        step_to(&mut s, (far.0 - 1, far.1));
        assert!(s.player().contract.is_none());
        // the step fed you one first
        assert_eq!(
            s.player().health,
            (health + 1).min(s.player().max_health(&s.armoury)) - 3
        );
        assert_eq!(s.player().loyalty.get("208"), Some(&4));
        assert!(s.message.contains("The contract lapsed."));
    }

    #[test]
    fn vraskas_gaze_takes_the_strongest_creature_near_you_and_her_assassins_halve_a_foe() {
        let mut s = started();
        let at = quiet_tile(&mut s);
        let vraska = card(&s, "Vraska the Unseen");
        let st = s.stories["208"].clone();
        s.screen = Screen::Draw { at, card: vraska };
        s.act("draw.read", "").unwrap();
        assert!(
            !s.can_choose(vraska, &st.choices[1]),
            "no one in sight to turn to stone"
        );
        let caracal = card(&s, "Trained Caracal");
        let sweeper = card(&s, "Street Sweeper");
        let near = (at.0, at.1 + 2);
        let next = (at.0 + 1, at.1);
        for (q, pool) in [(near, vec![sweeper]), (next, vec![caracal])] {
            let t = s.world.as_mut().unwrap().tiles.get_mut(&q).unwrap();
            t.pool = pool;
            t.cleared = false;
        }
        // Street Sweeper is mana value six, the Caracal one
        assert_eq!(s.petrify_target(), Some((near, sweeper)));
        s.act("story.choose", "1").unwrap();
        assert!(s.player().collection.contains(&sweeper));
        assert!(s.world().tile(near).unwrap().cleared);
        assert!(!s.world().tile(next).unwrap().cleared);
        assert_eq!(s.player().loyalty.get("208"), Some(&2));
        // the ultimate: three foes at half life
        s.player.as_mut().unwrap().loyalty.insert("208".into(), 7);
        meet(&mut s, "Vraska the Unseen");
        s.act("story.choose", "2").unwrap();
        assert_eq!(s.player().assassins, 3);
        assert_eq!(s.player().loyalty.get("208"), Some(&0));
        s.screen = Screen::Draw {
            at: next,
            card: caracal,
        };
        s.act("draw.fight", "").unwrap();
        let Screen::Battle(f) = &s.screen else {
            panic!("no fight")
        };
        // a tier-one foe brings eight life; half is four
        assert_eq!(f.duel.s[1].life, 4);
        assert_eq!(s.player().assassins, 2);
    }

    #[test]
    fn an_instant_kept_for_later_answers_a_bad_draw_or_lets_you_slip_past() {
        let mut s = started();
        let at = quiet_tile(&mut s);
        let cancel = card(&s, "Cancel");
        let mortars = card(&s, "Mizzium Mortars");
        s.screen = Screen::Draw { at, card: mortars };
        assert!(
            !s.ui_json().contains("Keep it for later"),
            "a sorcery is read, not kept"
        );
        s.world.as_mut().unwrap().tiles.get_mut(&at).unwrap().cleared = false;
        s.screen = Screen::Draw { at, card: cancel };
        assert!(s.ui_json().contains("Keep it for later"));
        let had = s.player().collection.clone();
        s.act("draw.keep", "").unwrap();
        assert_eq!(s.player().held, Some(cancel));
        assert_eq!(s.player().collection, had, "a held instant is not a card won");
        assert!(s.world().tile(at).unwrap().cleared);
        s.screen = Screen::Board;
        assert!(s.ui_json().contains("\"Held\",\"Cancel\""));
        // a bad face waits for the answer
        let shredder = card(&s, "Codex Shredder");
        s.screen = Screen::Fate {
            at,
            card: shredder,
            choice: 0,
            faces: vec![Face::Failure, Face::Success, Face::Success, Face::HugeFailure],
            shuffled: true,
            picked: None,
        };
        s.act("fate.pick", "0").unwrap();
        assert!(matches!(s.screen, Screen::Fate { picked: Some(0), .. }));
        assert!(s.ui_json().contains("Cast Cancel"));
        s.act("fate.answer", "").unwrap();
        assert_eq!(
            result_lines(&s)[..3],
            [
                "You drew Failure.".to_string(),
                "You cast Cancel: the Failure turns to a Success.".to_string(),
                "It chews contentedly, and the archivist lets you take it home.".to_string(),
            ]
        );
        assert!(s.player().collection.contains(&shredder));
        assert_eq!(s.player().held, None);
        // with nothing held, a bad face settles at once
        s.screen = Screen::Fate {
            at,
            card: shredder,
            choice: 0,
            faces: vec![Face::Failure, Face::Success, Face::Success, Face::HugeFailure],
            shuffled: true,
            picked: None,
        };
        s.act("fate.pick", "0").unwrap();
        assert_eq!(result_lines(&s)[1], "It eats your map too.");
        // a creature can be slipped past, and stays on its tile
        let caracal = card(&s, "Trained Caracal");
        s.player.as_mut().unwrap().held = Some(cancel);
        let t = s.world.as_mut().unwrap().tiles.get_mut(&at).unwrap();
        t.cleared = false;
        t.pool = vec![caracal];
        s.screen = Screen::Draw { at, card: caracal };
        assert!(s.ui_json().contains("Cast Cancel and slip past"));
        s.act("draw.slip", "").unwrap();
        assert_eq!(
            result_lines(&s),
            vec!["You cast Cancel and slip past Trained Caracal. It is still here.".to_string()]
        );
        assert_eq!(s.player().held, None);
        assert!(!s.world().tile(at).unwrap().cleared);
        assert_eq!(s.player().pos, at);
    }

    #[test]
    fn a_held_instant_detains_rakdos_for_six_steps() {
        let mut s = riot();
        let cancel = card(&s, "Cancel");
        s.player.as_mut().unwrap().held = Some(cancel);
        s.screen = Screen::Rakdos;
        assert!(s.ui_json().contains("Cast Cancel: detain him for 6 steps"));
        s.act("riot.detain", "").unwrap();
        assert_eq!(s.player().held, None);
        let cards = s.cards.clone();
        let w = s.world.as_mut().unwrap();
        assert_eq!(w.riot.as_ref().unwrap().detained, 6);
        let at = w.riot.as_ref().unwrap().at;
        for _ in 0..6 {
            assert_eq!(w.riot_step(&cards), None);
        }
        assert_eq!(w.riot.as_ref().unwrap().at, at, "he waited six steps");
        // then his three steps a move again
        for _ in 0..2 {
            assert_eq!(w.riot_step(&cards), None);
        }
        assert!(w.riot_step(&cards).is_some());
        assert_eq!(w.riot.as_ref().unwrap().at, at + 1);
    }

    #[test]
    fn story_cards_leave_shrines_hexes_and_devices_that_act_as_you_step_on() {
        let mut s = started();
        let at = quiet_tile(&mut s);
        let off = (at.0 + 1, at.1);
        let bloom = card(&s, "Mana Bloom");
        s.world.as_mut().unwrap().tiles.get_mut(&at).unwrap().cleared = false;
        s.screen = Screen::Story { at, card: bloom };
        s.act("story.choose", "2").unwrap();
        let t = s.world().tile(at).unwrap().clone();
        assert_eq!(
            t.mark,
            Some(crate::world::Mark {
                kind: MarkKind::Shrine,
                left: 5
            })
        );
        assert!(t.cleared, "the bloom became the shrine");
        assert!(!s.player().collection.contains(&bloom));
        let max = s.player().max_health(&s.armoury);
        // at full health it has nothing to do, and keeps its use
        s.player.as_mut().unwrap().health = max;
        step_to(&mut s, off);
        s.message.clear();
        step_to(&mut s, at);
        assert_eq!(s.world().tile(at).unwrap().mark.map(|m| m.left), Some(5));
        assert!(!s.message.contains("shrine"), "{}", s.message);
        s.player.as_mut().unwrap().health = 1;
        step_to(&mut s, off);
        step_to(&mut s, at);
        // fed one a step, and the shrine's two
        assert!(max > 5);
        assert_eq!(s.player().health, 1 + 1 + 1 + 2);
        assert!(s.message.contains("The shrine heals you 2."));
        assert!(s
            .ui_json()
            .contains("A shrine: you heal 2 as you step here, 4 more times."));
        for _ in 0..4 {
            s.player.as_mut().unwrap().health = 1;
            step_to(&mut s, off);
            step_to(&mut s, at);
        }
        assert_eq!(s.world().tile(at).unwrap().mark, None, "five uses, then gone");
        // a hex hurts but leaves one health
        s.world.as_mut().unwrap().mark(off, MarkKind::Hex);
        s.player.as_mut().unwrap().health = 4;
        step_to(&mut s, off);
        assert_eq!(s.player().health, 4 + 1 - 2);
        step_to(&mut s, at);
        // one, fed to two: a hex of two would kill, and leaves one instead
        s.player.as_mut().unwrap().health = 1;
        step_to(&mut s, off);
        assert_eq!(s.player().health, 1);
        assert_eq!(s.player().pos, off, "no defeat sent you home");
        // a device pays three gold, three times
        s.world.as_mut().unwrap().mark(at, MarkKind::Device);
        let gold = s.player().gold;
        for _ in 0..4 {
            step_to(&mut s, at);
            step_to(&mut s, (at.0, at.1 + 1));
        }
        assert_eq!(s.player().gold, gold + 9);
        let tablet = card(&s, "Tablet of the Guilds");
        s.world.as_mut().unwrap().tiles.get_mut(&at).unwrap().cleared = false;
        s.player.as_mut().unwrap().pos = at;
        s.screen = Screen::Story { at, card: tablet };
        s.act("story.choose", "2").unwrap();
        assert_eq!(
            s.world().tile(at).unwrap().mark.map(|m| m.kind),
            Some(MarkKind::Device)
        );
    }

    #[test]
    fn a_sorcery_opens_the_way_through_peaks_and_canals_and_rebuilds_what_was_razed() {
        let mut s = started();
        let at = quiet_tile(&mut s);
        let (n, e, so) = ((at.0, at.1 - 1), (at.0 + 1, at.1), (at.0, at.1 + 1));
        {
            let w = s.world.as_mut().unwrap();
            w.tiles.get_mut(&n).unwrap().feature = Feature::Peak;
            w.tiles.get_mut(&e).unwrap().feature = Feature::Canal;
            let t = w.tiles.get_mut(&so).unwrap();
            t.razed = true;
            t.minion = Some(0);
            t.cleared = false;
        }
        assert!(s.go(n).is_err());
        let horizon = card(&s, "Seek the Horizon");
        s.screen = Screen::Story { at, card: horizon };
        s.act("story.choose", "0").unwrap();
        // the first in the order north, east, south, west
        assert_eq!(s.world().tile(n).unwrap().feature, Feature::Cards);
        assert_eq!(s.world().tile(e).unwrap().feature, Feature::Canal);
        assert!(result_lines(&s)
            .iter()
            .any(|l| l == "The way opens. Tiles next to you that can be walked now: 1."));
        // overloaded, every neighbour
        assert_eq!(s.world.as_mut().unwrap().open_around(at, 4), 1);
        assert_eq!(s.world().tile(e).unwrap().feature, Feature::Cards);
        step_to(&mut s, n);
        assert_eq!(s.player().pos, n);
        step_to(&mut s, at);
        let survey = card(&s, "Survey the Wreckage");
        s.screen = Screen::Story { at, card: survey };
        s.act("story.choose", "1").unwrap();
        let t = s.world().tile(so).unwrap();
        assert!(!t.razed && t.minion.is_none());
        assert!(result_lines(&s)
            .iter()
            .any(|l| l == "Razed tiles rebuilt near you: 1. Their minions are gone."));
    }
}
