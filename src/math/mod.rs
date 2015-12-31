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
pub trait DotProduct
{
    fn dot(self, other: Self) -> f32;
}

pub fn dot<T: DotProduct>(a: T, b: T) -> f32 {
    a.dot(b)
}


/// Trait for calculating cross products.
pub trait CrossProduct
{
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
