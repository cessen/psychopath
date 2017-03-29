#![allow(dead_code)]

mod matrix;
mod normal;
mod point;
mod vector;

pub use self::matrix::Matrix4x4;
pub use self::normal::Normal;
pub use self::point::Point;
pub use self::vector::Vector;


/// Trait for calculating dot products.
pub trait DotProduct {
    fn dot(self, other: Self) -> f32;
}

pub fn dot<T: DotProduct>(a: T, b: T) -> f32 {
    a.dot(b)
}


/// Trait for calculating cross products.
pub trait CrossProduct {
    fn cross(self, other: Self) -> Self;
}

pub fn cross<T: CrossProduct>(a: T, b: T) -> T {
    a.cross(b)
}


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

// Adapted from from http://fastapprox.googlecode.com
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
pub fn log2_64(value: u64) -> u64 {
    const TAB64: [u64; 64] = [63, 0, 58, 1, 59, 47, 53, 2, 60, 39, 48, 27, 54, 33, 42, 3, 61, 51,
                              37, 40, 49, 18, 28, 20, 55, 30, 34, 11, 43, 14, 22, 4, 62, 57, 46,
                              52, 38, 26, 32, 41, 50, 36, 17, 19, 29, 10, 13, 21, 56, 45, 25, 31,
                              35, 16, 9, 12, 44, 24, 15, 8, 23, 7, 6, 5];

    let value = value | value.wrapping_shr(1);
    let value = value | value.wrapping_shr(2);
    let value = value | value.wrapping_shr(4);
    let value = value | value.wrapping_shr(8);
    let value = value | value.wrapping_shr(16);
    let value = value | value.wrapping_shr(32);

    TAB64[((value.wrapping_sub(value.wrapping_shr(1)) as u64).wrapping_mul(0x07EDD5E59A4E28C2))
        .wrapping_shr(58) as usize]
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
