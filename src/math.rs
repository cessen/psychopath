#![allow(dead_code)]
pub use math3d::{Matrix4x4, Normal, Point, Vector, DotProduct, dot, CrossProduct, cross};

/// Clamps a value between a min and max.
pub fn clamp<T: PartialOrd>(v: T, lower: T, upper: T) -> T {
    if v < lower {
        lower
    } else if v > upper {
        upper
    } else {
        v
    }
}

// The stdlib min function is slower than a simple if statement for some reason.
pub fn fast_minf32(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

// The stdlib max function is slower than a simple if statement for some reason.
pub fn fast_maxf32(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// Rounds an integer up to the next power of two.
pub fn upper_power_of_two(mut v: u32) -> u32 {
    v -= 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v + 1
}

/// Gets the log base 2 of the given integer
pub fn log2_64(mut value: u64) -> u64 {
    // This works by doing a binary search for the largest non-zero binary
    // digit in the number.  Its bit position is then the log2 of the integer.

    let mut log = 0;

    const POWERS: [(u64, u64); 6] = [
        (32, (1 << 32) - 1),
        (16, (1 << 16) - 1),
        (8, (1 << 8) - 1),
        (4, (1 << 4) - 1),
        (2, (1 << 2) - 1),
        (1, (1 << 1) - 1),
    ];

    for &(i, j) in POWERS.iter() {
        let tmp = value >> i;
        if tmp != 0 {
            log += i;
            value = tmp;
        } else {
            value &= j;
        }
    }

    log
}

/// Creates a coordinate system from a single vector.
///
/// The input vector, v, becomes the first vector of the
/// returned tuple, with the other two vectors in the returned
/// tuple defining the orthoganal axes.
///
/// Algorithm taken from "Building an Orthonormal Basis, Revisited" by Duff et al.
pub fn coordinate_system_from_vector(v: Vector) -> (Vector, Vector, Vector) {
    let sign = v.z().signum();
    let a = -1.0 / (sign + v.z());
    let b = v.x() * v.y() * a;
    let v2 = Vector::new(1.0 + sign * v.x() * v.x() * a, sign * b, -sign * v.x());
    let v3 = Vector::new(b, sign + v.y() * v.y() * a, -v.y());

    (v, v2, v3)
}

/// Simple mapping of a vector that exists in a z-up space to
/// the space of another vector who's direction is considered
/// z-up for the purpose.
/// Obviously this doesn't care about the direction _around_
/// the z-up, although it will be sufficiently consistent for
/// isotropic sampling purposes.
///
/// from: The vector we're transforming.
/// toz: The vector whose space we are transforming "from" into.
///
/// Returns the transformed vector.
pub fn zup_to_vec(from: Vector, toz: Vector) -> Vector {
    let (toz, tox, toy) = coordinate_system_from_vector(toz.normalized());

    // Use simple linear algebra to convert the "from"
    // vector to a space composed of tox, toy, and toz
    // as the x, y, and z axes.
    (tox * from.x()) + (toy * from.y()) + (toz * from.z())
}

/// The logit function, scaled to approximate the probit function.
///
/// We use this as a close approximation to the gaussian inverse CDF,
/// since the gaussian inverse CDF (probit) has no analytic formula.
pub fn logit(p: f32, width: f32) -> f32 {
    let n = 0.001 + (p * 0.998);

    (n / (1.0 - n)).ln() * width * (0.6266 / 4.0)
}

pub fn fast_logit(p: f32, width: f32) -> f32 {
    let n = 0.001 + (p * 0.998);

    fast_ln((n / (1.0 - n))) * width * (0.6266 / 4.0)
}


//----------------------------------------------------------------
// Adapted to Rust from https://code.google.com/archive/p/fastapprox/

pub fn fast_ln(x: f32) -> f32 {
    use std::mem::transmute_copy;

    let mut y = unsafe { transmute_copy::<f32, u32>(&x) as f32 };
    y *= 8.2629582881927490e-8;
    return y - 87.989971088;
}

pub fn fast_pow2(p: f32) -> f32 {
    use std::mem::transmute_copy;

    let offset: f32 = if p < 0.0 { 1.0 } else { 0.0 };
    let clipp: f32 = if p < -126.0 { -126.0 } else { p };
    let w: i32 = clipp as i32;
    let z: f32 = clipp - w as f32 + offset;

    let i: u32 = ((1 << 23) as f32 *
                      (clipp + 121.2740575 + 27.7280233 / (4.84252568 - z) - 1.49012907 * z)) as
        u32;

    unsafe { transmute_copy::<u32, f32>(&i) }
}

pub fn fast_exp(p: f32) -> f32 {
    fast_pow2(1.442695040 * p)
}

pub fn faster_pow2(p: f32) -> f32 {
    use std::mem::transmute_copy;

    let clipp: f32 = if p < -126.0 { -126.0 } else { p };
    let i: u32 = ((1 << 23) as f32 * (clipp + 126.94269504)) as u32;

    unsafe { transmute_copy::<u32, f32>(&i) }
}

pub fn faster_exp(p: f32) -> f32 {
    faster_pow2(1.442695040 * p)
}

// End of adapted code
//----------------------------------------------------------------


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log2_64_test() {
        assert_eq!(0, log2_64(0));

        for i in 0..64 {
            assert_eq!(i, log2_64(1 << i));
        }

        for i in 8..64 {
            assert_eq!(i, log2_64((1 << i) + 227));
        }

        for i in 16..64 {
            assert_eq!(i, log2_64((1 << i) + 56369));
        }

        for i in 32..64 {
            assert_eq!(i, log2_64((1 << i) + 2514124923));
        }
    }
}
