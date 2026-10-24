//! What the WebGL page draws for each screen, as a draw list the core
//! builds (Builder D3). The board is the 2D grid made 3D: each tile a slab,
//! themed by its terrain (a little mountain on a Mountain, a canal through
//! an Island), its district's tower, a face-down card standing over a tile
//! not yet drawn, crystals for the lands you placed. A fight is an arena
//! with both sides' creatures; a city is a plaza of CHUMWORLD's buildings;
//! the armourer turns a piece on a stand.

use crate::battle::{Ev, PermSnap, Snap};
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
        let top = t.guild.map(guild_tint).unwrap_or_else(|| terrain_top(t.terrain));
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
                        match (ew, ns) {
                            (true, _) => model(s, &mut b, "canal_straight", c, 0, ONE),
                            (false, true) => model(s, &mut b, "canal_straight", c, fx::TURN / 4, ONE),
                            _ => model(s, &mut b, "canal_pool", c, 0, ONE),
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
    b
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
        Screen::Title | Screen::NewGame => {
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
        Screen::Battle(_) => pitched(
            v3(0, ONE / 2, ONE * 2),
            fx::TURN * 50 / 360,
            15 * ONE,
            ONE * 26 / 10,
        ),
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
pub fn battle_spots<'a>(s: &Session, snap: &'a Snap) -> Vec<(V3, &'a PermSnap)> {
    let _ = s;
    let mut out = Vec::new();
    for side in 0..2u8 {
        let mine: Vec<&PermSnap> = snap
            .perms
            .iter()
            .filter(|p| p.side == side && p.attached < 0)
            .collect();
        let creatures: Vec<&&PermSnap> = mine.iter().filter(|p| p.creature).collect();
        let others: Vec<&&PermSnap> = mine.iter().filter(|p| !p.creature).collect();
        let dir = if side == 0 { 1 } else { -1 };
        let n = creatures.len() as i32;
        for (i, p) in creatures.iter().enumerate() {
            let x = (i as i32 * 2 - (n - 1)) * ONE * 11 / 10;
            let z = if p.attacking {
                dir * ONE * 3 / 5
            } else {
                dir * ONE * 19 / 10
            };
            out.push((v3(x, 0, z), **p));
        }
        let m = others.len() as i32;
        for (i, p) in others.iter().enumerate() {
            let x = (i as i32 * 2 - (m - 1)) * ONE * 7 / 10;
            out.push((v3(x, 0, dir * ONE * 36 / 10), **p));
        }
    }
    // a blocker stands in front of what it blocks
    let spots: Vec<(V3, &PermSnap)> = out.clone();
    for (pt, p) in out.iter_mut() {
        if p.blocking >= 0 {
            if let Some((apt, _)) = spots.iter().find(|(_, a)| a.id as i32 == p.blocking) {
                *pt = v3(apt.x, 0, if p.side == 0 { ONE * 3 / 10 } else { -ONE * 3 / 10 });
            }
        }
    }
    out
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
    let ev = &f.record.events[f.shown];
    let phase = (s.now / 1000 % 4000) as i32 * fx::TURN / 4000;
    for (pt, p) in battle_spots(s, &ev.snap) {
        let name = if p.card >= 0 {
            let c = &s.cards[p.card as usize];
            if c.is(crate::content::ty::CREATURE) {
                format!("c{}", c.number)
            } else {
                String::new()
            }
        } else {
            p.token.clone()
        };
        let striking = matches!(ev.ev, Ev::Hit | Ev::Face | Ev::Ability) && ev.focus.first() == Some(&p.id);
        let struck =
            matches!(ev.ev, Ev::Hit | Ev::Dies) && ev.focus.last() == Some(&p.id) && !ev.focus.is_empty();
        let yaw = if p.side == 0 { fx::TURN / 2 } else { 0 };
        if name.is_empty() || !p.creature {
            // an enchantment or artifact: a glowing gem in its colours
            b.solid(
                Mesh::Gem,
                pt.add(v3(0, ONE / 2, 0)),
                &upright(ONE / 2, ONE * 3 / 4, ONE / 2),
                hex("#b9a6e8"),
            );
            continue;
        }
        let height = if p.name == "Niv-Mizzet, Dracogenius" {
            ONE * 45 / 10
        } else {
            battle_height(p.power, p.toughness)
        };
        let mh = s.look.height(&name).max(ONE / 10);
        let k = fx::div(height, mh);
        let pose = Pose {
            yaw,
            scale: k,
            phase: phase + p.id as i32 * 1700,
            lunge: if striking { ONE * 7 / 10 } else { 0 },
            flash: struck && ev.ev == Ev::Hit,
            tapped: p.tapped,
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
    // the player stands behind their side
    model_h(
        s,
        b,
        "hero",
        v3(-ONE * 5, 0, ONE * 3),
        fx::TURN / 2 + fx::TURN / 10,
        ONE * 16 / 10,
    );
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
    model_h(s, b, "hero", v3(0, 0, 3 * ONE), fx::TURN / 2, ONE * 16 / 10);
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
    ) && s.world.is_some();
    if board || matches!(screen, Screen::Title | Screen::NewGame) {
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
                s.look.draw(
                    &mut b,
                    "hero",
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
            Screen::Victory => model_h(s, &mut b, "c183", V3::ZERO, 0, 4 * ONE),
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
            attached: -1,
        };
        let snap = Snap {
            life: [1, 1],
            bank: [vec![], vec![]],
            hand: [0, 0],
            library: [0, 0],
            grave: [0, 0],
            player_hand: vec![],
            perms: vec![mk(1, 0, true, -1), mk(2, 1, false, 1)],
        };
        let spots = battle_spots(&s, &snap);
        let a = spots.iter().find(|(_, p)| p.id == 1).unwrap().0;
        let b = spots.iter().find(|(_, p)| p.id == 2).unwrap().0;
        assert_eq!(a.x, b.x);
        assert!(b.z < a.z);
    }
}
