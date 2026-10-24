//! Vectors and matrices in Q16.16 metres (`fx.rs`). Every position in the
//! game is a `V3`; the ground is y = 0 and y points up.

use serde::{Deserialize, Serialize};

use crate::fx::{self, Fx, ONE};

/// Ticks a second for every simulation in the game.
pub const TPS: i32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct V3 {
    pub x: Fx,
    pub y: Fx,
    pub z: Fx,
}

pub const fn v3(x: Fx, y: Fx, z: Fx) -> V3 {
    V3 { x, y, z }
}

/// A position from whole centimetres, for content and tests.
pub const fn cm(x: i32, y: i32, z: i32) -> V3 {
    V3 {
        x: x * ONE / 100,
        y: y * ONE / 100,
        z: z * ONE / 100,
    }
}

// Named methods rather than operator traits, so saturation is visible at the call.
#[allow(clippy::should_implement_trait, clippy::len_without_is_empty)]
impl V3 {
    pub const ZERO: V3 = V3 { x: 0, y: 0, z: 0 };
    pub fn add(self, o: V3) -> V3 {
        v3(
            self.x.saturating_add(o.x),
            self.y.saturating_add(o.y),
            self.z.saturating_add(o.z),
        )
    }
    pub fn sub(self, o: V3) -> V3 {
        v3(
            self.x.saturating_sub(o.x),
            self.y.saturating_sub(o.y),
            self.z.saturating_sub(o.z),
        )
    }
    pub fn scale(self, k: Fx) -> V3 {
        v3(fx::mul(self.x, k), fx::mul(self.y, k), fx::mul(self.z, k))
    }
    /// Multiplies by num/den with integer arithmetic.
    pub fn ratio(self, num: i32, den: i32) -> V3 {
        let f = |a: Fx| ((a as i64 * num as i64) / den.max(1) as i64) as Fx;
        v3(f(self.x), f(self.y), f(self.z))
    }
    pub fn flat(self) -> V3 {
        v3(self.x, 0, self.z)
    }
    pub fn dot(self, o: V3) -> i64 {
        (self.x as i64 * o.x as i64 + self.y as i64 * o.y as i64 + self.z as i64 * o.z as i64) >> 16
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(
            fx::mul(self.y, o.z) - fx::mul(self.z, o.y),
            fx::mul(self.z, o.x) - fx::mul(self.x, o.z),
            fx::mul(self.x, o.y) - fx::mul(self.y, o.x),
        )
    }
    /// Squared length in Q32.32, so it never overflows for arena distances.
    pub fn len2(self) -> i64 {
        self.x as i64 * self.x as i64 + self.y as i64 * self.y as i64 + self.z as i64 * self.z as i64
    }
    pub fn len(self) -> Fx {
        isqrt(self.len2() as u64) as Fx
    }
    pub fn flat_len(self) -> Fx {
        isqrt((self.x as i64 * self.x as i64 + self.z as i64 * self.z as i64) as u64) as Fx
    }
    /// The same direction at length `l`; zero stays zero.
    pub fn with_len(self, l: Fx) -> V3 {
        let n = self.len();
        if n == 0 {
            return V3::ZERO;
        }
        self.ratio(l, n)
    }
    pub fn lerp(self, o: V3, t: Fx) -> V3 {
        self.add(o.sub(self).scale(t))
    }
    /// Rotates about the y axis by `angle` in turns (`fx::TURN`).
    pub fn yaw(self, angle: i32) -> V3 {
        let (s, c) = (fx::sin(angle), fx::cos(angle));
        v3(
            fx::mul(self.x, c) + fx::mul(self.z, s),
            self.y,
            fx::mul(self.z, c) - fx::mul(self.x, s),
        )
    }
}

pub fn isqrt(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    // Newton's method from a power of two at or above the root.
    let mut x = 1u64 << (64 - n.leading_zeros()).div_ceil(2);
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// The yaw (in turns) that faces along the horizontal vector (dx, dz).
pub fn yaw_of(dx: Fx, dz: Fx) -> i32 {
    if dx == 0 && dz == 0 {
        return 0;
    }
    // atan2 by octant and a rational approximation, within half a degree.
    let (ax, az) = (dx.unsigned_abs() as i64, dz.unsigned_abs() as i64);
    let (lo, hi) = if ax < az { (ax, az) } else { (az, ax) };
    let t = (lo << 16) / hi.max(1); // 0..=1 in Q16
                                    // atan(t) ≈ t·(π/4) + 0.273·t·(1−t), in turns: /(2π)
    let a = (t * 8192) >> 16; // t·(1/8 turn) in TURN units (TURN = 65536)
    let corr = (((t * (65536 - t)) >> 16) * 2847) >> 16; // 0.273/(2π)·65536 ≈ 2847
    let mut ang = (a + corr) as i32;
    if ax > az {
        ang = fx::QUARTER_TURN - ang;
    }
    // ang is now the angle from +z toward +x in the first quadrant
    match (dx >= 0, dz >= 0) {
        (true, true) => ang,
        (true, false) => fx::HALF_TURN - ang,
        (false, false) => fx::HALF_TURN + ang,
        (false, true) => fx::TURN - ang,
    }
}

/// A column-major 4×4 matrix in Q16.16, for the camera.
pub type M4 = [Fx; 16];

pub fn mat_mul(a: &M4, b: &M4) -> M4 {
    let mut m = [0; 16];
    for c in 0..4 {
        for r in 0..4 {
            let mut s: i64 = 0;
            for k in 0..4 {
                s += a[k * 4 + r] as i64 * b[c * 4 + k] as i64;
            }
            m[c * 4 + r] = (s >> 16).clamp(i32::MIN as i64, i32::MAX as i64) as Fx;
        }
    }
    m
}

/// Perspective projection with a vertical field of view given by the
/// reciprocal of tan(fov/2), `focal`, both in Q16.
pub fn perspective(focal: Fx, aspect: Fx, near: Fx, far: Fx) -> M4 {
    let mut m = [0; 16];
    m[0] = fx::div(focal, aspect);
    m[5] = focal;
    m[10] = fx::div(far + near, near - far);
    m[11] = -ONE;
    m[14] = fx::div(fx::mul(2 * far, near), near - far);
    m
}

/// A view matrix looking from `eye` toward `at`, with y up.
pub fn look_at(eye: V3, at: V3) -> M4 {
    let f = at.sub(eye).with_len(ONE);
    let mut s = f.cross(v3(0, ONE, 0)).with_len(ONE);
    if s == V3::ZERO {
        s = v3(ONE, 0, 0);
    }
    let u = s.cross(f);
    [
        s.x,
        u.x,
        -f.x,
        0,
        s.y,
        u.y,
        -f.y,
        0,
        s.z,
        u.z,
        -f.z,
        0,
        -(s.dot(eye) as Fx),
        -(u.dot(eye) as Fx),
        f.dot(eye) as Fx,
        ONE,
    ]
}

/// Projects a point through `m` into 0..=1000 screen thousandths (x right,
/// y down), or None when it is behind the eye.
pub fn project(m: &M4, p: V3) -> Option<(i32, i32)> {
    let c = |r: usize| -> i64 {
        (m[r] as i64 * p.x as i64 + m[4 + r] as i64 * p.y as i64 + m[8 + r] as i64 * p.z as i64) / ONE as i64
            + m[12 + r] as i64
    };
    let (x, y, w) = (c(0), c(1), c(3));
    if w <= 0 {
        return None;
    }
    Some(((500 + x * 500 / w) as i32, (500 - y * 500 / w) as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_and_square_roots_match_hand_values() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(1 << 40), 1 << 20);
        // a 3-4-5 triangle in metres
        assert_eq!(v3(3 * ONE, 0, 4 * ONE).len(), 5 * ONE);
        assert_eq!(
            v3(3 * ONE, 0, 4 * ONE).with_len(ONE * 10),
            v3(6 * ONE, 0, 8 * ONE)
        );
    }

    #[test]
    fn yaw_of_points_along_each_axis_and_diagonal() {
        let t = fx::TURN;
        assert_eq!(yaw_of(0, ONE), 0);
        assert_eq!(yaw_of(ONE, 0), t / 4);
        assert_eq!(yaw_of(0, -ONE), t / 2);
        assert_eq!(yaw_of(-ONE, 0), 3 * t / 4);
        // 45 degrees is an eighth of a turn, within half a degree (91 units)
        assert!((yaw_of(ONE, ONE) - t / 8).abs() < 91);
        assert!((yaw_of(-ONE, -ONE) - 5 * t / 8).abs() < 91);
    }

    #[test]
    fn a_point_straight_ahead_projects_to_the_centre() {
        let eye = v3(0, 10 * ONE, 10 * ONE);
        let view = look_at(eye, V3::ZERO);
        let proj = perspective(2 * ONE, ONE, ONE / 5, 200 * ONE);
        let m = mat_mul(&proj, &view);
        let (x, y) = project(&m, V3::ZERO).unwrap();
        assert!((x - 500).abs() <= 1 && (y - 500).abs() <= 1, "{x},{y}");
        assert_eq!(project(&m, v3(0, 20 * ONE, 20 * ONE)), None);
    }
}
