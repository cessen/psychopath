#![allow(dead_code)]

mod normal;
mod point;
mod transform;
mod vector;

pub use self::{normal::Normal, point::Point, transform::Transform, vector::Vector};

/// Trait for calculating dot products.
pub trait DotProduct {
    fn dot(self, other: Self) -> f32;
}

#[inline]
pub fn dot<T: DotProduct>(a: T, b: T) -> f32 {
    a.dot(b)
}

/// Trait for calculating cross products.
pub trait CrossProduct {
    fn cross(self, other: Self) -> Self;
}

#[inline]
pub fn cross<T: CrossProduct>(a: T, b: T) -> T {
    a.cross(b)
}
