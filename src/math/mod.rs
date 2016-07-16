#![allow(dead_code)]

mod vector;
mod normal;
mod point;
mod matrix;

pub use self::vector::Vector;
pub use self::normal::Normal;
pub use self::point::Point;
pub use self::matrix::Matrix4x4;

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

// Adapted from from http://fastapprox.googlecode.com
pub fn fast_ln(x: f32) -> f32 {
    use std::mem::transmute_copy;

    let mut y = unsafe { transmute_copy::<f32, u32>(&x) as f32 };
    y *= 8.2629582881927490e-8;
    return y - 87.989971088;
}



pub fn log2_64(value: u64) -> u64 {
    const TAB64: [u64; 64] = [63, 0, 58, 1, 59, 47, 53, 2, 60, 39, 48, 27, 54, 33, 42, 3, 61, 51,
                              37, 40, 49, 18, 28, 20, 55, 30, 34, 11, 43, 14, 22, 4, 62, 57, 46,
                              52, 38, 26, 32, 41, 50, 36, 17, 19, 29, 10, 13, 21, 56, 45, 25, 31,
                              35, 16, 9, 12, 44, 24, 15, 8, 23, 7, 6, 5];

    let value = value | (value >> 1);
    let value = value | (value >> 2);
    let value = value | (value >> 4);
    let value = value | (value >> 8);
    let value = value | (value >> 16);
    let value = value | (value >> 32);

    TAB64[((((value - (value >> 1)) * 0x07EDD5E59A4E28C2)) >> 58) as usize]
}



/// Creates a coordinate system from a single vector.
pub fn coordinate_system_from_vector(v: Vector) -> (Vector, Vector, Vector) {
    let v2 = if v[0].abs() > v[1].abs() {
        let invlen = 1.0 / ((v[0] * v[0]) + (v[2] * v[2])).sqrt();
        Vector::new(-v[2] * invlen, 0.0, v[0] * invlen)
    } else {
        let invlen = 1.0 / ((v[1] * v[1]) + (v[2] * v[2])).sqrt();
        Vector::new(0.0, v[2] * invlen, -v[1] * invlen)
    };

    let v3 = cross(v, v2);

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
/// Returns he transformed vector.
pub fn zup_to_vec(from: Vector, toz: Vector) -> Vector {
    let (toz, tox, toy) = coordinate_system_from_vector(toz.normalized());

    // Use simple linear algebra to convert the "from"
    // vector to a space composed of tox, toy, and toz
    // as the x, y, and z axes.
    (tox * from[0]) + (toy * from[1]) + (toz * from[2])
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
