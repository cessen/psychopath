#![allow(dead_code)]

use std::ops::{Add, Mul};

use approx::relative_eq;
use glam::{Affine3A, Mat3, Mat4, Vec3};

use super::Point;

/// A 4x3 affine transform matrix, used for transforms.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Transform(pub Affine3A);

impl Transform {
    /// Creates a new identity matrix
    #[inline]
    pub fn new() -> Transform {
        Transform(Affine3A::IDENTITY)
    }

    /// Creates a new matrix with the specified values:
    /// a b c d
    /// e f g h
    /// i j k l
    /// m n o p
    #[inline]
    #[allow(clippy::many_single_char_names)]
    #[allow(clippy::too_many_arguments)]
    pub fn new_from_values(
        a: f32,
        b: f32,
        c: f32,
        d: f32,
        e: f32,
        f: f32,
        g: f32,
        h: f32,
        i: f32,
        j: f32,
        k: f32,
        l: f32,
    ) -> Transform {
        Transform(Affine3A::from_mat3_translation(
            Mat3::from_cols(Vec3::new(a, e, i), Vec3::new(b, f, j), Vec3::new(c, g, k)),
            Vec3::new(d, h, l),
        ))
    }

    #[inline]
    pub fn from_location(loc: Point) -> Transform {
        Transform(Affine3A::from_translation(loc.co.into()))
    }

    /// Returns whether the matrices are approximately equal to each other.
    /// Each corresponding element in the matrices cannot have a relative
    /// error exceeding epsilon.
    #[inline]
    pub fn aprx_eq(&self, other: Transform, epsilon: f32) -> bool {
        let mut eq = true;
        for c in 0..3 {
            for r in 0..3 {
                let a = self.0.matrix3.col(c)[r];
                let b = other.0.matrix3.col(c)[r];
                eq &= relative_eq!(a, b, epsilon = epsilon);
            }
        }
        for i in 0..3 {
            let a = self.0.translation[i];
            let b = other.0.translation[i];
            eq &= relative_eq!(a, b, epsilon = epsilon);
        }
        eq
    }

    /// Returns the inverse of the Matrix
    #[inline]
    pub fn inverse(&self) -> Transform {
        Transform(self.0.inverse())
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

/// Multiply two matrices together
impl Mul for Transform {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self {
        Self(other.0 * self.0)
    }
}

/// Multiply a matrix by a f32
impl Mul<f32> for Transform {
    type Output = Self;

    #[inline]
    fn mul(self, other: f32) -> Self {
        Self(Affine3A::from_mat4(Mat4::from(self.0) * other))
    }
}

/// Add two matrices together
impl Add for Transform {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self(Affine3A::from_mat4(
            Mat4::from(self.0) + Mat4::from(other.0),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_test() {
        let a = Transform::new();
        let b = Transform::new();
        let c =
            Transform::new_from_values(1.1, 0.0, 0.0, 0.0, 0.0, 1.1, 0.0, 0.0, 0.0, 0.0, 1.1, 0.0);

        assert_eq!(a, b);
        assert!(a != c);
    }

    #[test]
    fn approximate_equality_test() {
        let a = Transform::new();
        let b = Transform::new_from_values(
            1.000001, 0.0, 0.0, 0.0, 0.0, 1.000001, 0.0, 0.0, 0.0, 0.0, 1.000001, 0.0,
        );
        let c = Transform::new_from_values(
            1.000003, 0.0, 0.0, 0.0, 0.0, 1.000003, 0.0, 0.0, 0.0, 0.0, 1.000003, 0.0,
        );
        let d = Transform::new_from_values(
            -1.000001, 0.0, 0.0, 0.0, 0.0, -1.000001, 0.0, 0.0, 0.0, 0.0, -1.000001, 0.0,
        );

        assert!(a.aprx_eq(b, 0.000001));
        assert!(!a.aprx_eq(c, 0.000001));
        assert!(!a.aprx_eq(d, 0.000001));
    }

    #[test]
    fn multiply_test() {
        let a = Transform::new_from_values(
            1.0, 2.0, 2.0, 1.5, 3.0, 6.0, 7.0, 8.0, 9.0, 2.0, 11.0, 12.0,
        );
        let b = Transform::new_from_values(
            1.0, 5.0, 9.0, 13.0, 2.0, 6.0, 10.0, 14.0, 3.0, 7.0, 11.0, 15.0,
        );
        let c = Transform::new_from_values(
            97.0, 50.0, 136.0, 162.5, 110.0, 60.0, 156.0, 185.0, 123.0, 70.0, 176.0, 207.5,
        );

        assert_eq!(a * b, c);
    }

    #[test]
    fn inverse_test() {
        let a = Transform::new_from_values(
            1.0, 0.33, 0.0, -2.0, 0.0, 1.0, 0.0, 0.0, 2.1, 0.7, 1.3, 0.0,
        );
        let b = a.inverse();
        let c = Transform::new();

        assert!((dbg!(a * b)).aprx_eq(dbg!(c), 0.0000001));
    }
}
