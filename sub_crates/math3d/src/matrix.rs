#![allow(dead_code)]

use std::ops::{Add, Mul};

use approx::RelativeEq;
use glam::{Mat4, Vec4};

use super::Point;

/// A 4x4 matrix, used for transforms
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Matrix4x4(pub Mat4);

impl Matrix4x4 {
    /// Creates a new identity matrix
    #[inline]
    pub fn new() -> Matrix4x4 {
        Matrix4x4(Mat4::identity())
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
        m: f32,
        n: f32,
        o: f32,
        p: f32,
    ) -> Matrix4x4 {
        Matrix4x4(Mat4::new(
            Vec4::new(a, e, i, m),
            Vec4::new(b, f, j, n),
            Vec4::new(c, g, k, o),
            Vec4::new(d, h, l, p),
        ))
    }

    #[inline]
    pub fn from_location(loc: Point) -> Matrix4x4 {
        Matrix4x4(Mat4::from_translation(loc.co.truncate()))
    }

    /// Returns whether the matrices are approximately equal to each other.
    /// Each corresponding element in the matrices cannot have a relative
    /// error exceeding epsilon.
    #[inline]
    pub fn aprx_eq(&self, other: Matrix4x4, epsilon: f32) -> bool {
        self.0.relative_eq(&other.0, std::f32::EPSILON, epsilon)
    }

    /// Returns the transpose of the matrix
    #[inline]
    pub fn transposed(&self) -> Matrix4x4 {
        Matrix4x4(self.0.transpose())
    }

    /// Returns the inverse of the Matrix
    #[inline]
    pub fn inverse(&self) -> Matrix4x4 {
        Matrix4x4(self.0.inverse())
    }
}

impl Default for Matrix4x4 {
    fn default() -> Self {
        Self::new()
    }
}

/// Multiply two matrices together
impl Mul for Matrix4x4 {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self {
        Self(other.0.mul_mat4(&self.0))
    }
}

/// Multiply a matrix by a f32
impl Mul<f32> for Matrix4x4 {
    type Output = Self;

    #[inline]
    fn mul(self, other: f32) -> Self {
        Self(self.0 * other)
    }
}

/// Add two matrices together
impl Add for Matrix4x4 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_test() {
        let a = Matrix4x4::new();
        let b = Matrix4x4::new();
        let c = Matrix4x4::new_from_values(
            1.1, 0.0, 0.0, 0.0, 0.0, 1.1, 0.0, 0.0, 0.0, 0.0, 1.1, 0.0, 0.0, 0.0, 0.0, 1.1,
        );

        assert_eq!(a, b);
        assert!(a != c);
    }

    #[test]
    fn approximate_equality_test() {
        let a = Matrix4x4::new();
        let b = Matrix4x4::new_from_values(
            1.000001, 0.0, 0.0, 0.0, 0.0, 1.000001, 0.0, 0.0, 0.0, 0.0, 1.000001, 0.0, 0.0, 0.0,
            0.0, 1.000001,
        );
        let c = Matrix4x4::new_from_values(
            1.000003, 0.0, 0.0, 0.0, 0.0, 1.000003, 0.0, 0.0, 0.0, 0.0, 1.000003, 0.0, 0.0, 0.0,
            0.0, 1.000003,
        );
        let d = Matrix4x4::new_from_values(
            -1.000001, 0.0, 0.0, 0.0, 0.0, -1.000001, 0.0, 0.0, 0.0, 0.0, -1.000001, 0.0, 0.0, 0.0,
            0.0, -1.000001,
        );

        assert!(a.aprx_eq(b, 0.000001));
        assert!(!a.aprx_eq(c, 0.000001));
        assert!(!a.aprx_eq(d, 0.000001));
    }

    #[test]
    fn multiply_test() {
        let a = Matrix4x4::new_from_values(
            1.0, 2.0, 2.0, 1.5, 3.0, 6.0, 7.0, 8.0, 9.0, 2.0, 11.0, 12.0, 13.0, 7.0, 15.0, 3.0,
        );
        let b = Matrix4x4::new_from_values(
            1.0, 5.0, 9.0, 13.0, 2.0, 6.0, 10.0, 14.0, 3.0, 7.0, 11.0, 15.0, 4.0, 8.0, 12.0, 16.0,
        );
        let c = Matrix4x4::new_from_values(
            266.0, 141.0, 331.0, 188.5, 292.0, 158.0, 366.0, 213.0, 318.0, 175.0, 401.0, 237.5,
            344.0, 192.0, 436.0, 262.0,
        );

        assert_eq!(a * b, c);
    }

    #[test]
    fn inverse_test() {
        let a = Matrix4x4::new_from_values(
            1.0, 0.33, 0.0, -2.0, 0.0, 1.0, 0.0, 0.0, 2.1, 0.7, 1.3, 0.0, 0.0, 0.0, 0.0, -1.0,
        );
        let b = a.inverse();
        let c = Matrix4x4::new();

        assert!((dbg!(a * b)).aprx_eq(dbg!(c), 0.0000001));
    }

    #[test]
    fn transpose_test() {
        let a = Matrix4x4::new_from_values(
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        );
        let b = Matrix4x4::new_from_values(
            1.0, 5.0, 9.0, 13.0, 2.0, 6.0, 10.0, 14.0, 3.0, 7.0, 11.0, 15.0, 4.0, 8.0, 12.0, 16.0,
        );
        let c = a.transposed();

        assert_eq!(b, c);
    }
}
