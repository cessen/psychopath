#![allow(dead_code)]

extern crate float4;

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
    #[inline]
    fn dot(self, other: Self) -> f32;
}

#[inline]
pub fn dot<T: DotProduct>(a: T, b: T) -> f32 {
    a.dot(b)
}


/// Trait for calculating cross products.
pub trait CrossProduct {
    #[inline]
    fn cross(self, other: Self) -> Self;
}

#[inline]
pub fn cross<T: CrossProduct>(a: T, b: T) -> T {
    a.cross(b)
}
