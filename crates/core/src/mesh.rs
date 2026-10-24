//! The unit meshes everything is drawn from, after Long Time Comin's
//! `mesh.rs`. Each is a triangle list of (position, normal) pairs in Q16,
//! centred on the origin and one unit across, so an instance's transform
//! carries its size. Every face is flat, each with its own normal, so a
//! shape reads as facets the way Ape Race's do (T34): the light picks each
//! face out. Stone blocks and rocks have their corners nudged a little, so
//! a wall of them is not ruled straight.

use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mesh {
    Cube = 0,
    /// a smooth globe: heads, bodies, orbs
    Sphere = 1,
    /// a low globe for crowds and scenery
    Blob = 2,
    Cyl = 3,
    Cone = 4,
    /// a flat square facing +y: the ground, rugs, decals
    Ground = 5,
    /// a twenty-sided low-poly crown for trees and rocks
    Ico = 6,
    /// a cylinder narrowing to three fifths at the top: robes, sleeves, hats
    Frustum = 7,
    /// an eight-sided gem: crystals, the staff's stone, stars
    Gem = 8,
    /// a block with its corners nudged: wall stones, books in a pile
    Chunk = 9,
    /// an icosahedron with its corners nudged: rocks, boulders
    Rock = 10,
    /// a smooth globe for the glow pass only: a halo fades toward its
    /// outline, which a faceted shape would break into flat patches
    Halo = 11,
}

impl Mesh {
    pub const ALL: [Mesh; 12] = [
        Mesh::Cube,
        Mesh::Sphere,
        Mesh::Blob,
        Mesh::Cyl,
        Mesh::Cone,
        Mesh::Ground,
        Mesh::Ico,
        Mesh::Frustum,
        Mesh::Gem,
        Mesh::Chunk,
        Mesh::Rock,
        Mesh::Halo,
    ];

    pub fn from_name(s: &str) -> Option<Mesh> {
        match s {
            "cube" => Some(Mesh::Cube),
            "sphere" => Some(Mesh::Sphere),
            "blob" => Some(Mesh::Blob),
            "cyl" => Some(Mesh::Cyl),
            "cone" => Some(Mesh::Cone),
            "ground" => Some(Mesh::Ground),
            "ico" => Some(Mesh::Ico),
            "frustum" => Some(Mesh::Frustum),
            "gem" => Some(Mesh::Gem),
            "chunk" => Some(Mesh::Chunk),
            "rock" => Some(Mesh::Rock),
            "halo" => Some(Mesh::Halo),
            _ => None,
        }
    }
}

/// Values per vertex: position xyz, normal xyz.
pub const VERTEX_VALUES: usize = 6;

const H: Fx = ONE / 2;

fn norm(p: V3) -> V3 {
    p.with_len(ONE)
}

fn push_tri(out: &mut Vec<i32>, a: V3, b: V3, c: V3) {
    let n = norm(b.sub(a).cross(c.sub(a)));
    for p in [a, b, c] {
        out.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z]);
    }
}

/// How far a nudged corner moves on each axis, from a fixed pattern of
/// its position: the same corner always moves the same way, so faces that
/// share it still meet.
const NUDGE: Fx = ONE / 28;

fn nudge(p: V3) -> V3 {
    let h = |a: i32, b: i32, c: i32| {
        let k = (p.x / 97).wrapping_mul(a) ^ (p.y / 89).wrapping_mul(b) ^ (p.z / 83).wrapping_mul(c);
        (k.rem_euclid(2001) - 1000) * NUDGE / 1000
    };
    v3(p.x + h(73, 151, 29), p.y + h(37, 61, 199), p.z + h(113, 17, 167))
}

fn push_quad(out: &mut Vec<i32>, a: V3, b: V3, c: V3, d: V3) {
    push_tri(out, a, b, c);
    push_tri(out, a, c, d);
}

/// Vertex data of a mesh, counter-clockwise from outside.
pub fn vertices(mesh: Mesh) -> Vec<i32> {
    let mut o = Vec::new();
    match mesh {
        Mesh::Cube => {
            let p = |x: i32, y: i32, z: i32| v3(x * H, y * H, z * H);
            push_quad(&mut o, p(1, -1, 1), p(1, -1, -1), p(1, 1, -1), p(1, 1, 1));
            push_quad(&mut o, p(-1, -1, -1), p(-1, -1, 1), p(-1, 1, 1), p(-1, 1, -1));
            push_quad(&mut o, p(-1, 1, 1), p(1, 1, 1), p(1, 1, -1), p(-1, 1, -1));
            push_quad(&mut o, p(-1, -1, -1), p(1, -1, -1), p(1, -1, 1), p(-1, -1, 1));
            push_quad(&mut o, p(-1, -1, 1), p(1, -1, 1), p(1, 1, 1), p(-1, 1, 1));
            push_quad(&mut o, p(1, -1, -1), p(-1, -1, -1), p(-1, 1, -1), p(1, 1, -1));
        }
        Mesh::Ground => {
            let p = |x: i32, z: i32| v3(x * H, 0, z * H);
            push_quad(&mut o, p(-1, 1), p(1, 1), p(1, -1), p(-1, -1));
        }
        Mesh::Sphere | Mesh::Blob | Mesh::Halo => {
            // latitude and longitude bands, each face flat but a halo's
            let (lat, lon) = match mesh {
                Mesh::Sphere => (7, 11),
                Mesh::Blob => (5, 7),
                _ => (8, 14),
            };
            let smooth = mesh == Mesh::Halo;
            let at = |i: i32, j: i32| {
                let a = fx::HALF_TURN * i / lat - fx::QUARTER_TURN;
                let b = fx::TURN * j / lon;
                let r = fx::cos(a);
                v3(
                    fx::mul(fx::mul(r, fx::cos(b)), H),
                    fx::mul(fx::sin(a), H),
                    fx::mul(fx::mul(r, -fx::sin(b)), H),
                )
            };
            let mut tri = |a: V3, b: V3, c: V3| {
                if smooth {
                    for p in [a, b, c] {
                        let n = norm(p);
                        o.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z]);
                    }
                } else {
                    push_tri(&mut o, a, b, c);
                }
            };
            for i in 0..lat {
                for j in 0..lon {
                    let (a, b, c, d) = (at(i, j), at(i, j + 1), at(i + 1, j + 1), at(i + 1, j));
                    if i > 0 {
                        tri(a, b, c);
                    }
                    if i < lat - 1 {
                        tri(a, c, d);
                    }
                }
            }
        }
        Mesh::Gem => {
            let c = [v3(H, 0, 0), v3(0, 0, -H), v3(-H, 0, 0), v3(0, 0, H)];
            for j in 0..4 {
                let (a, b) = (c[j], c[(j + 1) % 4]);
                push_tri(&mut o, a, b, v3(0, H, 0));
                push_tri(&mut o, b, a, v3(0, -H, 0));
            }
        }
        Mesh::Chunk => {
            let p = |x: i32, y: i32, z: i32| nudge(v3(x * H, y * H, z * H));
            push_quad(&mut o, p(1, -1, 1), p(1, -1, -1), p(1, 1, -1), p(1, 1, 1));
            push_quad(&mut o, p(-1, -1, -1), p(-1, -1, 1), p(-1, 1, 1), p(-1, 1, -1));
            push_quad(&mut o, p(-1, 1, 1), p(1, 1, 1), p(1, 1, -1), p(-1, 1, -1));
            push_quad(&mut o, p(-1, -1, -1), p(1, -1, -1), p(1, -1, 1), p(-1, -1, 1));
            push_quad(&mut o, p(-1, -1, 1), p(1, -1, 1), p(1, 1, 1), p(-1, 1, 1));
            push_quad(&mut o, p(1, -1, -1), p(-1, -1, -1), p(-1, 1, -1), p(1, 1, -1));
        }
        Mesh::Ico | Mesh::Rock => {
            // an icosahedron: twelve corners from three golden rectangles
            let g: Fx = ONE * 1618 / 1000;
            let s = |v: V3| v.with_len(H);
            let c = [
                v3(-ONE, g, 0),
                v3(ONE, g, 0),
                v3(-ONE, -g, 0),
                v3(ONE, -g, 0),
                v3(0, -ONE, g),
                v3(0, ONE, g),
                v3(0, -ONE, -g),
                v3(0, ONE, -g),
                v3(g, 0, -ONE),
                v3(g, 0, ONE),
                v3(-g, 0, -ONE),
                v3(-g, 0, ONE),
            ]
            .map(s)
            .map(|v| if mesh == Mesh::Rock { nudge(v) } else { v });
            let faces = [
                (0, 11, 5),
                (0, 5, 1),
                (0, 1, 7),
                (0, 7, 10),
                (0, 10, 11),
                (1, 5, 9),
                (5, 11, 4),
                (11, 10, 2),
                (10, 7, 6),
                (7, 1, 8),
                (3, 9, 4),
                (3, 4, 2),
                (3, 2, 6),
                (3, 6, 8),
                (3, 8, 9),
                (4, 9, 5),
                (2, 4, 11),
                (6, 2, 10),
                (8, 6, 7),
                (9, 8, 1),
            ];
            for (a, b, d) in faces {
                push_tri(&mut o, c[a], c[b], c[d]);
            }
        }
        Mesh::Cyl | Mesh::Cone | Mesh::Frustum => {
            const SIDES: i32 = 9;
            let cone = mesh == Mesh::Cone;
            let top = if mesh == Mesh::Frustum { ONE * 3 / 5 } else { ONE };
            let rim = |j: i32, y: i32| {
                let a = fx::TURN * j / SIDES;
                let r = if y > 0 { fx::mul(H, top) } else { H };
                v3(fx::mul(fx::cos(a), r), y, fx::mul(-fx::sin(a), r))
            };
            let apex = v3(0, H, 0);
            for j in 0..SIDES {
                let (a, b) = (rim(j, -H), rim(j + 1, -H));
                if cone {
                    push_tri(&mut o, a, b, apex);
                } else {
                    let (c, d) = (rim(j + 1, H), rim(j, H));
                    push_quad(&mut o, a, b, c, d);
                    push_tri(&mut o, d, c, apex);
                }
                push_tri(&mut o, b, a, v3(0, -H, 0));
            }
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tris(m: Mesh) -> Vec<[V3; 4]> {
        vertices(m)
            .chunks(3 * VERTEX_VALUES)
            .map(|t| {
                let v = |i: usize| v3(t[i], t[i + 1], t[i + 2]);
                [v(0), v(6), v(12), v(3)]
            })
            .collect()
    }

    #[test]
    fn every_face_of_a_closed_mesh_points_away_from_its_centre() {
        for (m, centre) in [
            (Mesh::Cube, V3::ZERO),
            (Mesh::Sphere, V3::ZERO),
            (Mesh::Blob, V3::ZERO),
            (Mesh::Cyl, V3::ZERO),
            (Mesh::Cone, v3(0, -H / 2, 0)),
            (Mesh::Ico, V3::ZERO),
            (Mesh::Frustum, V3::ZERO),
            (Mesh::Gem, V3::ZERO),
            (Mesh::Chunk, V3::ZERO),
            (Mesh::Rock, V3::ZERO),
            (Mesh::Halo, V3::ZERO),
        ] {
            for [a, b, c, n] in tris(m) {
                let centroid = v3(
                    (a.x + b.x + c.x) / 3,
                    (a.y + b.y + c.y) / 3,
                    (a.z + b.z + c.z) / 3,
                );
                assert!(
                    centroid.sub(centre).dot(n) > 0,
                    "{m:?}: face at {centroid:?} has normal {n:?}"
                );
            }
        }
    }

    #[test]
    fn every_face_is_flat_so_shapes_read_as_facets() {
        // a vertex's normal is its face's, never its own direction from the
        // centre: all three corners of a triangle share one normal. The
        // halo is light, not a shape, and stays smooth.
        for m in Mesh::ALL.into_iter().filter(|&m| m != Mesh::Halo) {
            for t in vertices(m).chunks(3 * VERTEX_VALUES) {
                let n = |i: usize| (t[i + 3], t[i + 4], t[i + 5]);
                assert!(n(0) == n(6) && n(6) == n(12), "{m:?} has a smooth face");
            }
        }
    }

    #[test]
    fn a_frustum_narrows_toward_its_top_and_a_chunk_is_not_a_true_cube() {
        let width_at = |m: Mesh, top: bool| {
            vertices(m)
                .chunks(VERTEX_VALUES)
                .filter(|v| (v[1] > 0) == top)
                .map(|v| v[0].abs())
                .max()
                .unwrap_or(0)
        };
        // the top's corners sit at three fifths of the bottom's: 0.3 against 0.5
        let (top, bottom) = (width_at(Mesh::Frustum, true), width_at(Mesh::Frustum, false));
        assert!(
            (top - ONE * 3 / 10).abs() < ONE / 100 && (bottom - H).abs() < ONE / 100,
            "{top} {bottom}"
        );
        let cube: Vec<i32> = vertices(Mesh::Cube).chunks(VERTEX_VALUES).map(|v| v[0]).collect();
        let chunk: Vec<i32> = vertices(Mesh::Chunk)
            .chunks(VERTEX_VALUES)
            .map(|v| v[0])
            .collect();
        assert!(cube.iter().zip(&chunk).any(|(a, b)| (a - b).abs() > ONE / 100));
        assert!(chunk.iter().all(|x| x.abs() < H + NUDGE + 1));
    }

    #[test]
    fn the_cube_has_twelve_triangles_the_ico_twenty_and_the_ground_faces_up() {
        assert_eq!(tris(Mesh::Cube).len(), 12);
        assert_eq!(tris(Mesh::Ico).len(), 20);
        for [_, _, _, n] in tris(Mesh::Ground) {
            assert_eq!(n, v3(0, ONE, 0));
        }
    }
}
