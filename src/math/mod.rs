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
