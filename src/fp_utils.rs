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
    let v = if v == -0.0 { 0.0 } else { v };

    // Decrease ulp by 1
    if v >= 0.0 {
        bits_to_f32(f32_to_bits(v) - 1)
    } else {
        bits_to_f32(f32_to_bits(v) + 1)
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
    let x = if offset.x() >= 0.0 {
        increment_ulp(p.x())
    } else {
        decrement_ulp(p.x())
    };

    let y = if offset.y() >= 0.0 {
        increment_ulp(p.y())
    } else {
        decrement_ulp(p.y())
    };

    let z = if offset.z() >= 0.0 {
        increment_ulp(p.z())
    } else {
        decrement_ulp(p.z())
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
