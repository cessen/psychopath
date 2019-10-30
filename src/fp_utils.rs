//! Utilities for handling floating point precision issues
//!
//! This is based on the work in section 3.9 of "Physically Based Rendering:
//! From Theory to Implementation" 3rd edition by Pharr et al.

use crate::math::{dot, Normal, Point, Vector};

#[inline(always)]
pub fn fp_gamma(n: u32) -> f32 {
    use std::f32::EPSILON;
    let e = EPSILON * 0.5;
    (e * n as f32) / (1.0 - (e * n as f32))
}

pub fn increment_ulp(v: f32) -> f32 {
    if v.is_finite() {
        if v > 0.0 {
            f32::from_bits(v.to_bits() + 1)
        } else if v < -0.0 {
            f32::from_bits(v.to_bits() - 1)
        } else {
            f32::from_bits(0x00_00_00_01)
        }
    } else {
        // Infinity or NaN.
        v
    }
}

pub fn decrement_ulp(v: f32) -> f32 {
    if v.is_finite() {
        if v > 0.0 {
            f32::from_bits(v.to_bits() - 1)
        } else if v < -0.0 {
            f32::from_bits(v.to_bits() + 1)
        } else {
            f32::from_bits(0x80_00_00_01)
        }
    } else {
        // Infinity or NaN.
        v
    }
}

pub fn robust_ray_origin(pos: Point, pos_err: f32, nor: Normal, ray_dir: Vector) -> Point {
    // Get surface normal pointing in the same
    // direction as ray_dir.
    let nor = {
        let nor = nor.into_vector();
        if dot(nor, ray_dir) >= 0.0 {
            nor
        } else {
            -nor
        }
    };

    // Calculate offset point
    let d = dot(nor.abs(), Vector::new(pos_err, pos_err, pos_err));
    let offset = nor * d;
    let p = pos + offset;

    // Calculate ulp offsets
    let x = if nor.x() >= 0.0 {
        increment_ulp(p.x())
    } else {
        decrement_ulp(p.x())
    };

    let y = if nor.y() >= 0.0 {
        increment_ulp(p.y())
    } else {
        decrement_ulp(p.y())
    };

    let z = if nor.z() >= 0.0 {
        increment_ulp(p.z())
    } else {
        decrement_ulp(p.z())
    };

    Point::new(x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inc_ulp() {
        assert!(increment_ulp(1.0) > 1.0);
        assert!(increment_ulp(-1.0) > -1.0);
    }

    #[test]
    fn dec_ulp() {
        assert!(decrement_ulp(1.0) < 1.0);
        assert!(decrement_ulp(-1.0) < -1.0);
    }

    #[test]
    fn inc_ulp_zero() {
        assert!(increment_ulp(0.0) > 0.0);
        assert!(increment_ulp(0.0) > -0.0);
        assert!(increment_ulp(-0.0) > 0.0);
        assert!(increment_ulp(-0.0) > -0.0);
    }

    #[test]
    fn dec_ulp_zero() {
        assert!(decrement_ulp(0.0) < 0.0);
        assert!(decrement_ulp(0.0) < -0.0);
        assert!(decrement_ulp(-0.0) < 0.0);
        assert!(decrement_ulp(-0.0) < -0.0);
    }

    #[test]
    fn inc_dec_ulp() {
        assert_eq!(decrement_ulp(increment_ulp(1.0)), 1.0);
        assert_eq!(decrement_ulp(increment_ulp(-1.0)), -1.0);
        assert_eq!(decrement_ulp(increment_ulp(1.2)), 1.2);
        assert_eq!(decrement_ulp(increment_ulp(-1.2)), -1.2);
    }

    #[test]
    fn dec_inc_ulp() {
        assert_eq!(increment_ulp(decrement_ulp(1.0)), 1.0);
        assert_eq!(increment_ulp(decrement_ulp(-1.0)), -1.0);
        assert_eq!(increment_ulp(decrement_ulp(1.2)), 1.2);
        assert_eq!(increment_ulp(decrement_ulp(-1.2)), -1.2);
    }

}
