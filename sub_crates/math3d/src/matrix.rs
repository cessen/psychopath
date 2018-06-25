#![allow(dead_code)]

use std;
use std::ops::{Index, IndexMut, Mul};

use float4::{invert, transpose, Float4};

use super::Point;

/// A 4x4 matrix, used for transforms
#[derive(Debug, Copy, Clone)]
pub struct Matrix4x4 {
    pub values: [Float4; 4],
}

impl Matrix4x4 {
    /// Creates a new identity matrix
    #[inline]
    pub fn new() -> Matrix4x4 {
        Matrix4x4 {
            values: [
                Float4::new(1.0, 0.0, 0.0, 0.0),
                Float4::new(0.0, 1.0, 0.0, 0.0),
                Float4::new(0.0, 0.0, 1.0, 0.0),
                Float4::new(0.0, 0.0, 0.0, 1.0),
            ],
        }
    }

    /// Creates a new matrix with the specified values:
    /// a b c d
    /// e f g h
    /// i j k l
    /// m n o p
    #[inline]
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
        Matrix4x4 {
            values: [
                Float4::new(a, b, c, d),
                Float4::new(e, f, g, h),
                Float4::new(i, j, k, l),
                Float4::new(m, n, o, p),
            ],
        }
    }

    #[inline]
    pub fn from_location(loc: Point) -> Matrix4x4 {
        Matrix4x4 {
            values: [
                Float4::new(1.0, 0.0, 0.0, loc.x()),
                Float4::new(0.0, 1.0, 0.0, loc.y()),
                Float4::new(0.0, 0.0, 1.0, loc.z()),
                Float4::new(0.0, 0.0, 0.0, 1.0),
            ],
        }
    }

    /// Returns whether the matrices are approximately equal to each other.
    /// Each corresponding element in the matrices cannot have a relative error
    /// exceeding `epsilon`.
    #[inline]
    pub fn aprx_eq(&self, other: Matrix4x4, epsilon: f32) -> bool {
        let mut result = true;

        for y in 0..4 {
            for x in 0..4 {
                // All of this stuff is just an approximate comparison
                // of floating point numbers.  See:
                // http://floating-point-gui.de/errors/comparison/
                // It might be worth breaking this out into a separate funcion,
                // but I'm not entirely sure where to put it.
                let a = self[y].get_n(x);
                let b = other[y].get_n(x);
                let aabs = a.abs();
                let babs = b.abs();
                let diff = (a - b).abs();
                if a == b {
                } else if (aabs <= std::f32::EPSILON) || (babs <= std::f32::EPSILON) {
                    result = result && (diff < std::f32::EPSILON);
                } else {
                    let rel = 2.0 * diff / (aabs + babs);
                    println!("{}", rel);
                    result = result && (rel < epsilon);
                }
            }
        }

        return result;
    }

    /// Returns the transpose of the matrix
    #[inline]
    pub fn transposed(&self) -> Matrix4x4 {
        let mut m = *self;
        transpose(&mut m.values);
        m
    }

    /// Returns the inverse of the Matrix
    #[inline]
    pub fn inverse(&self) -> Matrix4x4 {
        let mut m = *self;
        let det = invert(&mut m.values);
        debug_assert_ne!(det, 0.0);
        m
    }
}

impl Index<usize> for Matrix4x4 {
    type Output = Float4;

    #[inline(always)]
    fn index<'a>(&'a self, _index: usize) -> &'a Float4 {
        &self.values[_index]
    }
}

impl IndexMut<usize> for Matrix4x4 {
    #[inline(always)]
    fn index_mut<'a>(&'a mut self, _index: usize) -> &'a mut Float4 {
        &mut self.values[_index]
    }
}

impl PartialEq for Matrix4x4 {
    #[inline]
    fn eq(&self, other: &Matrix4x4) -> bool {
        let mut result = true;

        for y in 0..4 {
            for x in 0..4 {
                result = result && (self[y].get_n(x) == other[y].get_n(x));
            }
        }

        return result;
    }
}

/// Multiply two matrices together
impl Mul<Matrix4x4> for Matrix4x4 {
    type Output = Matrix4x4;

    #[inline]
    fn mul(self, other: Matrix4x4) -> Matrix4x4 {
        let m = self.transposed();
        Matrix4x4 {
            values: [
                Float4::new(
                    (m[0] * other[0]).h_sum(),
                    (m[1] * other[0]).h_sum(),
                    (m[2] * other[0]).h_sum(),
                    (m[3] * other[0]).h_sum(),
                ),
                Float4::new(
                    (m[0] * other[1]).h_sum(),
                    (m[1] * other[1]).h_sum(),
                    (m[2] * other[1]).h_sum(),
                    (m[3] * other[1]).h_sum(),
                ),
                Float4::new(
                    (m[0] * other[2]).h_sum(),
                    (m[1] * other[2]).h_sum(),
                    (m[2] * other[2]).h_sum(),
                    (m[3] * other[2]).h_sum(),
                ),
                Float4::new(
                    (m[0] * other[3]).h_sum(),
                    (m[1] * other[3]).h_sum(),
                    (m[2] * other[3]).h_sum(),
                    (m[3] * other[3]).h_sum(),
                ),
            ],
        }
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
    fn aproximate_equality_test() {
        let a = Matrix4x4::new();
        let b = Matrix4x4::new_from_values(
            1.001, 0.0, 0.0, 0.0, 0.0, 1.001, 0.0, 0.0, 0.0, 0.0, 1.001, 0.0, 0.0, 0.0, 0.0, 1.001,
        );
        let c = Matrix4x4::new_from_values(
            1.003, 0.0, 0.0, 0.0, 0.0, 1.003, 0.0, 0.0, 0.0, 0.0, 1.003, 0.0, 0.0, 0.0, 0.0, 1.003,
        );
        let d = Matrix4x4::new_from_values(
            -1.001, 0.0, 0.0, 0.0, 0.0, -1.001, 0.0, 0.0, 0.0, 0.0, -1.001, 0.0, 0.0, 0.0, 0.0,
            -1.001,
        );

        assert!(a.aprx_eq(b, 0.002));
        assert!(!a.aprx_eq(c, 0.002));
        assert!(!a.aprx_eq(d, 0.002));
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

        assert!((a * b).aprx_eq(c, 0.00001));
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
