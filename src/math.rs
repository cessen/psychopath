#![allow(dead_code)]

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
