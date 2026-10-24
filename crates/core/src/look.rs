//! How things look: the models (`data/models.json`, written by
//! `tools/models/*.py`) and the palette. A model is parts: a unit mesh with
//! a position and a size in centimetres, a colour, a tilt and the node it
//! moves with. Creatures move as a few nodes: the body bobs, wings flap,
//! the head nods, a tail sways. After Mage Tower Battle's `look.rs`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};
use crate::mesh::Mesh;
use crate::render::{hex, rotate, Batch, Frame3, Pass, Rgb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    Body,
    Head,
    WingL,
    WingR,
    Tail,
}

impl Node {
    fn from_name(s: &str) -> Option<Node> {
        Some(match s {
            "body" => Node::Body,
            "head" => Node::Head,
            "wing_l" => Node::WingL,
            "wing_r" => Node::WingR,
            "tail" => Node::Tail,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawPart {
    m: String,
    p: [i32; 3],
    s: [i32; 3],
    c: String,
    #[serde(default)]
    n: String,
    /// pitch, roll and (optionally) yaw, in thousandths of a turn
    #[serde(default)]
    t: Vec<i32>,
    #[serde(default)]
    g: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct RawModels {
    models: BTreeMap<String, Vec<RawPart>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub mesh: Mesh,
    pub at: V3,
    pub size: V3,
    pub rgb: Rgb,
    pub node: Node,
    pub tilt: (i32, i32, i32),
    pub glow: Fx,
}

#[derive(Debug, Clone, Default)]
pub struct Look {
    pub palette: BTreeMap<String, Rgb>,
    pub models: BTreeMap<String, Vec<Part>>,
}

/// How a model moves this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pose {
    /// Turns about the vertical.
    pub yaw: i32,
    /// Q16: 0 means one.
    pub scale: Fx,
    /// A phase in turns: the body bobs, wings flap and the tail sways with it.
    pub phase: i32,
    /// A shove toward where it faces, in Q16 metres (an attack).
    pub lunge: Fx,
    /// Struck: drawn white.
    pub flash: bool,
    /// Tapped: tired. It slumps forward, breathes slow and deep, droops its
    /// wings and head, and is drawn greyed out (the person's rule).
    pub tired: bool,
    /// An attack, after Protection of the Spheres' swing (`models.rs`
    /// there): Q16 0..ONE from wind-up to the hit, then `recover` 0..ONE
    /// through the follow-through. Both 0 at rest.
    pub swing: Fx,
    pub recover: Fx,
    /// Faded toward the haze: detained.
    pub dim: bool,
}

/// Which parts of a model to draw, and how to fit them: scaled by `scale`
/// and moved by `offset` in the model's own space, before it is posed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fit {
    pub scale: Fx,
    pub offset: V3,
    /// Keep only parts left of the centre line (-1), right of it (1), or all (0).
    pub side: i32,
    /// Leave out parts of this colour role's colour above this height, when set:
    /// the mage's own cap under a helm, its body under a cuirass.
    pub hide: [Option<(Rgb, Fx, Fx)>; 6],
}

impl Fit {
    pub const WHOLE: Fit = Fit {
        scale: ONE,
        offset: V3::ZERO,
        side: 0,
        hide: [None; 6],
    };

    fn keeps(&self, p: &Part) -> bool {
        let side_ok = match self.side {
            0 => true,
            s => p.at.x * s > 0,
        };
        let shown = !self
            .hide
            .iter()
            .flatten()
            .any(|(rgb, lo, hi)| p.rgb == *rgb && p.at.y >= *lo && p.at.y < *hi);
        side_ok && shown
    }
}

const MAGENTA: Rgb = [ONE, 0, ONE];
const WING_FLAP: i32 = fx::TURN / 16;
const HEAD_NOD: i32 = fx::TURN / 80;
const TAIL_SWAY: i32 = fx::TURN / 30;
const BOB: Fx = ONE / 60;
/// Tired: how far it slumps forward, and how much grey it takes.
const TIRED_SLUMP: i32 = fx::TURN / 30;
const TIRED_GREY: Fx = ONE * 13 / 20;
const TIRED_SINK: Fx = ONE / 25;

/// Eases in and out: slow at both ends, quick in the middle. From
/// Protection of the Spheres (`models.rs`, MIT, Sam Gilson).
fn smooth(t: i32) -> i32 {
    let t = t.clamp(0, ONE);
    fx::mul(fx::mul(t, t), ONE * 3 - 2 * t)
}

/// The body's lean through an attack, in turns, after Protection of the
/// Spheres' `lean`: back a little on the wind-up, held, forward hard into
/// the strike, upright again by the end of the follow-through.
pub fn attack_lean(swing: Fx, recover: Fx) -> i32 {
    let back = -fx::TURN / 72;
    let strike = fx::TURN / 22;
    if recover > 0 {
        return fx::mul(ONE - smooth(recover), strike);
    }
    if swing <= 0 {
        0
    } else if swing < ONE * 13 / 20 {
        fx::mul(smooth(fx::div(swing, ONE * 13 / 20)), back)
    } else {
        let u = fx::div(swing - ONE * 13 / 20, ONE * 7 / 20);
        back + fx::mul(fx::mul(u, u), strike - back)
    }
}

/// How far forward the attack carries the model, in Q16 of a lunge: a step
/// back on the wind-up, a dash in at the strike, back home after.
pub fn attack_reach(swing: Fx, recover: Fx) -> Fx {
    if recover > 0 {
        return ONE - smooth(recover);
    }
    if swing <= 0 {
        0
    } else if swing < ONE * 13 / 20 {
        -fx::mul(smooth(fx::div(swing, ONE * 13 / 20)), ONE / 5)
    } else {
        let u = fx::div(swing - ONE * 13 / 20, ONE * 7 / 20);
        -ONE / 5 + fx::mul(fx::mul(u, u), ONE + ONE / 5)
    }
}

impl Look {
    pub fn parse(palette_json: &str, models_json: &str) -> Result<Look, String> {
        let raw: BTreeMap<String, serde_json::Value> =
            serde_json::from_str(palette_json).map_err(|e| format!("The palette could not be read: {e}"))?;
        let mut palette = BTreeMap::new();
        for (k, v) in raw {
            if k.starts_with('_') {
                continue;
            }
            let s = v
                .as_str()
                .ok_or_else(|| format!("The palette role \"{k}\" is not a colour."))?;
            palette.insert(k, hex(s));
        }
        let m: RawModels =
            serde_json::from_str(models_json).map_err(|e| format!("The models could not be read: {e}"))?;
        let mut models = BTreeMap::new();
        for (name, parts) in m.models {
            let mut out = Vec::new();
            for p in parts {
                let mesh = Mesh::from_name(&p.m).ok_or_else(|| {
                    format!("The model \"{name}\" uses a mesh \"{}\" there is none of.", p.m)
                })?;
                let node = if p.n.is_empty() {
                    Node::Body
                } else {
                    Node::from_name(&p.n)
                        .ok_or_else(|| format!("The model \"{name}\" names a node \"{}\".", p.n))?
                };
                let rgb = if p.c.starts_with('#') {
                    hex(&p.c)
                } else {
                    *palette.get(&p.c).ok_or_else(|| {
                        format!(
                            "The model \"{name}\" uses a colour \"{}\" the palette does not have.",
                            p.c
                        )
                    })?
                };
                let cm = |v: i32| v * ONE / 100;
                let t = |i: usize| p.t.get(i).copied().unwrap_or(0) * fx::TURN / 1000;
                out.push(Part {
                    mesh,
                    at: v3(cm(p.p[0]), cm(p.p[1]), cm(p.p[2])),
                    size: v3(cm(p.s[0]), cm(p.s[1]), cm(p.s[2])),
                    rgb,
                    node,
                    tilt: (t(0), t(1), t(2)),
                    glow: p.g * ONE / 1000,
                });
            }
            models.insert(name, out);
        }
        Ok(Look { palette, models })
    }

    pub fn colour(&self, role: &str) -> Rgb {
        self.palette.get(role).copied().unwrap_or(MAGENTA)
    }

    pub fn has(&self, model: &str) -> bool {
        self.models.contains_key(model)
    }

    /// The tallest point of a model, in Q16 metres.
    pub fn height(&self, model: &str) -> Fx {
        self.models
            .get(model)
            .map(|ps| ps.iter().map(|p| p.at.y + p.size.y / 2).max().unwrap_or(ONE))
            .unwrap_or(ONE)
    }

    /// The widest reach of a model from its centre line, in Q16 metres.
    pub fn radius(&self, model: &str) -> Fx {
        self.models
            .get(model)
            .map(|ps| {
                ps.iter()
                    .map(|p| p.at.x.abs().max(p.at.z.abs()) + p.size.x.max(p.size.z) / 2)
                    .max()
                    .unwrap_or(ONE)
            })
            .unwrap_or(ONE)
    }

    /// Draws a model standing at `base` in a pose.
    pub fn draw(&self, b: &mut Batch, model: &str, base: V3, pose: &Pose) {
        self.draw_fitted(b, model, base, pose, &Fit::WHOLE);
    }

    /// Draws some of a model's parts, scaled and moved within the model's own
    /// space before it is posed: a piece of armour fitted onto the mage.
    pub fn draw_fitted(&self, b: &mut Batch, model: &str, base: V3, pose: &Pose, fit: &Fit) {
        let Some(parts) = self.models.get(model) else {
            return;
        };
        let k = if pose.scale == 0 { ONE } else { pose.scale };
        // tired: breathing at a third the pace and twice as deep, slumped and sunk
        let phase = if pose.tired { pose.phase / 3 } else { pose.phase };
        let s = fx::sin(phase);
        let bob = if pose.tired {
            fx::mul(s.abs(), BOB * 2) - TIRED_SINK
        } else {
            fx::mul(s.abs(), BOB)
        };
        let lean = attack_lean(pose.swing, pose.recover) + if pose.tired { TIRED_SLUMP } else { 0 };
        let turn = pose.yaw;
        let forward = rotate(v3(0, 0, ONE), pose.yaw, 0, 0);
        let reach = pose.lunge + fx::mul(attack_reach(pose.swing, pose.recover), ONE * 7 / 10);
        let shove = forward.scale(reach);
        let s = if pose.tired { s / 3 } else { s };
        for part in parts.iter().filter(|p| fit.keeps(p)) {
            let (mut pitch, mut roll, yaw) = part.tilt;
            let mut at = part.at.scale(fit.scale).add(fit.offset);
            match part.node {
                Node::WingL => roll += fx::mul(s, WING_FLAP) - if pose.tired { WING_FLAP } else { 0 },
                Node::WingR => roll -= fx::mul(s, WING_FLAP) - if pose.tired { WING_FLAP } else { 0 },
                Node::Head => pitch += fx::mul(s, HEAD_NOD) + if pose.tired { TIRED_SLUMP } else { 0 },
                Node::Tail => roll += fx::mul(s, TAIL_SWAY),
                Node::Body => {}
            }
            at.y += bob;
            // the lean turns the whole model about its feet, toward where it faces
            let at = rotate(at, 0, lean, 0);
            let at = rotate(at, turn, 0, 0).scale(k).add(base).add(shove);
            let f = Frame3::new(
                turn + yaw,
                pitch + lean,
                roll,
                part.size.scale(fx::mul(k, fit.scale)),
            );
            let rgb = if pose.flash {
                [ONE, ONE, ONE]
            } else if pose.tired {
                let grey = (part.rgb[0] + part.rgb[1] + part.rgb[2]) / 3;
                crate::render::mix(part.rgb, [grey, grey, grey * 21 / 20], TIRED_GREY)
            } else if pose.dim {
                crate::render::mix(part.rgb, [ONE * 7 / 10, ONE * 7 / 10, ONE * 8 / 10], ONE / 2)
            } else {
                part.rgb
            };
            if part.glow > 0 {
                b.push(part.mesh, Pass::Solid, at, &f, rgb, part.glow);
            } else {
                b.solid(part.mesh, at, &f, rgb);
            }
        }
    }
}

#[cfg(test)]
pub fn for_tests() -> Look {
    Look::parse(
        include_str!("../../../data/palette.json"),
        include_str!("../../../data/models.json"),
    )
    .expect("look")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_cube() -> Look {
        let pal = r##"{"red": "#ff0000"}"##;
        let models =
            r##"{"models": {"x": [{"m": "cube", "p": [0, 50, 0], "s": [100, 100, 100], "c": "red"}]}}"##;
        Look::parse(pal, models).unwrap()
    }

    #[test]
    fn a_tired_model_keeps_its_facing_and_is_greyed() {
        let l = one_cube();
        let draw = |tired: bool| {
            let mut b = Batch::new();
            l.draw(
                &mut b,
                "x",
                V3::ZERO,
                &Pose {
                    tired,
                    ..Pose::default()
                },
            );
            b
        };
        let (rest, tired) = (draw(false), draw(true));
        let solid = |b: &Batch| {
            b.lists_for_test()[Pass::Solid as usize * crate::render::MESHES + Mesh::Cube as usize].clone()
        };
        let (a, t) = (solid(&rest), solid(&tired));
        // the x axis of the cube's frame: turned a quarter it would point along z
        assert!(
            t[0] > t[2].abs() && t[5] > t[4].abs(),
            "turned or tipped over: {:?}",
            &t[..12]
        );
        assert_eq!(a[12], ONE, "red at rest");
        // greyed: red falls and green rises toward the grey
        assert!(t[12] < ONE && t[13] > 0);
    }

    #[test]
    fn an_attack_leans_back_then_strikes_forward_and_comes_home() {
        assert!(attack_lean(ONE / 3, 0) < 0, "the wind-up leans back");
        assert!(attack_lean(ONE, 0) > fx::TURN / 30, "the hit leans forward");
        assert_eq!(attack_lean(0, ONE), 0, "upright after the follow-through");
        // full reach at the hit (to fixed-point rounding), home after
        assert!((attack_reach(ONE, 0) - ONE).abs() < 16 && attack_reach(0, ONE) == 0);
        assert!(attack_reach(ONE / 2, 0) < 0, "a step back to wind up");
    }

    #[test]
    fn a_part_in_hex_or_a_palette_role_parses_and_an_unknown_role_is_refused() {
        let pal = r##"{"ink": "#000000", "gold": "#ff0000"}"##;
        let models = r##"{"models": {"x": [{"m": "cube", "p": [0, 50, 0], "s": [100, 100, 100], "c": "gold"},
            {"m": "sphere", "p": [0, 150, 0], "s": [40, 40, 40], "c": "#00ff00", "n": "head", "t": [0, 0, 250]}]}}"##;
        let l = Look::parse(pal, models).unwrap();
        assert_eq!(l.models["x"][0].rgb, [ONE, 0, 0]);
        assert_eq!(l.models["x"][1].node, Node::Head);
        assert_eq!(l.models["x"][1].tilt.2, fx::TURN / 4);
        // tallest point: 150 + 20 cm
        assert_eq!(l.height("x"), ONE * 170 / 100);
        let bad = models.replace("gold", "plaid");
        assert_eq!(
            Look::parse(pal, &bad).unwrap_err(),
            "The model \"x\" uses a colour \"plaid\" the palette does not have."
        );
    }

    #[test]
    fn every_creature_and_token_has_a_model() {
        let look = for_tests();
        let cards = crate::content::for_tests();
        for c in cards.iter().filter(|c| c.is(crate::content::ty::CREATURE)) {
            assert!(look.has(&format!("c{}", c.number)), "no model for {}", c.name);
        }
        for t in [
            "token_bird",
            "token_soldier",
            "token_goblin",
            "token_centaur",
            "token_rhino",
            "token_knight",
            "hero",
        ] {
            assert!(look.has(t), "no model {t}");
        }
    }
}
