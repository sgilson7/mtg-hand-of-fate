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
    /// Tapped: turned a quarter, as a tapped card turns sideways.
    pub tapped: bool,
    /// Faded toward the haze: detained.
    pub dim: bool,
}

const MAGENTA: Rgb = [ONE, 0, ONE];
const WING_FLAP: i32 = fx::TURN / 16;
const HEAD_NOD: i32 = fx::TURN / 80;
const TAIL_SWAY: i32 = fx::TURN / 30;
const BOB: Fx = ONE / 60;

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
        let Some(parts) = self.models.get(model) else {
            return;
        };
        let k = if pose.scale == 0 { ONE } else { pose.scale };
        let s = fx::sin(pose.phase);
        let bob = fx::mul(s.abs(), BOB);
        // tapped: the whole model turns a quarter about its feet, as a card turns sideways
        let turn = pose.yaw + if pose.tapped { fx::TURN / 4 } else { 0 };
        let forward = rotate(v3(0, 0, ONE), pose.yaw, 0, 0);
        let shove = forward.scale(pose.lunge);
        for part in parts {
            let (mut pitch, mut roll, yaw) = part.tilt;
            let mut at = part.at;
            match part.node {
                Node::WingL => roll += fx::mul(s, WING_FLAP),
                Node::WingR => roll -= fx::mul(s, WING_FLAP),
                Node::Head => pitch += fx::mul(s, HEAD_NOD),
                Node::Tail => roll += fx::mul(s, TAIL_SWAY),
                Node::Body => {}
            }
            at.y += bob;
            let at = rotate(at, turn, 0, 0).scale(k).add(base).add(shove);
            let f = Frame3::new(turn + yaw, pitch, roll, part.size.scale(k));
            let rgb = if pose.flash {
                [ONE, ONE, ONE]
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
