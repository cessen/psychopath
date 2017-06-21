//! Utilities for handling floating point precision issues
//!
//! This is based on the work in section 3.9 of "Physically Based Rendering:
//! From Theory to Implementation" 3rd edition by Pharr et al.

use math::{Point, Vector, Normal, dot};

#[inline(always)]
pub fn fp_gamma(n: u32) -> f32 {
    use std::f32::EPSILON;
    let e = EPSILON * 0.5;
    (e * n as f32) / (1.0 - (e * n as f32))
}


pub fn increment_ulp(v: f32) -> f32 {
    // Handle special cases
    if (v.is_infinite() && v > 0.0) || v.is_nan() {
        return v;
    }

    // Handle zero
    let v = if v == -0.0 { 0.0 } else { v };

    // Increase ulp by 1
    if v >= 0.0 {
        bits_to_f32(f32_to_bits(v) + 1)
    } else {
        bits_to_f32(f32_to_bits(v) - 1)
    }
}


pub fn decrement_ulp(v: f32) -> f32 {
    // Handle special cases
    if (v.is_infinite() && v < 0.0) || v.is_nan() {
        return v;
    }

    // Handle zero
    let v = if v == 0.0 { -0.0 } else { v };

    // Decrease ulp by 1
    if v <= -0.0 {
        bits_to_f32(f32_to_bits(v) + 1)
    } else {
        bits_to_f32(f32_to_bits(v) - 1)
    }
}

pub fn robust_ray_origin(pos: Point, pos_err: Vector, nor: Normal, ray_dir: Vector) -> Point {
    // Get surface normal pointing in the same
    // direction as ray_dir.
    let nor = {
        let nor = nor.into_vector();
        if dot(nor, ray_dir) >= 0.0 { nor } else { -nor }
    };

    // Calculate offset point
    let d = dot(nor.abs(), pos_err);
    let offset = nor * d;
    let p = pos + offset;

    // Calculate ulp offsets
    //
    // The additiona/subtraction of MIN_POSITIVE is to keep numbers out of the
    // subnormal range when the original value is 0.0, because that seems to be
    // causing issues.  Not sure why.  For now this seems like a reasonable
    // solution because it only affects extremely small numbers near zero
    // anyway.  But this seems worth investigating at some point.
    //
    // TODO: investigate cause of subnormal numbers being a problem, and fix
    // if possible.  Test case: a horizontal plane at z = 0.0 and four lights
    // evenly spaced apart at z = 4.0.  Causes strange render artifacts.
    use std::f32::MIN_POSITIVE;

    let x = if nor.x() >= 0.0 {
        increment_ulp(p.x() + MIN_POSITIVE)
    } else {
        decrement_ulp(p.x() - MIN_POSITIVE)
    };

    let y = if nor.y() >= 0.0 {
        increment_ulp(p.y() + MIN_POSITIVE)
    } else {
        decrement_ulp(p.y() - MIN_POSITIVE)
    };

    let z = if nor.z() >= 0.0 {
        increment_ulp(p.z() + MIN_POSITIVE)
    } else {
        decrement_ulp(p.z() - MIN_POSITIVE)
    };

    Point::new(x, y, z)
}


#[inline(always)]
fn f32_to_bits(v: f32) -> u32 {
    use std::mem::transmute_copy;
    unsafe { transmute_copy::<f32, u32>(&v) }
}

#[inline(always)]
fn bits_to_f32(bits: u32) -> f32 {
    use std::mem::transmute_copy;
    unsafe { transmute_copy::<u32, f32>(&bits) }
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
