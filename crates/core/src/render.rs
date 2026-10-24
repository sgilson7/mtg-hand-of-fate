//! What the WebGL page draws, as numbers the core builds (Builder D3).
//! After Long Time Comin's `render.rs`, with Long Time Comin's ways of
//! drawing a great many orbs (SECOND-ORDER R3.3):
//!
//! - one instanced draw per mesh and pass, 16 values an instance: three
//!   columns of the shape's transform with the translation in their w,
//!   then rgb and glow;
//! - a static buffer for what does not move, uploaded once a scene, and a
//!   dynamic buffer re-sent each frame;
//! - a glow pass, added with no depth writes, for orbs, fire and light.
//!
//! An ink pass, after Protection of the Spheres: each solid is drawn again,
//! back faces only, pushed out along its normals and filled with ink, so
//! every shape carries a dark outline. It reuses the solid pass's instances
//! (the same buffer, offset and count), so it costs draws, not data. A
//! frame is a list of commands: a viewport (with the view and the mood it
//! uses) and draws. (From Mage Tower Battle's render.rs, MIT, Sam Gilson.)

use crate::fx::{self, Fx, ONE};
use crate::geom::{self, v3, M4, V3};
use crate::mesh::Mesh;

pub const INSTANCE_VALUES: usize = 16;
pub const COMMAND_VALUES: usize = 6;
/// Per view: the 16-value matrix, the eye and a pad, the clear colour,
/// then the mood: the fog's start and end, the light from the sky, the
/// light bounced from the ground, the sun's colour and where it shines
/// from (each three values and a pad).
pub const VIEW_VALUES: usize = 44;
/// Where each of the view's uniforms starts in its values, and its size.
pub const VIEW_UNIFORMS: [(&str, usize, usize); 7] = [
    ("uEye", 16, 3),
    ("uFogColour", 20, 3),
    ("uFog", 24, 2),
    ("uSkyLight", 28, 3),
    ("uGroundLight", 32, 3),
    ("uSun", 36, 3),
    ("uSunDir", 40, 3),
];
pub const MESHES: usize = Mesh::ALL.len();

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pass {
    Solid = 0,
    Glow = 1,
    /// Back faces pushed out and drawn in ink: the outline.
    Ink = 2,
}

/// The passes that hold instances; the ink pass replays the solid one.
pub const PASSES: [Pass; 2] = [Pass::Solid, Pass::Glow];
pub const STATIC: u32 = 0;
pub const DYNAMIC: u32 = 1;

/// A colour in Q16 thirds, 0..=ONE each.
pub type Rgb = [Fx; 3];

pub fn hex(s: &str) -> Rgb {
    let h = s.trim_start_matches('#');
    let p = |i: usize| i32::from_str_radix(h.get(i..i + 2).unwrap_or("80"), 16).unwrap_or(128);
    [p(0) * ONE / 255, p(2) * ONE / 255, p(4) * ONE / 255]
}

pub fn mix(a: Rgb, b: Rgb, t: Fx) -> Rgb {
    [
        a[0] + fx::mul(b[0] - a[0], t),
        a[1] + fx::mul(b[1] - a[1], t),
        a[2] + fx::mul(b[2] - a[2], t),
    ]
}

/// Rotates a vector by roll (about z), then pitch (about x), then yaw
/// (about y), angles in turns.
pub fn rotate(v: V3, yaw: i32, pitch: i32, roll: i32) -> V3 {
    let (sr, cr) = (fx::sin(roll), fx::cos(roll));
    let v = v3(
        fx::mul(v.x, cr) - fx::mul(v.y, sr),
        fx::mul(v.x, sr) + fx::mul(v.y, cr),
        v.z,
    );
    let (sp, cp) = (fx::sin(pitch), fx::cos(pitch));
    let v = v3(
        v.x,
        fx::mul(v.y, cp) - fx::mul(v.z, sp),
        fx::mul(v.y, sp) + fx::mul(v.z, cp),
    );
    v.yaw(yaw)
}

/// An orientation and size: the columns of a shape's transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame3 {
    pub x: V3,
    pub y: V3,
    pub z: V3,
}

impl Frame3 {
    pub fn new(yaw: i32, pitch: i32, roll: i32, size: V3) -> Frame3 {
        Frame3 {
            x: rotate(v3(size.x, 0, 0), yaw, pitch, roll),
            y: rotate(v3(0, size.y, 0), yaw, pitch, roll),
            z: rotate(v3(0, 0, size.z), yaw, pitch, roll),
        }
    }
    pub fn upright(size: V3) -> Frame3 {
        Frame3 {
            x: v3(size.x, 0, 0),
            y: v3(0, size.y, 0),
            z: v3(0, 0, size.z),
        }
    }
}

/// How a place is lit: Ape Race's recipe of sky light from above, the
/// ground's colour bounced from below, a sun whose light wraps part way
/// round each shape, and haze in the place's own colour (T34).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mood {
    /// The haze's colour, which is also the sky behind everything.
    pub fog: Rgb,
    /// Haze starts this far from the eye and is whole at `fog_far`.
    pub fog_near: Fx,
    pub fog_far: Fx,
    pub sky: Rgb,
    pub ground: Rgb,
    pub sun: Rgb,
    /// Toward the sun, from the scene.
    pub sun_dir: V3,
}

/// Instances grouped by pass and mesh.
#[derive(Debug, Clone, Default)]
pub struct Batch {
    lists: Vec<Vec<i32>>,
}

impl Batch {
    pub fn new() -> Batch {
        Batch {
            lists: vec![Vec::new(); PASSES.len() * MESHES],
        }
    }

    pub fn push(&mut self, mesh: Mesh, pass: Pass, at: V3, f: &Frame3, rgb: Rgb, glow: Fx) {
        let l = &mut self.lists[pass as usize * MESHES + mesh as usize];
        l.extend_from_slice(&[
            f.x.x, f.x.y, f.x.z, at.x, f.y.x, f.y.y, f.y.z, at.y, f.z.x, f.z.y, f.z.z, at.z, rgb[0], rgb[1],
            rgb[2], glow,
        ]);
    }

    /// A solid shape, lit by the mood.
    pub fn solid(&mut self, mesh: Mesh, at: V3, f: &Frame3, rgb: Rgb) {
        self.push(mesh, Pass::Solid, at, f, rgb, 0);
    }

    /// A glowing shape: a solid core and a soft halo twice its size.
    pub fn orb(&mut self, at: V3, r: Fx, rgb: Rgb) {
        let f = Frame3::upright(v3(r * 2, r * 2, r * 2));
        self.push(Mesh::Blob, Pass::Solid, at, &f, rgb, ONE);
        let h = Frame3::upright(v3(r * 4, r * 4, r * 4));
        self.push(Mesh::Halo, Pass::Glow, at, &h, rgb, ONE / 2);
    }

    #[cfg(test)]
    pub fn lists_for_test(&self) -> &[Vec<i32>] {
        &self.lists
    }

    pub fn count(&self) -> usize {
        self.lists.iter().map(|l| l.len() / INSTANCE_VALUES).sum()
    }

    /// Appends the batch's values to `values` and its draws to `commands`,
    /// in pass order, for buffer slot `buffer`.
    pub fn emit(&self, buffer: u32, values: &mut Vec<i32>, commands: &mut Vec<u32>) {
        for pass in PASSES {
            for mesh in Mesh::ALL {
                let l = &self.lists[pass as usize * MESHES + mesh as usize];
                if l.is_empty() {
                    continue;
                }
                let offset = (values.len() * 4) as u32;
                values.extend_from_slice(l);
                commands.extend_from_slice(&[
                    1,
                    buffer,
                    mesh as u32,
                    offset,
                    (l.len() / INSTANCE_VALUES) as u32,
                    pass as u32,
                ]);
                if pass == Pass::Solid && mesh != Mesh::Ground && mesh != Mesh::Halo {
                    commands.extend_from_slice(&[
                        1,
                        buffer,
                        mesh as u32,
                        offset,
                        (l.len() / INSTANCE_VALUES) as u32,
                        Pass::Ink as u32,
                    ]);
                }
            }
        }
    }
}

/// A camera: where it is, where it looks, and its lens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camera {
    pub eye: V3,
    pub at: V3,
    /// 1 / tan(half the vertical field of view), Q16.
    pub focal: Fx,
}

impl Camera {
    pub fn matrix(&self, aspect: Fx) -> M4 {
        let proj = geom::perspective(self.focal, aspect.max(1), ONE / 5, 300 * ONE);
        geom::mat_mul(&proj, &geom::look_at(self.eye, self.at))
    }
}

/// One frame's commands and values.
#[derive(Debug, Clone, Default)]
pub struct FrameOut {
    pub commands: Vec<u32>,
    pub values: Vec<i32>,
    pub views: Vec<i32>,
}

impl FrameOut {
    /// Starts a viewport (pixels from the bottom left) drawn with `cam` and
    /// lit by `mood`; the haze's colour is its sky.
    pub fn viewport(&mut self, x: i32, y: i32, w: i32, h: i32, cam: &Camera, mood: &Mood) {
        let aspect = (w.max(1) as i64 * ONE as i64 / h.max(1) as i64) as Fx;
        let m = cam.matrix(aspect);
        let at = self.views.len() as u32;
        self.views.extend_from_slice(&m);
        self.views
            .extend_from_slice(&[cam.eye.x, cam.eye.y, cam.eye.z, 0]);
        let c = mood.fog;
        self.views.extend_from_slice(&[c[0], c[1], c[2], ONE]);
        self.views.extend_from_slice(&[mood.fog_near, mood.fog_far, 0, 0]);
        for c in [mood.sky, mood.ground, mood.sun] {
            self.views.extend_from_slice(&[c[0], c[1], c[2], 0]);
        }
        let d = mood.sun_dir.with_len(ONE);
        self.views.extend_from_slice(&[d.x, d.y, d.z, 0]);
        self.commands.extend_from_slice(&[
            0,
            x.max(0) as u32,
            y.max(0) as u32,
            w.max(1) as u32,
            h.max(1) as u32,
            at,
        ]);
    }

    pub fn batch(&mut self, b: &Batch) {
        b.emit(DYNAMIC, &mut self.values, &mut self.commands);
    }

    /// Replays a static scene's draws (already offset into the static buffer).
    pub fn static_draws(&mut self, draws: &[u32]) {
        self.commands.extend_from_slice(draws);
    }
}

/// A scene part that does not move, uploaded once.
#[derive(Debug, Clone, Default)]
pub struct StaticScene {
    pub values: Vec<i32>,
    /// Draw commands for each part, offset into `values`.
    pub parts: Vec<Vec<u32>>,
    pub version: u32,
}

impl StaticScene {
    pub fn add(&mut self, b: &Batch) -> usize {
        let mut cmds = Vec::new();
        b.emit(STATIC, &mut self.values, &mut cmds);
        self.parts.push(cmds);
        self.parts.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_batch_emits_one_draw_per_mesh_and_pass_with_byte_offsets() {
        let mut b = Batch::new();
        let f = Frame3::upright(v3(ONE, ONE, ONE));
        b.solid(Mesh::Sphere, V3::ZERO, &f, [ONE, 0, 0]);
        b.solid(Mesh::Sphere, v3(ONE, 0, 0), &f, [ONE, 0, 0]);
        b.push(Mesh::Cube, Pass::Solid, V3::ZERO, &f, [0, ONE, 0], 0);
        b.push(Mesh::Cube, Pass::Glow, V3::ZERO, &f, [0, ONE, 0], ONE);
        // one instance a shape: no outline copies
        assert_eq!(b.count(), 4);
        let (mut values, mut cmds) = (vec![0; 16], Vec::new());
        b.emit(DYNAMIC, &mut values, &mut cmds);
        // solid cube and its ink, solid spheres and their ink, glow cube: five draws
        assert_eq!(cmds.len(), 5 * COMMAND_VALUES);
        // the first starts after the 16 values already there: byte 64
        assert_eq!(
            &cmds[..6],
            &[1, DYNAMIC, Mesh::Cube as u32, 64, 1, Pass::Solid as u32]
        );
        assert_eq!(
            &cmds[6..12],
            &[1, DYNAMIC, Mesh::Cube as u32, 64, 1, Pass::Ink as u32]
        );
        assert_eq!(
            &cmds[12..18],
            &[1, DYNAMIC, Mesh::Sphere as u32, 128, 2, Pass::Solid as u32]
        );
        assert_eq!(cmds[29], Pass::Glow as u32);
        assert_eq!(values.len(), 16 + 4 * INSTANCE_VALUES);
    }

    #[test]
    fn a_viewport_carries_its_mood_where_the_page_reads_it() {
        let mood = Mood {
            fog: [ONE, 0, 0],
            fog_near: 20 * ONE,
            fog_far: 50 * ONE,
            sky: [0, ONE, 0],
            ground: [0, 0, ONE],
            sun: [ONE, ONE, 0],
            sun_dir: v3(0, 3 * ONE, 4 * ONE),
        };
        let cam = Camera {
            eye: v3(ONE, 2 * ONE, 3 * ONE),
            at: V3::ZERO,
            focal: ONE,
        };
        let mut f = FrameOut::default();
        f.viewport(0, 0, 10, 10, &cam, &mood);
        let at = |name: &str| {
            let (_, start, size) = VIEW_UNIFORMS.iter().find(|u| u.0 == name).copied().unwrap();
            f.views[start..start + size].to_vec()
        };
        assert_eq!(f.views.len(), VIEW_VALUES);
        assert_eq!(at("uEye"), vec![ONE, 2 * ONE, 3 * ONE]);
        assert_eq!(at("uFogColour"), vec![ONE, 0, 0]);
        assert_eq!(at("uFog"), vec![20 * ONE, 50 * ONE]);
        assert_eq!(at("uSkyLight"), vec![0, ONE, 0]);
        assert_eq!(at("uGroundLight"), vec![0, 0, ONE]);
        assert_eq!(at("uSun"), vec![ONE, ONE, 0]);
        // (0, 3, 4) made one long: (0, 0.6, 0.8)
        let d = at("uSunDir");
        assert!(
            d[0] == 0 && (d[1] - ONE * 3 / 5).abs() < 50 && (d[2] - ONE * 4 / 5).abs() < 50,
            "{d:?}"
        );
    }

    #[test]
    fn a_quarter_turn_of_yaw_turns_x_into_minus_z() {
        let v = rotate(v3(ONE, 0, 0), fx::QUARTER_TURN, 0, 0);
        assert!(v.x.abs() < 200 && (v.z + ONE).abs() < 200, "{v:?}");
        let h = hex("#ff8000");
        assert_eq!(h, [ONE, 128 * ONE / 255, 0]);
    }
}
