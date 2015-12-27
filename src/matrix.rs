#![allow(dead_code)]

use std;
use std::ops::{Index, IndexMut, Mul};

use float4::Float4;
use lerp::Lerp;




/// A 4x4 matrix, used for transforms
#[derive(Debug, Copy, Clone)]
pub struct Matrix4x4 {
    values: [Float4; 4],
}


impl Matrix4x4 {
    /// Creates a new identity matrix
    pub fn new() -> Matrix4x4 {
        Matrix4x4 {
            values: [Float4::new(1.0, 0.0, 0.0, 0.0),
                     Float4::new(0.0, 1.0, 0.0, 0.0),
                     Float4::new(0.0, 0.0, 1.0, 0.0),
                     Float4::new(0.0, 0.0, 0.0, 1.0)],
        }
    }

    /// Creates a new matrix with the specified values:
    /// a b c d
    /// e f g h
    /// i j k l
    /// m n o p
    pub fn new_from_values(a: f32,
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
                           p: f32)
                           -> Matrix4x4 {
        Matrix4x4 {
            values: [Float4::new(a, b, c, d),
                     Float4::new(e, f, g, h),
                     Float4::new(i, j, k, l),
                     Float4::new(m, n, o, p)],
        }
    }

    /// Returns whether the matrices are approximately equal to each other.
    /// Each corresponding element in the matrices cannot have a relative error
    /// exceeding `epsilon`.
    pub fn aprx_eq(&self, other: Matrix4x4, epsilon: f32) -> bool {
        let mut result = true;

        for y in 0..4 {
            for x in 0..4 {
                // All of this stuff is just an approximate comparison
                // of floating point numbers.  See:
                // http://floating-point-gui.de/errors/comparison/
                // It might be worth breaking this out into a separate funcion,
                // but I'm not entirely sure where to put it.
                let a = self[y][x];
                let b = other[y][x];
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
    pub fn transposed(&self) -> Matrix4x4 {
        Matrix4x4 {
            values: {
                [Float4::new(self[0][0], self[1][0], self[2][0], self[3][0]),
                 Float4::new(self[0][1], self[1][1], self[2][1], self[3][1]),
                 Float4::new(self[0][2], self[1][2], self[2][2], self[3][2]),
                 Float4::new(self[0][3], self[1][3], self[2][3], self[3][3])]
            },
        }
    }


    /// Returns the inverse of the Matrix
    pub fn inverse(&self) -> Matrix4x4 {
        let s0 = (self[0][0] * self[1][1]) - (self[1][0] * self[0][1]);
        let s1 = (self[0][0] * self[1][2]) - (self[1][0] * self[0][2]);
        let s2 = (self[0][0] * self[1][3]) - (self[1][0] * self[0][3]);
        let s3 = (self[0][1] * self[1][2]) - (self[1][1] * self[0][2]);
        let s4 = (self[0][1] * self[1][3]) - (self[1][1] * self[0][3]);
        let s5 = (self[0][2] * self[1][3]) - (self[1][2] * self[0][3]);

        let c5 = (self[2][2] * self[3][3]) - (self[3][2] * self[2][3]);
        let c4 = (self[2][1] * self[3][3]) - (self[3][1] * self[2][3]);
        let c3 = (self[2][1] * self[3][2]) - (self[3][1] * self[2][2]);
        let c2 = (self[2][0] * self[3][3]) - (self[3][0] * self[2][3]);
        let c1 = (self[2][0] * self[3][2]) - (self[3][0] * self[2][2]);
        let c0 = (self[2][0] * self[3][1]) - (self[3][0] * self[2][1]);

        // TODO: handle 0.0 determinant
        let det = (s0 * c5) - (s1 * c4) + (s2 * c3) + (s3 * c2) - (s4 * c1) + (s5 * c0);
        let invdet = 1.0 / det;

        Matrix4x4 {
            values: {
                [Float4::new(((self[1][1] * c5) - (self[1][2] * c4) + (self[1][3] * c3)) * invdet,
                             ((-self[0][1] * c5) + (self[0][2] * c4) - (self[0][3] * c3)) * invdet,
                             ((self[3][1] * s5) - (self[3][2] * s4) + (self[3][3] * s3)) * invdet,
                             ((-self[2][1] * s5) + (self[2][2] * s4) - (self[2][3] * s3)) * invdet),

                 Float4::new(((-self[1][0] * c5) + (self[1][2] * c2) - (self[1][3] * c1)) * invdet,
                             ((self[0][0] * c5) - (self[0][2] * c2) + (self[0][3] * c1)) * invdet,
                             ((-self[3][0] * s5) + (self[3][2] * s2) - (self[3][3] * s1)) * invdet,
                             ((self[2][0] * s5) - (self[2][2] * s2) + (self[2][3] * s1)) * invdet),

                 Float4::new(((self[1][0] * c4) - (self[1][1] * c2) + (self[1][3] * c0)) * invdet,
                             ((-self[0][0] * c4) + (self[0][1] * c2) - (self[0][3] * c0)) * invdet,
                             ((self[3][0] * s4) - (self[3][1] * s2) + (self[3][3] * s0)) * invdet,
                             ((-self[2][0] * s4) + (self[2][1] * s2) - (self[2][3] * s0)) * invdet),

                 Float4::new(((-self[1][0] * c3) + (self[1][1] * c1) - (self[1][2] * c0)) * invdet,
                             ((self[0][0] * c3) - (self[0][1] * c1) + (self[0][2] * c0)) * invdet,
                             ((-self[3][0] * s3) + (self[3][1] * s1) - (self[3][2] * s0)) * invdet,
                             ((self[2][0] * s3) - (self[2][1] * s1) + (self[2][2] * s0)) * invdet)]
            },
        }
    }
}


impl Index<usize> for Matrix4x4 {
    type Output = Float4;

    fn index<'a>(&'a self, _index: usize) -> &'a Float4 {
        &self.values[_index]
    }
}


impl IndexMut<usize> for Matrix4x4 {
    fn index_mut<'a>(&'a mut self, _index: usize) -> &'a mut Float4 {
        &mut self.values[_index]
    }
}


impl PartialEq for Matrix4x4 {
    fn eq(&self, other: &Matrix4x4) -> bool {
        let mut result = true;

        for y in 0..4 {
            for x in 0..4 {
                result = result && (self[y][x] == other[y][x]);
            }
        }

        return result;
    }
}


/// Multiply two matrices together
impl Mul<Matrix4x4> for Matrix4x4 {
    type Output = Matrix4x4;

    fn mul(self, other: Matrix4x4) -> Matrix4x4 {
        let m = self.transposed();
        Matrix4x4 {
            values: [Float4::new((m[0] * other[0]).h_sum(),
                                 (m[1] * other[0]).h_sum(),
                                 (m[2] * other[0]).h_sum(),
                                 (m[3] * other[0]).h_sum()),

                     Float4::new((m[0] * other[1]).h_sum(),
                                 (m[1] * other[1]).h_sum(),
                                 (m[2] * other[1]).h_sum(),
                                 (m[3] * other[1]).h_sum()),

                     Float4::new((m[0] * other[2]).h_sum(),
                                 (m[1] * other[2]).h_sum(),
                                 (m[2] * other[2]).h_sum(),
                                 (m[3] * other[2]).h_sum()),

                     Float4::new((m[0] * other[3]).h_sum(),
                                 (m[1] * other[3]).h_sum(),
                                 (m[2] * other[3]).h_sum(),
                                 (m[3] * other[3]).h_sum())],
        }
    }
}


impl Lerp for Matrix4x4 {
    fn lerp(self, other: Matrix4x4, alpha: f32) -> Matrix4x4 {
        let alpha_minus = 1.0 - alpha;
        Matrix4x4 {
            values: [Float4::new((self[0][0] * alpha_minus) + (other[0][0] * alpha),
                                 (self[0][1] * alpha_minus) + (other[0][1] * alpha),
                                 (self[0][2] * alpha_minus) + (other[0][2] * alpha),
                                 (self[0][3] * alpha_minus) + (other[0][3] * alpha)),

                     Float4::new((self[1][0] * alpha_minus) + (other[1][0] * alpha),
                                 (self[1][1] * alpha_minus) + (other[1][1] * alpha),
                                 (self[1][2] * alpha_minus) + (other[1][2] * alpha),
                                 (self[1][3] * alpha_minus) + (other[1][3] * alpha)),

                     Float4::new((self[2][0] * alpha_minus) + (other[2][0] * alpha),
                                 (self[2][1] * alpha_minus) + (other[2][1] * alpha),
                                 (self[2][2] * alpha_minus) + (other[2][2] * alpha),
                                 (self[2][3] * alpha_minus) + (other[2][3] * alpha)),

                     Float4::new((self[3][0] * alpha_minus) + (other[3][0] * alpha),
                                 (self[3][1] * alpha_minus) + (other[3][1] * alpha),
                                 (self[3][2] * alpha_minus) + (other[3][2] * alpha),
                                 (self[3][3] * alpha_minus) + (other[3][3] * alpha))],
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use lerp::Lerp;

    #[test]
    fn equality_test() {
        let a = Matrix4x4::new();
        let b = Matrix4x4::new();
        let c = Matrix4x4::new_from_values(1.1,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.1,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.1,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.1);

        assert_eq!(a, b);
        assert!(a != c);
    }

    #[test]
    fn aproximate_equality_test() {
        let a = Matrix4x4::new();
        let b = Matrix4x4::new_from_values(1.001,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.001,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.001,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.001);
        let c = Matrix4x4::new_from_values(1.003,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.003,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.003,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.003);
        let d = Matrix4x4::new_from_values(-1.001,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           -1.001,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           -1.001,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           -1.001);

        assert!(a.aprx_eq(b, 0.002));
        assert!(!a.aprx_eq(c, 0.002));
        assert!(!a.aprx_eq(d, 0.002));
    }

    #[test]
    fn multiply_test() {
        let a = Matrix4x4::new_from_values(1.0,
                                           2.0,
                                           2.0,
                                           1.5,
                                           3.0,
                                           6.0,
                                           7.0,
                                           8.0,
                                           9.0,
                                           2.0,
                                           11.0,
                                           12.0,
                                           13.0,
                                           7.0,
                                           15.0,
                                           3.0);
        let b = Matrix4x4::new_from_values(1.0,
                                           5.0,
                                           9.0,
                                           13.0,
                                           2.0,
                                           6.0,
                                           10.0,
                                           14.0,
                                           3.0,
                                           7.0,
                                           11.0,
                                           15.0,
                                           4.0,
                                           8.0,
                                           12.0,
                                           16.0);
        let c = Matrix4x4::new_from_values(266.0,
                                           141.0,
                                           331.0,
                                           188.5,
                                           292.0,
                                           158.0,
                                           366.0,
                                           213.0,
                                           318.0,
                                           175.0,
                                           401.0,
                                           237.5,
                                           344.0,
                                           192.0,
                                           436.0,
                                           262.0);

        assert_eq!(a * b, c);
    }

    #[test]
    fn inverse_test() {
        let a = Matrix4x4::new_from_values(1.0,
                                           0.33,
                                           0.0,
                                           -2.0,
                                           0.0,
                                           1.0,
                                           0.0,
                                           0.0,
                                           2.1,
                                           0.7,
                                           1.3,
                                           0.0,
                                           0.0,
                                           0.0,
                                           0.0,
                                           -1.0);
        let b = a.inverse();
        let c = Matrix4x4::new();

        assert!((a * b).aprx_eq(c, 0.00001));
    }

    #[test]
    fn transpose_test() {
        let a = Matrix4x4::new_from_values(1.0,
                                           2.0,
                                           3.0,
                                           4.0,
                                           5.0,
                                           6.0,
                                           7.0,
                                           8.0,
                                           9.0,
                                           10.0,
                                           11.0,
                                           12.0,
                                           13.0,
                                           14.0,
                                           15.0,
                                           16.0);
        let b = Matrix4x4::new_from_values(1.0,
                                           5.0,
                                           9.0,
                                           13.0,
                                           2.0,
                                           6.0,
                                           10.0,
                                           14.0,
                                           3.0,
                                           7.0,
                                           11.0,
                                           15.0,
                                           4.0,
                                           8.0,
                                           12.0,
                                           16.0);
        let c = a.transposed();

        assert_eq!(b, c);
    }

    #[test]
    fn lerp_test() {
        let a = Matrix4x4::new_from_values(0.0,
                                           2.0,
                                           2.0,
                                           3.0,
                                           4.0,
                                           5.0,
                                           6.0,
                                           7.0,
                                           8.0,
                                           9.0,
                                           10.0,
                                           11.0,
                                           12.0,
                                           13.0,
                                           14.0,
                                           15.0);
        let b = Matrix4x4::new_from_values(-1.0,
                                           1.0,
                                           3.0,
                                           4.0,
                                           5.0,
                                           6.0,
                                           7.0,
                                           8.0,
                                           9.0,
                                           10.0,
                                           11.0,
                                           12.0,
                                           13.0,
                                           14.0,
                                           15.0,
                                           16.0);

        let c1 = Matrix4x4::new_from_values(-0.25,
                                            1.75,
                                            2.25,
                                            3.25,
                                            4.25,
                                            5.25,
                                            6.25,
                                            7.25,
                                            8.25,
                                            9.25,
                                            10.25,
                                            11.25,
                                            12.25,
                                            13.25,
                                            14.25,
                                            15.25);
        let c2 = Matrix4x4::new_from_values(-0.5,
                                            1.5,
                                            2.5,
                                            3.5,
                                            4.5,
                                            5.5,
                                            6.5,
                                            7.5,
                                            8.5,
                                            9.5,
                                            10.5,
                                            11.5,
                                            12.5,
                                            13.5,
                                            14.5,
                                            15.5);
        let c3 = Matrix4x4::new_from_values(-0.75,
                                            1.25,
                                            2.75,
                                            3.75,
                                            4.75,
                                            5.75,
                                            6.75,
                                            7.75,
                                            8.75,
                                            9.75,
                                            10.75,
                                            11.75,
                                            12.75,
                                            13.75,
                                            14.75,
                                            15.75);

        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 0.25), c1);
        assert_eq!(a.lerp(b, 0.5), c2);
        assert_eq!(a.lerp(b, 0.75), c3);
        assert_eq!(a.lerp(b, 1.0), b);
    }
}
