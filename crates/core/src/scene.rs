//! What the WebGL page draws for each screen, as a draw list the core
//! builds (Builder D3). The board is the 2D grid made 3D: each tile a slab,
//! themed by its terrain (a little mountain on a Mountain, a canal through
//! an Island), its district's tower, a face-down card standing over a tile
//! not yet drawn, crystals for the lands you placed. A fight is an arena
//! with both sides' creatures; a city is a plaza of CHUMWORLD's buildings;
//! the armourer turns a piece on a stand.

use crate::duel::{Ev, PermSnap, Snap};
use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};
use crate::look::Pose;
use crate::mesh::Mesh;
use crate::render::{hex, Batch, Camera, Frame3, FrameOut, Mood, Pass, Rgb};
use crate::session::{Room, Screen, Session};
use crate::world::{distance, Feature, Guild, Pos, Terrain, World};

/// Metres a tile.
pub const TILE: Fx = 2 * ONE;
const SLAB: Fx = ONE * 2 / 5;
/// Board figures are drawn at this share of their size.
const BOARD_SCALE: Fx = ONE * 9 / 20;
const MODELS_PER_PAGE: usize = 12;

fn mood() -> Mood {
    Mood {
        fog: hex("#e9d6b8"),
        fog_near: 28 * ONE,
        fog_far: 80 * ONE,
        sky: hex("#fff4e0"),
        ground: hex("#8a7060"),
        sun: hex("#fff0d6"),
        sun_dir: v3(3 * ONE, 6 * ONE, 4 * ONE),
    }
}

fn dark_mood() -> Mood {
    Mood {
        fog: hex("#3a2c48"),
        fog_near: 22 * ONE,
        fog_far: 70 * ONE,
        sky: hex("#e8dcff"),
        ground: hex("#5a3a2a"),
        sun: hex("#ffd8a8"),
        sun_dir: v3(2 * ONE, 6 * ONE, 5 * ONE),
    }
}

pub fn tile_centre(p: Pos) -> V3 {
    v3(p.0 * TILE, 0, p.1 * TILE)
}

/// A point over a tile's centre, `up` metres high (for labels).
pub fn tile_point(p: Pos, up: i32) -> V3 {
    tile_centre(p).add(v3(0, up * ONE, 0))
}

fn terrain_top(t: Terrain) -> Rgb {
    hex(match t {
        Terrain::Plains => "#e8dfc6",
        Terrain::Island => "#9fb48a",
        Terrain::Swamp => "#5a5560",
        Terrain::Mountain => "#a88466",
        Terrain::Forest => "#7fb069",
    })
}

/// The ground of a tile wearing its terrain's second look (T24).
fn terrain_top_alternate(t: Terrain) -> Rgb {
    hex(match t {
        Terrain::Plains => "#dcd9b4",
        Terrain::Island => "#8fae96",
        Terrain::Swamp => "#4d5650",
        Terrain::Mountain => "#9a7462",
        Terrain::Forest => "#679e5a",
    })
}

fn guild_tint(g: Guild) -> Rgb {
    hex(match g {
        Guild::Azorius => "#d6e2f0",
        Guild::Izzet => "#c79a8a",
        Guild::Rakdos => "#7a4a4a",
        Guild::Golgari => "#6a7a4a",
        Guild::Selesnya => "#cfe0a0",
    })
}

fn side_of(c: Rgb) -> Rgb {
    [c[0] * 3 / 4, c[1] * 3 / 4, c[2] * 3 / 4]
}

fn upright(x: Fx, y: Fx, z: Fx) -> Frame3 {
    Frame3::upright(v3(x, y, z))
}

/// Draws a model, or a plain block in its place if the model is not made yet.
fn model(s: &Session, b: &mut Batch, name: &str, at: V3, yaw: i32, scale: Fx) {
    if s.look.has(name) {
        s.look.draw(
            b,
            name,
            at,
            &Pose {
                yaw,
                scale,
                ..Pose::default()
            },
        );
    } else {
        let k = fx::mul(ONE, scale);
        b.solid(
            Mesh::Cube,
            at.add(v3(0, k / 2, 0)),
            &upright(k / 2, k, k / 2),
            hex("#c0a080"),
        );
    }
}

/// A model scaled so its height is `height` metres.
fn model_h(s: &Session, b: &mut Batch, name: &str, at: V3, yaw: i32, height: Fx) {
    let h = s.look.height(name).max(ONE / 10);
    let k = fx::div(height, h);
    model(s, b, name, at, yaw, k);
}

fn crystal_name(unit: u8) -> Option<&'static str> {
    use crate::mana::*;
    Some(match unit {
        W => "crystal_W",
        U => "crystal_U",
        B => "crystal_B",
        R => "crystal_R",
        G => "crystal_G",
        _ => return None,
    })
}

/// The board's still parts: tiles, terrain, places, cards, lands.
fn board_statics(s: &Session, w: &World) -> Batch {
    let mut b = Batch::new();
    // the table the board lies on
    b.solid(
        Mesh::Cube,
        v3(0, -SLAB - ONE / 4, 0),
        &upright(60 * ONE, ONE / 2, 60 * ONE),
        hex("#5a3a2a"),
    );
    for (&p, t) in &w.tiles {
        let c = tile_centre(p);
        let alt = crate::world::alternate(w.seed, p);
        let top = match (t.guild, alt) {
            _ if t.razed => hex("#4a3a36"),
            (Some(g), _) => guild_tint(g),
            (None, true) => terrain_top_alternate(t.terrain),
            (None, false) => terrain_top(t.terrain),
        };
        let size = TILE * 97 / 100;
        b.solid(
            Mesh::Cube,
            c.add(v3(0, -SLAB / 2, 0)),
            &upright(size, SLAB, size),
            side_of(top),
        );
        b.solid(
            Mesh::Ground,
            c.add(v3(0, ONE / 200, 0)),
            &upright(size, ONE, size),
            top,
        );
        let seed = (p.0 * 7 + p.1 * 13).rem_euclid(4);
        match t.feature {
            Feature::City(_) => {
                for (k, (name, dx, dz)) in [
                    ("chum_hall", -1, -1),
                    ("chum_inn", 1, -1),
                    ("chum_house", -1, 1),
                    ("chum_mill", 1, 1),
                ]
                .iter()
                .enumerate()
                {
                    let at = c.add(v3(dx * ONE * 11 / 20, 0, dz * ONE * 11 / 20));
                    model_h(s, &mut b, name, at, fx::TURN * k as i32 / 4, ONE * 8 / 10);
                }
                model_h(s, &mut b, "chum_well", c, 0, ONE / 2);
            }
            Feature::Lair => {
                model_h(s, &mut b, "nivix", c, 0, ONE * 3);
            }
            Feature::Peak => model_h(s, &mut b, "peak", c, seed * fx::TURN / 4, ONE * 2),
            Feature::Place(place) => model_h(s, &mut b, place.model(), c, 0, ONE * 2),
            Feature::Cards if t.razed => {
                model(s, &mut b, "razed", c, seed * fx::TURN / 4, ONE * 9 / 10);
                if let Some(m) = t.minion {
                    let name = format!("c{}", s.cards[m].number);
                    model_h(s, &mut b, &name, c, fx::TURN / 2, ONE * 11 / 10);
                }
            }
            Feature::Canal => model(s, &mut b, "canal_pool", c, 0, ONE),
            Feature::Cards => {
                if let Some(g) = t.guild {
                    let name = match g {
                        Guild::Azorius => "tower_azorius",
                        Guild::Izzet => "tower_izzet",
                        Guild::Rakdos => "tower_rakdos",
                        Guild::Golgari => "tower_golgari",
                        Guild::Selesnya => "tower_selesnya",
                    };
                    model_h(s, &mut b, name, c.add(v3(-ONE / 2, 0, -ONE / 2)), 0, ONE * 3 / 2);
                }
                match t.terrain {
                    Terrain::Forest if alt => model_h(
                        s,
                        &mut b,
                        "forest_conifers",
                        c.add(v3(ONE / 4, 0, ONE / 4)),
                        seed * 300,
                        ONE * 13 / 10,
                    ),
                    Terrain::Mountain if alt => model_h(
                        s,
                        &mut b,
                        "mountain_crags",
                        c.add(v3(ONE / 4, 0, ONE / 5)),
                        seed * fx::TURN / 4,
                        ONE * 13 / 10,
                    ),
                    Terrain::Swamp if alt => model(
                        s,
                        &mut b,
                        "swamp_fungus",
                        c.add(v3(ONE / 5, 0, ONE / 4)),
                        seed * 400,
                        ONE * 7 / 10,
                    ),
                    Terrain::Plains if alt => model(
                        s,
                        &mut b,
                        "plains_arch",
                        c.add(v3(ONE / 5, 0, ONE / 3)),
                        seed * fx::TURN / 4,
                        ONE * 6 / 10,
                    ),
                    Terrain::Forest => {
                        model_h(
                            s,
                            &mut b,
                            "tree_a",
                            c.add(v3(ONE / 2, 0, ONE / 3)),
                            seed * 300,
                            ONE,
                        );
                        model_h(
                            s,
                            &mut b,
                            "tree_b",
                            c.add(v3(-ONE / 3, 0, ONE * 3 / 5)),
                            seed * 500,
                            ONE * 4 / 5,
                        );
                        if t.guild.is_none() {
                            model_h(
                                s,
                                &mut b,
                                "tree_a",
                                c.add(v3(-ONE / 2, 0, -ONE / 2)),
                                seed * 700,
                                ONE * 9 / 10,
                            );
                        }
                    }
                    Terrain::Mountain => model_h(
                        s,
                        &mut b,
                        "mountain",
                        c.add(v3(ONE / 4, 0, ONE / 5)),
                        seed * fx::TURN / 4,
                        ONE * 11 / 10,
                    ),
                    Terrain::Swamp => {
                        model(
                            s,
                            &mut b,
                            "swamp_pool",
                            c.add(v3(ONE / 4, 0, ONE / 4)),
                            seed * 400,
                            ONE * 6 / 10,
                        );
                        model_h(
                            s,
                            &mut b,
                            "dead_tree",
                            c.add(v3(-ONE / 2, 0, ONE / 3)),
                            seed * 900,
                            ONE * 4 / 5,
                        );
                    }
                    Terrain::Plains => model(
                        s,
                        &mut b,
                        "plains_columns",
                        c.add(v3(ONE / 5, 0, ONE / 3)),
                        seed * fx::TURN / 4,
                        ONE * 6 / 10,
                    ),
                    Terrain::Island => {
                        let ew = [(1, 0), (-1, 0)].iter().any(|d| {
                            w.tile((p.0 + d.0, p.1 + d.1))
                                .map(|n| n.terrain == Terrain::Island)
                                .unwrap_or(false)
                        });
                        let ns = [(0, 1), (0, -1)].iter().any(|d| {
                            w.tile((p.0 + d.0, p.1 + d.1))
                                .map(|n| n.terrain == Terrain::Island)
                                .unwrap_or(false)
                        });
                        let yaw = match (ew, ns) {
                            (true, _) => {
                                model(s, &mut b, "canal_straight", c, 0, ONE);
                                0
                            }
                            (false, true) => {
                                model(s, &mut b, "canal_straight", c, fx::TURN / 4, ONE);
                                fx::TURN / 4
                            }
                            _ => {
                                model(s, &mut b, "canal_pool", c, 0, ONE);
                                0
                            }
                        };
                        // the second look: a skiff moored on the water
                        if alt {
                            model(s, &mut b, "island_skiff", c, yaw, ONE * 8 / 10);
                        }
                    }
                }
                if !t.cleared && !t.visited && !t.pool.is_empty() {
                    model_h(
                        s,
                        &mut b,
                        "card_stand",
                        c.add(v3(ONE / 5, ONE / 4, -ONE / 5)),
                        (p.0 * 97 + p.1 * 61).rem_euclid(200) - 100,
                        ONE * 85 / 100,
                    );
                }
            }
        }
        // a visited tile carries a thin gold frame
        if t.visited {
            let gold = hex("#f2c14e");
            let e = TILE * 47 / 100;
            let th = ONE / 22;
            for (dx, dz, sx, sz) in [
                (0, -e, 2 * e, th),
                (0, e, 2 * e, th),
                (-e, 0, th, 2 * e),
                (e, 0, th, 2 * e),
            ] {
                b.solid(
                    Mesh::Cube,
                    c.add(v3(dx, ONE / 60, dz)),
                    &upright(sx, ONE / 40, sz),
                    gold,
                );
            }
        }
        // lands: a crystal of their colour on a corner, a gate for a guildgate
        for (k, &l) in t.lands.iter().enumerate() {
            let unit = crate::rules::land_unit(&s.cards[l].name).unwrap_or(0);
            let corner = [
                v3(ONE * 7 / 10, 0, ONE * 7 / 10),
                v3(-ONE * 7 / 10, 0, ONE * 7 / 10),
                v3(ONE * 7 / 10, 0, -ONE * 7 / 10),
            ][k % 3];
            match crystal_name(unit) {
                Some(n) => model_h(s, &mut b, n, c.add(corner), 0, ONE / 2),
                None => model_h(s, &mut b, "gate_marker", c.add(corner), 0, ONE / 2),
            }
        }
    }
    if s.player.is_some() {
        quest_marks(s, w, &mut b);
    }
    b
}

/// Each quest's next place (T33): a grey tile where it is not found yet and
/// a banner over it; Rakdos where he stands (T32).
fn quest_marks(s: &Session, w: &World, b: &mut Batch) {
    let grey = hex("#a8a29a");
    for (p, quest) in s.quest_marks() {
        let c = tile_centre(p);
        if !w.tiles.contains_key(&p) {
            let size = TILE * 97 / 100;
            b.solid(
                Mesh::Cube,
                c.add(v3(0, -SLAB / 2, 0)),
                &upright(size, SLAB, size),
                side_of(grey),
            );
            b.solid(
                Mesh::Ground,
                c.add(v3(0, ONE / 200, 0)),
                &upright(size, ONE, size),
                grey,
            );
        }
        if quest == crate::session::RAKDOS_MARK {
            model_h(s, b, "c187", c, fx::TURN / 2, ONE * 16 / 10);
        } else {
            model_h(
                s,
                b,
                "quest_banner",
                c.add(v3(-ONE * 6 / 10, 0, -ONE * 6 / 10)),
                0,
                ONE * 13 / 10,
            );
        }
    }
}

fn board_camera(s: &Session, focus: V3) -> Camera {
    let _ = s;
    pitched(focus, fx::TURN * 52 / 360, 17 * ONE, ONE * 29 / 10)
}

/// A camera above `at`, pitched down by `pitch` turns, `dist` away, looking north.
fn pitched(at: V3, pitch: i32, dist: Fx, focal: Fx) -> Camera {
    let up = fx::mul(dist, fx::sin(pitch));
    let back = fx::mul(dist, fx::cos(pitch));
    Camera {
        eye: at.add(v3(0, up, back)),
        at,
        focal,
    }
}

/// Where the player stands this frame: between tiles during a step, with a hop.
fn hero_spot(s: &Session) -> V3 {
    let p = s.player().pos;
    match s.walk {
        Some((from, to, t0)) => {
            let t = ((s.now - t0).max(0) * ONE as i64 / crate::session::STEP_US) as Fx;
            let t = t.clamp(0, ONE);
            let a = tile_centre(from);
            let b = tile_centre(to);
            let hop = fx::mul(fx::sin(t / 2), ONE / 2);
            a.lerp(b, t).add(v3(0, hop, 0))
        }
        None => tile_centre(p),
    }
}

pub fn camera_for(s: &Session) -> Camera {
    match &s.screen {
        Screen::Title | Screen::Mission | Screen::NewGame | Screen::Starter { .. } => {
            let t = (s.now / 1000 % 60_000) as i32;
            let yaw = (t as i64 * fx::TURN as i64 / 60_000) as i32;
            let r = 22 * ONE;
            let at = v3(0, 0, 0);
            Camera {
                eye: v3(fx::mul(r, fx::sin(yaw)), 15 * ONE, fx::mul(r, fx::cos(yaw))),
                at,
                focal: ONE * 26 / 10,
            }
        }
        Screen::Battle(_) => {
            // the table turns about its middle; home is behind the player's side
            let home = pitched(
                v3(0, ONE / 2, ONE * 2),
                fx::TURN * 50 / 360,
                15 * ONE,
                ONE * 26 / 10,
            );
            Camera {
                eye: home.eye.yaw(s.view_yaw),
                at: home.at.yaw(s.view_yaw),
                focal: home.focal,
            }
        }
        Screen::City {
            room: Room::Armourer, ..
        } => Camera {
            eye: v3(0, ONE * 22 / 10, ONE * 52 / 10),
            at: v3(0, ONE, 0),
            focal: ONE * 30 / 10,
        },
        Screen::City { .. } => pitched(v3(0, 0, ONE / 2), fx::TURN * 40 / 360, 17 * ONE, ONE * 25 / 10),
        Screen::Deck { .. } => Camera {
            eye: v3(0, 4 * ONE, 3 * ONE),
            at: v3(0, 0, 0),
            focal: ONE * 3,
        },
        Screen::Models { .. } => pitched(v3(0, ONE, 0), fx::TURN * 30 / 360, 26 * ONE, ONE * 26 / 10),
        _ if s.world.is_some() => board_camera(s, hero_spot(s)),
        _ => pitched(V3::ZERO, fx::TURN / 8, 16 * ONE, ONE * 26 / 10),
    }
}

// ---------- the battle ----------

/// Where each permanent stands in the arena, for drawing and labels.
/// Where `n` things stand along a row `width` wide, `most` apart at most:
/// the x of the `i`th, the row centred.
fn row_x(i: usize, n: usize, width: Fx, most: Fx) -> Fx {
    if n <= 1 {
        return 0;
    }
    let gap = (width / (n as i32 - 1)).min(most);
    (i as i32 * 2 - (n as i32 - 1)) * gap / 2
}

/// The most creatures in one row of a side before a second row starts.
const ROW_MOST: usize = 5;

pub fn battle_spots<'a>(s: &Session, snap: &'a Snap) -> Vec<(V3, &'a PermSnap)> {
    let _ = s;
    let mut out = Vec::new();
    for side in 0..2u8 {
        let mine: Vec<&PermSnap> = snap
            .perms
            .iter()
            .filter(|p| p.side == side && p.attached < 0)
            .collect();
        let attackers: Vec<&&PermSnap> = mine.iter().filter(|p| p.creature && p.attacking).collect();
        let home: Vec<&&PermSnap> = mine.iter().filter(|p| p.creature && !p.attacking).collect();
        let others: Vec<&&PermSnap> = mine.iter().filter(|p| !p.creature && !p.land).collect();
        let lands: Vec<&&PermSnap> = mine.iter().filter(|p| p.land).collect();
        let dir = if side == 0 { 1 } else { -1 };
        // lands: columns in the side's left corner, as a player keeps them;
        // a third column when there are more than eight
        let cols = if lands.len() > 8 { 3 } else { 2 };
        for (i, p) in lands.iter().enumerate() {
            let col = (i % cols) as i32;
            let row = (i / cols) as i32;
            let x = -dir * (ONE * 53 / 10 - col * ONE);
            let z = dir * (ONE * 4 / 10 + row * ONE);
            out.push((v3(x, 0, z), **p));
        }
        // enchantments, artifacts and planeswalkers: the same in the right
        // corner, a column more each time it fills
        let cols = match others.len() {
            0..=8 => 2,
            9..=15 => 3,
            _ => 4,
        };
        for (i, p) in others.iter().enumerate() {
            let col = (i % cols) as i32;
            let row = (i / cols) as i32;
            let x = dir * (ONE * 52 / 10 - col * ONE * 9 / 10);
            let z = dir * (ONE * 6 / 10 + row * ONE * 9 / 10);
            out.push((v3(x, 0, z), **p));
        }
        // attackers in one row toward the middle, closer together when many
        for (i, p) in attackers.iter().enumerate() {
            let x = row_x(i, attackers.len(), ONE * 8, ONE * 22 / 10);
            out.push((v3(x, 0, dir * ONE * 3 / 5), **p));
        }
        // the rest in rows of five, the second row behind the first
        let rows = home.len().div_ceil(ROW_MOST).max(1);
        for (i, p) in home.iter().enumerate() {
            let row = i / ROW_MOST;
            let in_row = if row + 1 == rows {
                home.len() - row * ROW_MOST
            } else {
                ROW_MOST
            };
            let x = row_x(i % ROW_MOST, in_row, ONE * 8, ONE * 22 / 10);
            let z = dir * (ONE * 17 / 10 + row as i32 * ONE * 13 / 10);
            out.push((v3(x, 0, z), **p));
        }
    }
    // a blocker stands in front of what it blocks; several side by side
    let spots: Vec<(V3, &PermSnap)> = out.clone();
    for (pt, p) in out.iter_mut() {
        if p.blocking >= 0 {
            if let Some((apt, _)) = spots.iter().find(|(_, a)| a.id as i32 == p.blocking) {
                let mates: Vec<u32> = spots
                    .iter()
                    .filter(|(_, q)| q.blocking == p.blocking)
                    .map(|(_, q)| q.id)
                    .collect();
                let k = mates.iter().position(|&id| id == p.id).unwrap_or(0);
                let x = apt.x + row_x(k, mates.len(), ONE * 2, ONE);
                *pt = v3(x, 0, if p.side == 0 { ONE * 3 / 10 } else { -ONE * 3 / 10 });
            }
        }
    }
    out
}

/// The model a permanent is drawn with: its card's ("c" and its collector
/// number), or its token's.
fn model_of(s: &Session, p: &PermSnap) -> String {
    if p.card >= 0 {
        format!("c{}", s.cards[p.card as usize].number)
    } else {
        p.token.clone()
    }
}

/// How tall a creature stands on the table: by its size, the guardian tallest.
fn creature_height(p: &PermSnap) -> Fx {
    if p.name == "Niv-Mizzet, Dracogenius" {
        ONE * 45 / 10
    } else {
        battle_height(p.power, p.toughness)
    }
}

/// Auras and equipment, drawn on what they are attached to: an Aura's sigil
/// floating over its host with a ring of its colour at the host's feet;
/// equipment at the host's side.
fn attachments(s: &Session, b: &mut Batch, snap: &Snap, spots: &[(V3, &PermSnap)], phase: i32) {
    for a in snap.perms.iter().filter(|p| p.attached >= 0) {
        let Some((at, host)) = spots.iter().find(|(_, h)| h.id as i32 == a.attached) else {
            continue;
        };
        let name = model_of(s, a);
        if !s.look.has(&name) {
            continue;
        }
        let card = (a.card >= 0).then(|| &s.cards[a.card as usize]);
        let equipment = card.map(|c| c.rules.equip.is_some()).unwrap_or(false);
        let tall = if host.creature {
            creature_height(host)
        } else {
            ONE * 7 / 10
        };
        let h = s.look.height(&name).max(ONE / 10);
        if equipment {
            let k = fx::div(ONE * 8 / 10, h);
            s.look.draw(
                b,
                &name,
                at.add(v3(ONE * 6 / 10, 0, ONE / 5)),
                &Pose {
                    scale: k,
                    ..Pose::default()
                },
            );
            continue;
        }
        let bob = fx::mul(fx::sin(phase + a.id as i32 * 900), ONE / 12);
        let k = fx::div(ONE * 45 / 100, h);
        let spin = (s.now / 1000 % 6000) as i32 * fx::TURN / 6000;
        s.look.draw(
            b,
            &name,
            at.add(v3(0, tall + ONE / 4 + bob, 0)),
            &Pose {
                yaw: spin,
                scale: k,
                ..Pose::default()
            },
        );
        let rgb = colour_glow(card.map(|c| c.colours).unwrap_or(0));
        for i in 0..12 {
            let ang = fx::TURN * i / 12 + spin;
            let p = at.add(v3(
                fx::mul(fx::cos(ang), ONE * 7 / 10),
                ONE / 10,
                fx::mul(fx::sin(ang), ONE * 7 / 10),
            ));
            glow(b, p, ONE / 8, rgb, ONE * 3 / 4);
        }
    }
}

fn battle_height(power: i32, toughness: i32) -> Fx {
    (ONE + (power + toughness).max(0) * ONE / 5).min(ONE * 32 / 10)
}

fn draw_battle(s: &Session, b: &mut Batch, f: &crate::session::Fight) {
    if s.look.has("arena") {
        model(s, b, "arena", V3::ZERO, 0, ONE);
    } else {
        b.solid(
            Mesh::Cube,
            v3(0, -ONE / 4, 0),
            &upright(12 * ONE, ONE / 2, 9 * ONE),
            hex("#e8dfc6"),
        );
    }
    b.solid(
        Mesh::Cube,
        v3(0, -ONE, 0),
        &upright(60 * ONE, ONE / 2, 60 * ONE),
        hex("#5a3a2a"),
    );
    let ev = f.event();
    let phase = (s.now / 1000 % 4000) as i32 * fx::TURN / 4000;
    // how far through the event on screen we are, Q16
    let span = (crate::session::EVENT_US / f.speed.max(1)).max(1);
    let u = (((s.now - f.shown_at).max(0) * ONE as i64 / span) as Fx).clamp(0, ONE);
    let spots = battle_spots(s, &ev.snap);
    let in_combat = |id: u32| {
        ev.snap
            .perms
            .iter()
            .any(|q| q.id == id && (q.attacking || q.blocking >= 0))
    };
    let swinger = match ev.ev {
        Ev::Hit | Ev::Face => ev.focus.first().copied().filter(|&id| in_combat(id)),
        _ => None,
    };
    for (pt, p) in spots.iter().map(|(a, b)| (*a, *b)) {
        let name = model_of(s, p);
        let striking = swinger == Some(p.id);
        // the blow lands at the hit, 65 % through the swing
        let landed = u >= ONE * 13 / 20;
        let struck =
            matches!(ev.ev, Ev::Hit) && ev.focus.len() == 2 && ev.focus.last() == Some(&p.id) && landed;
        let mut yaw = if p.side == 0 { fx::TURN / 2 } else { 0 };
        if striking {
            // a creature striking turns to what it strikes
            let target = match ev.ev {
                Ev::Hit => ev
                    .focus
                    .last()
                    .and_then(|t| spots.iter().find(|(_, q)| q.id == *t))
                    .map(|(a, _)| *a),
                _ => Some(side_point(if p.side == 0 { 1 } else { 0 })),
            };
            if let Some(t) = target {
                yaw = crate::geom::yaw_of(t.x - pt.x, t.z - pt.z);
            }
        }
        if p.land {
            // a land: a crystal of its colour, sunk when tapped
            let unit = if p.card >= 0 {
                crate::rules::land_unit(&s.cards[p.card as usize].name).unwrap_or(0)
            } else {
                0
            };
            let h = if p.tapped { ONE * 3 / 4 } else { ONE * 16 / 10 };
            match crystal_name(unit) {
                Some(n) => model_h(s, b, n, pt, 0, h),
                None => model_h(s, b, "gate_marker", pt, 0, h),
            }
            continue;
        }
        if !p.creature {
            // an enchantment as its sigil, an artifact as the object, turning slowly
            let spin = (s.now / 1000 % 9000) as i32 * fx::TURN / 9000;
            if s.look.has(&name) {
                let h = s.look.height(&name).max(ONE / 10);
                let k = fx::div(ONE * 9 / 10, h);
                s.look.draw(
                    b,
                    &name,
                    pt,
                    &Pose {
                        yaw: spin,
                        scale: k,
                        phase: phase + p.id as i32 * 1700,
                        tired: p.tapped,
                        dim: p.detained,
                        ..Pose::default()
                    },
                );
            } else {
                b.solid(
                    Mesh::Gem,
                    pt.add(v3(0, ONE / 2, 0)),
                    &upright(ONE / 2, ONE * 3 / 4, ONE / 2),
                    hex("#b9a6e8"),
                );
            }
            continue;
        }
        let height = creature_height(p);
        let mh = s.look.height(&name).max(ONE / 10);
        let k = fx::div(height, mh);
        let (swing, recover) = if striking {
            if landed {
                (0, fx::div(u - ONE * 13 / 20, ONE * 7 / 20).max(1))
            } else {
                (fx::div(u, ONE * 13 / 20).max(1), 0)
            }
        } else {
            (0, 0)
        };
        // T35, after MTG Arena: a creature that dies sinks and shrinks away;
        // one that enters drops onto the table
        let (k, pt) = match ev.ev {
            Ev::Dies if ev.focus.first() == Some(&p.id) => (
                fx::mul(k, ONE - fx::mul(u, DIE_SHRINK)),
                pt.sub(v3(0, fx::mul(u, height / 2), 0)),
            ),
            Ev::Enter | Ev::Token if ev.focus.first() == Some(&p.id) => (k, pt.add(v3(0, enter_drop(u), 0))),
            _ => (k, pt),
        };
        let pose = Pose {
            yaw,
            scale: k,
            phase: phase + p.id as i32 * 1700,
            lunge: 0,
            flash: struck,
            tired: p.tapped && !striking,
            swing,
            recover,
            dim: p.detained,
        };
        if s.look.has(&name) {
            s.look.draw(b, &name, pt, &pose);
        } else {
            b.solid(
                Mesh::Blob,
                pt.add(v3(0, height / 2, 0)),
                &upright(ONE, height, ONE),
                hex("#c09070"),
            );
        }
        if p.detained {
            b.push(
                Mesh::Ground,
                Pass::Glow,
                pt.add(v3(0, ONE / 50, 0)),
                &upright(ONE * 2, ONE, ONE * 2),
                hex("#7fb6ff"),
                ONE / 2,
            );
        }
    }
    attachments(s, b, &ev.snap, &spots, phase);
    effects(s, b, ev, &spots, swinger, u);
    // T35: arcs from the top of the stack to what it aims at, a bright bead
    // running along each
    if let Some((_, _, card)) = ev.snap.stack.last() {
        let rgb = colour_glow(if *card >= 0 {
            s.cards[*card as usize].colours
        } else {
            0
        });
        for (from, to) in aim_arcs(&ev.snap, &spots) {
            arc(b, from, to, phase, rgb);
        }
    }
    // the player stands behind their side
    let k = fx::div(ONE * 16 / 10, s.look.height("hero").max(ONE / 10));
    draw_hero(
        s,
        b,
        v3(-ONE * 66 / 10, 0, ONE * 36 / 10),
        &Pose {
            yaw: fx::TURN / 2 + fx::TURN / 10,
            scale: k,
            phase,
            ..Pose::default()
        },
    );
}

/// Where each worn piece sits on the mage, in the mage's own space: its
/// scale, its offset, and which half of a pair (gloves and boots are drawn
/// one hand and one foot at a time).
fn armour_fits(slot: &str) -> Vec<crate::look::Fit> {
    use crate::look::Fit;
    let cm = |v: i32| v * ONE / 100;
    let fit = |scale: Fx, x: i32, y: i32, z: i32, side: i32| Fit {
        scale,
        offset: v3(cm(x), cm(y), cm(z)),
        side,
        hide: [None; 6],
    };
    match slot {
        "head" => vec![fit(ONE * 2, 0, 74, 0, 0)],
        "chest" => vec![fit(ONE * 3 / 2, 0, -5, 5, 0)],
        "hands" => vec![fit(ONE * 7 / 5, -32, 23, 14, -1), fit(ONE * 7 / 5, 32, 23, 14, 1)],
        "feet" => vec![fit(ONE * 13 / 10, -4, 0, -1, -1), fit(ONE * 13 / 10, 4, 0, -1, 1)],
        _ => Vec::new(),
    }
}

/// The mage, wearing what the player wears: each worn piece drawn on its
/// place, and the mage's own cap, robe, hands or feet left out beneath it.
fn draw_hero(s: &Session, b: &mut Batch, at: V3, pose: &Pose) {
    use crate::look::Fit;
    let worn = s.player.as_ref().map(|p| p.worn.clone()).unwrap_or_default();
    let cm = |v: i32| v * ONE / 100;
    let robe = s.look.colour("U");
    let trim = s.look.colour("gold");
    let skin = s.look.colour("skin");
    let boots = hex("#5a3a2a");
    // what each slot covers: a colour of the mage's and the heights it spans
    let mut hide = [None; 6];
    let mut n = 0;
    for (slot, band) in [
        ("head", (robe, cm(100), cm(200))),
        ("head", (trim, cm(130), cm(200))),
        ("chest", (robe, 0, cm(100))),
        ("chest", (trim, cm(10), cm(40))),
        ("hands", (skin, cm(30), cm(70))),
        ("feet", (boots, 0, cm(30))),
    ] {
        if worn.contains_key(slot) {
            hide[n] = Some(band);
            n += 1;
        }
    }
    let body = Fit { hide, ..Fit::WHOLE };
    s.look.draw_fitted(b, "hero", at, pose, &body);
    for (slot, id) in &worn {
        let model = format!("armour_{id}");
        for f in armour_fits(slot) {
            s.look.draw_fitted(b, &model, at, pose, &f);
        }
    }
}

/// Where a side's spells come from and its life is struck: the player
/// behind their side, the foe at the far end.
pub fn side_point(side: u8) -> V3 {
    if side == 0 {
        v3(-ONE * 66 / 10, ONE * 12 / 10, ONE * 36 / 10)
    } else {
        v3(0, ONE * 12 / 10, -ONE * 42 / 10)
    }
}

/// How much of its size a dying creature loses as it sinks (T35).
const DIE_SHRINK: Fx = ONE * 4 / 5;
/// How high above the table an entering creature starts (T35).
const ENTER_FROM: Fx = ONE * 3;
/// The share of the event an entering creature takes to land.
const ENTER_LAND: Fx = ONE * 2 / 5;

/// How far above its place an entering creature is at `u` through its
/// event: it falls faster as it comes, and stays down once landed.
pub fn enter_drop(u: Fx) -> Fx {
    if u >= ENTER_LAND {
        return 0;
    }
    let left = ONE - fx::div(u, ENTER_LAND);
    fx::mul(ENTER_FROM, fx::mul(left, left))
}

/// Each arc the table draws for the top of the stack (T35): from the
/// permanent whose ability it is, or else its controller's side, to each
/// permanent and player it aims at.
pub fn aim_arcs(snap: &Snap, spots: &[(V3, &PermSnap)]) -> Vec<(V3, V3)> {
    let (Some((side, _, _)), Some(aims)) = (snap.stack.last(), snap.aims.last()) else {
        return Vec::new();
    };
    let at = |id: u32| {
        spots
            .iter()
            .find(|(_, q)| q.id == id)
            .map(|(a, _)| a.add(v3(0, ONE, 0)))
    };
    let from = aims.from.and_then(at).unwrap_or(side_point(*side));
    aims.perms
        .iter()
        .filter_map(|&id| at(id))
        .chain(aims.players.iter().map(|&p| side_point(p)))
        .map(|to| (from, to))
        .collect()
}

/// An arc of small glows, high in the middle, with one bright bead running
/// from its start to its end over and over.
fn arc(b: &mut Batch, from: V3, to: V3, phase: i32, rgb: Rgb) {
    const BEADS: i32 = 16;
    let at = |t: Fx| {
        let lift = fx::mul(fx::mul(t, ONE - t), ONE * 3);
        from.lerp(to, t).add(v3(0, lift, 0))
    };
    for i in 1..BEADS {
        glow(b, at(ONE * i / BEADS), ONE / 9, rgb, ONE / 2);
    }
    let run = (phase * 2).rem_euclid(fx::TURN) * ONE / fx::TURN;
    glow(b, at(run), ONE * 3 / 10, [ONE, ONE, ONE], ONE * 3 / 4);
    glow(b, at(run), ONE * 2 / 5, rgb, ONE * 3 / 4);
    ring(b, to.sub(v3(0, ONE, 0)), ONE, ONE / 3, rgb);
}

/// A Magic colour's glow.
fn colour_glow(mask: u8) -> Rgb {
    use crate::mana::*;
    match mask {
        W => hex("#fff1b8"),
        U => hex("#6fb0ff"),
        B => hex("#a07ce0"),
        R => hex("#ff7a3c"),
        G => hex("#7fd36a"),
        0 => hex("#e8e0ff"),
        _ => hex("#ffd24a"),
    }
}

fn glow(b: &mut Batch, at: V3, r: Fx, rgb: Rgb, alpha: Fx) {
    if r <= 0 || alpha <= 0 {
        return;
    }
    b.push(Mesh::Halo, Pass::Glow, at, &upright(r, r, r), rgb, alpha);
}

/// Protection of the Spheres' "ring" (`marks.rs` there): a ring opening on
/// the ground and fading, drawn as a circle of glows.
fn ring(b: &mut Batch, at: V3, size: Fx, u: Fx, rgb: Rgb) {
    let open = ONE - fx::mul(ONE - u, ONE - u);
    let r = fx::mul(size, ONE / 5 + fx::mul(open, ONE * 4 / 5));
    let fade = ONE - u;
    for i in 0..14 {
        let a = fx::TURN * i / 14;
        let p = at.add(v3(fx::mul(fx::cos(a), r), ONE / 10, fx::mul(fx::sin(a), r)));
        glow(b, p, ONE / 6, rgb, fade);
    }
}

/// Protection of the Spheres' "rise": glows climbing in a slow spiral, for
/// a spell cast, a boost or life gained.
fn rise(b: &mut Batch, at: V3, size: Fx, u: Fx, rgb: Rgb) {
    let fade = ONE - u;
    for i in 0..5 {
        let a = fx::TURN * i / 5 + fx::mul(u, fx::TURN / 3);
        let r = size / 2;
        let p = at.add(v3(
            fx::mul(fx::cos(a), r),
            fx::mul(u, size * 2) + ONE / 3,
            fx::mul(fx::sin(a), r),
        ));
        glow(b, p, ONE / 5, rgb, fade);
    }
}

/// Protection of the Spheres' "bolt": a bright head with a fading trail,
/// on a shallow arc, from `from` to `to` over `u` (0..ONE).
fn bolt(b: &mut Batch, from: V3, to: V3, u: Fx, rgb: Rgb) {
    let head = |t: Fx| {
        let lift = fx::mul(fx::mul(t, ONE - t), ONE * 2);
        from.lerp(to, t).add(v3(0, lift, 0))
    };
    glow(b, head(u), ONE * 2 / 5, [ONE, ONE, ONE], ONE);
    glow(b, head(u), ONE * 3 / 5, rgb, ONE);
    for i in 1..=5 {
        let t = u - i * ONE / 40;
        if t <= 0 {
            break;
        }
        glow(
            b,
            head(t),
            ONE * 2 / 5 - i * ONE / 16,
            rgb,
            ONE / 2 - i * ONE / 12,
        );
    }
}

/// Protection of the Spheres' "sky": a beam from above onto its target, and
/// a ring at its feet (detain).
fn sky(b: &mut Batch, at: V3, u: Fx, rgb: Rgb) {
    let fade = ONE - u;
    for i in 0..10 {
        glow(b, at.add(v3(0, ONE * i * 7 / 10, 0)), ONE / 4, rgb, fade / 2);
    }
    ring(b, at, ONE, u, rgb);
}

/// The look of the event on screen: spells and abilities as Protection of
/// the Spheres draws its abilities.
fn effects(
    s: &Session,
    b: &mut Batch,
    ev: &crate::duel::Event,
    spots: &[(V3, &PermSnap)],
    swinger: Option<u32>,
    u: Fx,
) {
    let at_perm = |id: u32| {
        spots
            .iter()
            .find(|(_, q)| q.id == id)
            .map(|(a, _)| a.add(v3(0, ONE, 0)))
    };
    let arg = |k: &str| {
        ev.args
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    let card_colour = |name: &str| {
        s.cards
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.colours)
            .unwrap_or(0)
    };
    let hit_at = ONE * 13 / 20;
    match ev.ev {
        Ev::Hit | Ev::Face if swinger.is_none() => {
            let to = match ev.ev {
                Ev::Hit => ev.focus.last().and_then(|&t| at_perm(t)),
                _ => Some(side_point(ev.side)),
            };
            let source_perm = match ev.ev {
                Ev::Hit if ev.focus.len() == 2 => ev.focus.first().and_then(|&t| at_perm(t)),
                Ev::Face => ev.focus.first().and_then(|&t| at_perm(t)),
                _ => None,
            };
            // a spell comes from its caster: the other side from the one it hits
            let caster = if ev.ev == Ev::Face {
                1 - ev.side.min(1)
            } else {
                ev.side
            };
            let from = source_perm.unwrap_or(side_point(caster));
            let rgb = colour_glow(card_colour(&arg("source")));
            let Some(to) = to else { return };
            if u < hit_at {
                bolt(b, from, to, fx::div(u, hit_at), rgb);
            } else {
                let v = fx::div(u - hit_at, ONE - hit_at);
                glow(b, to, ONE / 2 + fx::mul(v, ONE), rgb, ONE - v);
                ring(b, to.sub(v3(0, ONE, 0)), ONE * 3 / 2, v, rgb);
            }
        }
        Ev::Cast => {
            let rgb = colour_glow(card_colour(&arg("card")));
            rise(b, side_point(ev.side).sub(v3(0, ONE, 0)), ONE, u, rgb);
        }
        Ev::Ability => {
            if let Some(at) = ev.focus.last().and_then(|&t| at_perm(t)) {
                rise(b, at.sub(v3(0, ONE, 0)), ONE, u, hex("#ffe08a"));
            }
        }
        Ev::Detain => {
            if let Some(at) = ev.focus.first().and_then(|&t| at_perm(t)) {
                sky(b, at.sub(v3(0, ONE, 0)), u, hex("#7fb6ff"));
            }
        }
        Ev::Life => rise(b, side_point(ev.side).sub(v3(0, ONE, 0)), ONE, u, hex("#9ff0a0")),
        Ev::Token | Ev::Enter => {
            if let Some(at) = ev.focus.first().and_then(|&t| at_perm(t)) {
                ring(b, at.sub(v3(0, ONE, 0)), ONE * 3 / 2, u, hex("#fff6d8"));
            }
        }
        Ev::Dies => {
            // motes rising and spreading as it goes (T35)
            if let Some(at) = ev.focus.first().and_then(|&t| at_perm(t)) {
                for i in 0..8 {
                    let a = fx::TURN * i / 8 + fx::mul(u, fx::TURN / 4);
                    let r = ONE / 3 + fx::mul(u, ONE);
                    let p = at.add(v3(
                        fx::mul(fx::cos(a), r),
                        fx::mul(u, ONE * 2),
                        fx::mul(fx::sin(a), r),
                    ));
                    glow(b, p, ONE / 5, hex("#d8d0e8"), ONE - u);
                }
            }
        }
        Ev::Counter => glow(
            b,
            side_point(ev.side),
            ONE + fx::mul(u, ONE),
            hex("#c9a0ff"),
            ONE - u,
        ),
        _ => {}
    }
}

// ---------- the city ----------

/// The five doors: where each room's building stands.
pub fn city_doors(s: &Session) -> Vec<(V3, &'static str)> {
    let _ = s;
    vec![
        (v3(-5 * ONE, 3 * ONE, -3 * ONE), "armourer"),
        (v3(0, 3 * ONE, -4 * ONE), "cards"),
        (v3(5 * ONE, 4 * ONE, -3 * ONE), "hall"),
        (v3(-5 * ONE, 3 * ONE, 2 * ONE), "inn"),
        (v3(5 * ONE, 4 * ONE, 2 * ONE), "trainer"),
    ]
}

fn draw_city(s: &Session, b: &mut Batch) {
    if s.look.has("plaza") {
        model(s, b, "plaza", V3::ZERO, 0, ONE * 2);
    } else {
        b.solid(
            Mesh::Cube,
            v3(0, -ONE / 4, 0),
            &upright(16 * ONE, ONE / 2, 11 * ONE),
            hex("#d8cbb0"),
        );
    }
    b.solid(
        Mesh::Cube,
        v3(0, -ONE, 0),
        &upright(60 * ONE, ONE / 2, 60 * ONE),
        hex("#5a3a2a"),
    );
    let place = [
        ("chum_house", v3(-5 * ONE, 0, -3 * ONE), 0, ONE * 26 / 10),
        ("chum_stall", v3(0, 0, -4 * ONE), 0, ONE * 2),
        ("chum_hall", v3(5 * ONE, 0, -3 * ONE), 0, ONE * 34 / 10),
        ("chum_inn", v3(-5 * ONE, 0, 2 * ONE), fx::TURN / 4, ONE * 28 / 10),
        ("chum_watchtower", v3(5 * ONE, 0, 2 * ONE), 0, ONE * 4),
        ("chum_well", v3(0, 0, 0), 0, ONE * 12 / 10),
        ("chum_notice", v3(-2 * ONE, 0, ONE), fx::TURN / 8, ONE * 15 / 10),
        ("chum_stall", v3(2 * ONE, 0, -ONE), fx::TURN / 2, ONE * 16 / 10),
        ("chum_gate", v3(0, 0, 6 * ONE), 0, ONE * 2),
        ("lamp", v3(-3 * ONE, 0, 3 * ONE), 0, ONE * 2),
        ("lamp", v3(3 * ONE, 0, 3 * ONE), 0, ONE * 2),
        ("tree_a", v3(-7 * ONE, 0, -5 * ONE), 0, ONE * 26 / 10),
        ("tree_b", v3(7 * ONE, 0, -5 * ONE), 0, ONE * 24 / 10),
        ("chum_house", v3(-8 * ONE, 0, ONE), fx::TURN / 4, ONE * 2),
        ("chum_house", v3(8 * ONE, 0, -ONE), -fx::TURN / 4, ONE * 2),
        ("chum_mill", v3(-8 * ONE, 0, -3 * ONE), 0, ONE * 3),
        ("chum_tent", v3(8 * ONE, 0, 3 * ONE), 0, ONE * 18 / 10),
    ];
    for (name, at, yaw, h) in place {
        model_h(s, b, name, at, yaw, h);
    }
    // a guild's city shows its guild's tower over the roofs (T27)
    if let Screen::City { city, .. } = &s.screen {
        if let Some(g) = city.guild() {
            let name = format!("tower_{}", g.name().to_lowercase());
            model_h(s, b, &name, v3(0, 0, -9 * ONE), 0, ONE * 9 / 2);
        }
    }
    let k = fx::div(ONE * 16 / 10, s.look.height("hero").max(ONE / 10));
    draw_hero(
        s,
        b,
        v3(0, 0, 3 * ONE),
        &Pose {
            yaw: fx::TURN / 2,
            scale: k,
            ..Pose::default()
        },
    );
}

fn draw_armourer(s: &Session, b: &mut Batch) {
    model_h(s, b, "pedestal", V3::ZERO, 0, ONE / 2);
    let yaw = (s.now / 1000 % 8000) as i32 * fx::TURN / 8000;
    let name = format!("armour_{}", s.showing);
    let h = s.look.height(&name).max(ONE / 10);
    let r = s.look.radius(&name).max(ONE / 10);
    // fit the stand: about a metre and a half of piece
    let k = fx::div(ONE * 15 / 10, h.max(r * 2));
    if s.look.has(&name) {
        s.look.draw(
            b,
            &name,
            v3(0, ONE / 2, 0),
            &Pose {
                yaw,
                scale: k,
                ..Pose::default()
            },
        );
    }
    b.solid(
        Mesh::Cube,
        v3(0, -ONE / 2, 0),
        &upright(40 * ONE, ONE / 2, 40 * ONE),
        hex("#2b2140"),
    );
}

fn draw_deck(s: &Session, b: &mut Batch) {
    b.solid(
        Mesh::Cube,
        v3(0, -ONE / 8, 0),
        &upright(10 * ONE, ONE / 4, 6 * ONE),
        hex("#6a4a32"),
    );
    let n = s.player().deck.len() as i32;
    for i in 0..n {
        let yaw = ((i * 37) % 11 - 5) * fx::TURN / 900;
        let f = Frame3::new(yaw, 0, 0, v3(ONE * 63 / 100, ONE / 50, ONE * 88 / 100));
        b.solid(
            Mesh::Cube,
            v3(-ONE * 2, ONE / 50 * i + ONE / 100, 0),
            &f,
            if i == n - 1 {
                hex("#3a2a5a")
            } else {
                hex("#e8e0d0")
            },
        );
    }
    b.solid(
        Mesh::Cyl,
        v3(-ONE * 2, ONE / 50 * n + ONE / 40, 0),
        &upright(ONE * 3 / 10, ONE / 60, ONE * 3 / 10),
        hex("#f2c14e"),
    );
}

// ---------- the model sheet ----------

pub fn model_pages(s: &Session) -> usize {
    s.look.models.len().div_ceil(MODELS_PER_PAGE).max(1)
}

pub fn model_page(s: &Session, page: usize) -> Vec<String> {
    s.look
        .models
        .keys()
        .skip(page * MODELS_PER_PAGE)
        .take(MODELS_PER_PAGE)
        .cloned()
        .collect()
}

pub fn model_spots(s: &Session, page: usize) -> Vec<(V3, String)> {
    model_page(s, page)
        .into_iter()
        .enumerate()
        .map(|(i, n)| {
            let x = (i as i32 % 4 * 2 - 3) * ONE * 3;
            let z = (i as i32 / 4 - 1) * ONE * 4;
            (v3(x, -ONE / 2, z + ONE * 2), n)
        })
        .collect()
}

fn draw_models(s: &Session, b: &mut Batch, page: usize) {
    let yaw = (s.now / 1000 % 10_000) as i32 * fx::TURN / 10_000;
    for (i, n) in model_page(s, page).iter().enumerate() {
        let x = (i as i32 % 4 * 2 - 3) * ONE * 3;
        let z = (i as i32 / 4 - 1) * ONE * 4;
        let h = s.look.height(n).max(ONE / 10);
        let r = s.look.radius(n).max(ONE / 10);
        let k = fx::div(ONE * 26 / 10, h.max(r * 2));
        s.look.draw(
            b,
            n,
            v3(x, 0, z),
            &Pose {
                yaw,
                scale: k,
                phase: yaw * 3,
                ..Pose::default()
            },
        );
        b.solid(
            Mesh::Cyl,
            v3(x, -ONE / 20, z),
            &upright(ONE * 3, ONE / 10, ONE * 3),
            hex("#cdb894"),
        );
    }
}

// ---------- the frame ----------

pub fn build(s: &mut Session) {
    let (w, h) = s.pixels;
    let mut out = FrameOut::default();
    let cam = camera_for(s);
    let screen = s.screen.clone();
    let dark = matches!(screen, Screen::Battle(ref f) if f.guardian)
        || matches!(
            screen,
            Screen::City {
                room: Room::Armourer,
                ..
            } | Screen::Lair
                | Screen::Rakdos
                | Screen::Victory
        );
    out.viewport(0, 0, w, h, &cam, &if dark { dark_mood() } else { mood() });
    let mut b = Batch::new();
    let board = matches!(
        screen,
        Screen::Board
            | Screen::Draw { .. }
            | Screen::Story { .. }
            | Screen::Fate { .. }
            | Screen::Result { .. }
            | Screen::Land { .. }
            | Screen::Lair
            | Screen::Rakdos
    ) && s.world.is_some();
    if board
        || matches!(
            screen,
            Screen::Title | Screen::Mission | Screen::NewGame | Screen::Starter { .. }
        )
    {
        if s.statics_dirty || s.statics.parts.is_empty() {
            let world = match &s.world {
                Some(w) => w.clone(),
                None => World::new(1, &s.cards),
            };
            let still = board_statics(s, &world);
            let mut st = crate::render::StaticScene::default();
            st.add(&still);
            st.version = s.statics.version + 1;
            s.statics = st;
            s.statics_dirty = false;
        }
        for part in s.statics.parts.clone() {
            out.static_draws(&part);
        }
        if s.world.is_some() && board {
            let at = hero_spot(s);
            let phase = (s.now / 1000 % 3000) as i32 * fx::TURN / 3000;
            if s.look.has("hero") {
                draw_hero(
                    s,
                    &mut b,
                    at,
                    &Pose {
                        yaw: fx::TURN / 2,
                        scale: BOARD_SCALE,
                        phase,
                        ..Pose::default()
                    },
                );
            }
            if let Screen::Land { .. } = screen {
                let reach = s.player().reach(&s.armoury);
                let me = s.player().pos;
                for (&p, t) in &s.world().tiles {
                    if t.visited && t.lands.len() < t.land_room() {
                        b.push(
                            Mesh::Ground,
                            Pass::Glow,
                            tile_centre(p).add(v3(0, ONE / 30, 0)),
                            &upright(TILE * 9 / 10, ONE, TILE * 9 / 10),
                            hex("#f2c14e"),
                            ONE / 3,
                        );
                    } else if distance(p, me) <= reach {
                        b.push(
                            Mesh::Ground,
                            Pass::Glow,
                            tile_centre(p).add(v3(0, ONE / 30, 0)),
                            &upright(TILE * 9 / 10, ONE, TILE * 9 / 10),
                            hex("#5a8ad0"),
                            ONE / 5,
                        );
                    }
                }
            }
        }
    } else {
        match &screen {
            Screen::Battle(f) => draw_battle(s, &mut b, f),
            Screen::City {
                room: Room::Armourer, ..
            } => draw_armourer(s, &mut b),
            Screen::City { .. } => draw_city(s, &mut b),
            Screen::Deck { .. } => draw_deck(s, &mut b),
            Screen::Models { page } => draw_models(s, &mut b, *page),
            Screen::Victory => {
                let riot = s.world.as_ref().and_then(|w| w.riot.as_ref()).is_some();
                model_h(
                    s,
                    &mut b,
                    if riot { "c187" } else { "c183" },
                    V3::ZERO,
                    0,
                    4 * ONE,
                )
            }
            _ => {}
        }
    }
    out.batch(&b);
    s.frame_out = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_centre_is_two_metres_a_step() {
        assert_eq!(tile_centre((1, -2)), v3(2 * ONE, 0, -4 * ONE));
    }

    #[test]
    fn the_board_draws_and_the_static_part_is_built_once() {
        let mut s = crate::session::for_tests();
        s.act("game.new", "").unwrap();
        s.act("game.guild", "Izzet").unwrap();
        s.act("starter.start", "").unwrap();
        s.act("result.next", "").unwrap();
        s.act("city.leave", "").unwrap();
        build(&mut s);
        assert!(!s.statics.values.is_empty());
        let v = s.statics.version;
        build(&mut s);
        assert_eq!(s.statics.version, v);
        assert!(!s.frame_out.commands.is_empty());
    }

    #[test]
    fn a_blocker_stands_in_front_of_its_attacker() {
        let s = crate::session::for_tests();
        let mk = |id: u32, side: u8, attacking: bool, blocking: i32| PermSnap {
            id,
            side,
            card: -1,
            token: "token_goblin".into(),
            name: String::new(),
            power: 1,
            toughness: 1,
            damage: 0,
            tapped: false,
            attacking,
            blocking,
            detained: false,
            creature: true,
            land: false,
            attached: -1,
            counters: 0,
            fresh: false,
            loyalty: 0,
        };
        let snap = Snap {
            life: [1, 1],
            hand: [0, 0],
            library: [0, 0],
            grave: [0, 0],
            pile: [0, 0],
            player_hand: vec![],
            perms: vec![mk(1, 0, true, -1), mk(2, 1, false, 1)],
            stack: vec![],
            aims: vec![],
            step: crate::duel::Step::Damage,
            active: 0,
        };
        let spots = battle_spots(&s, &snap);
        let a = spots.iter().find(|(_, p)| p.id == 1).unwrap().0;
        let b = spots.iter().find(|(_, p)| p.id == 2).unwrap().0;
        assert_eq!(a.x, b.x);
        assert!(b.z < a.z);
    }

    #[test]
    fn the_top_of_the_stack_draws_an_arc_to_each_thing_it_aims_at() {
        let s = crate::session::for_tests();
        let mk = |id: u32, side: u8| PermSnap {
            id,
            side,
            card: -1,
            token: "token_goblin".into(),
            name: String::new(),
            power: 1,
            toughness: 1,
            damage: 0,
            tapped: false,
            attacking: false,
            blocking: -1,
            detained: false,
            creature: true,
            land: false,
            attached: -1,
            counters: 0,
            fresh: false,
            loyalty: 0,
        };
        let aims =
            |from: Option<u32>, perms: Vec<u32>, players: Vec<u8>| crate::duel::Aims { from, perms, players };
        let mut snap = Snap {
            life: [1, 1],
            hand: [0, 0],
            library: [0, 0],
            grave: [0, 0],
            pile: [0, 0],
            player_hand: vec![],
            perms: vec![mk(1, 0), mk(2, 1)],
            // bottom: the foe's spell at the player; top: the player's at creature 2
            stack: vec![(1, "a".into(), -1), (0, "b".into(), -1)],
            aims: vec![aims(None, vec![], vec![0]), aims(None, vec![2], vec![])],
            step: crate::duel::Step::Main1,
            active: 0,
        };
        let table = snap.clone();
        let spots = battle_spots(&s, &table);
        let at = |id: u32| {
            spots
                .iter()
                .find(|(_, p)| p.id == id)
                .unwrap()
                .0
                .add(v3(0, ONE, 0))
        };
        // a spell: from its controller's side, to its target, the top item only
        assert_eq!(aim_arcs(&snap, &spots), vec![(side_point(0), at(2))]);
        // an ability: from the permanent whose it is, to a player
        snap.aims[1] = aims(Some(1), vec![], vec![1]);
        assert_eq!(aim_arcs(&snap, &spots), vec![(at(1), side_point(1))]);
        snap.stack.clear();
        snap.aims.clear();
        assert!(aim_arcs(&snap, &spots).is_empty());
    }

    #[test]
    fn an_entering_creature_falls_from_three_metres_and_lands_two_fifths_in() {
        assert_eq!(enter_drop(0), 3 * ONE);
        // halfway to landing it has a quarter of the height left
        assert_eq!(enter_drop(ONE / 5), 3 * ONE / 4);
        assert_eq!(enter_drop(ONE * 2 / 5), 0);
        assert_eq!(enter_drop(ONE), 0);
    }
}
