#![allow(dead_code)]

use std::ops::{Index, IndexMut, Add, Sub, Mul, Div};
use std::cmp::PartialEq;

use lerp::Lerp;
use float4::Float4;

use super::{DotProduct, CrossProduct};
use super::{Matrix4x4, Vector};

/// A surface normal in 3d homogeneous space.
#[derive(Debug, Copy, Clone)]
pub struct Normal {
    pub co: Float4,
}

impl Normal {
    pub fn new(x: f32, y: f32, z: f32) -> Normal {
        Normal { co: Float4::new(x, y, z, 0.0) }
    }

    pub fn length(&self) -> f32 {
        (self.co * self.co).h_sum().sqrt()
    }

    pub fn length2(&self) -> f32 {
        (self.co * self.co).h_sum()
    }

    pub fn normalized(&self) -> Normal {
        *self / self.length()
    }

    pub fn into_vector(self) -> Vector {
        Vector::new(self.co[0], self.co[1], self.co[2])
    }
}


impl Index<usize> for Normal {
    type Output = f32;

    fn index(&self, index: usize) -> &f32 {
        debug_assert!(index < 3);

        &self.co[index]
    }
}

impl IndexMut<usize> for Normal {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        debug_assert!(index < 3);

        &mut self.co[index]
    }
}


impl PartialEq for Normal {
    fn eq(&self, other: &Normal) -> bool {
        self.co == other.co
    }
}


impl Add for Normal {
    type Output = Normal;

    fn add(self, other: Normal) -> Normal {
        Normal { co: self.co + other.co }
    }
}


impl Sub for Normal {
    type Output = Normal;

    fn sub(self, other: Normal) -> Normal {
        Normal { co: self.co - other.co }
    }
}


impl Mul<f32> for Normal {
    type Output = Normal;

    fn mul(self, other: f32) -> Normal {
        Normal { co: self.co * other }
    }
}

impl Mul<Matrix4x4> for Normal {
    type Output = Normal;

    fn mul(self, other: Matrix4x4) -> Normal {
        let mat = other.inverse().transposed();
        Normal {
            co: Float4::new((self[0] * mat[0][0]) + (self[1] * mat[0][1]) + (self[2] * mat[0][2]),
                            (self[0] * mat[1][0]) + (self[1] * mat[1][1]) + (self[2] * mat[1][2]),
                            (self[0] * mat[2][0]) + (self[1] * mat[2][1]) + (self[2] * mat[2][2]),
                            0.0),
        }
    }
}


impl Div<f32> for Normal {
    type Output = Normal;

    fn div(self, other: f32) -> Normal {
        Normal { co: self.co / other }
    }
}


impl Lerp for Normal {
    fn lerp(self, other: Normal, alpha: f32) -> Normal {
        (self * (1.0 - alpha)) + (other * alpha)
    }
}


impl DotProduct for Normal {
    fn dot(self, other: Normal) -> f32 {
        (self.co * other.co).h_sum()
    }
}


impl CrossProduct for Normal {
    fn cross(self, other: Normal) -> Normal {
        Normal {
            co: Float4::new((self[1] * other[2]) - (self[2] * other[1]),
                            (self[2] * other[0]) - (self[0] * other[2]),
                            (self[0] * other[1]) - (self[1] * other[0]),
                            0.0),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{Matrix4x4, CrossProduct, DotProduct};
    use lerp::Lerp;

    #[test]
    fn add() {
        let v1 = Normal::new(1.0, 2.0, 3.0);
        let v2 = Normal::new(1.5, 4.5, 2.5);
        let v3 = Normal::new(2.5, 6.5, 5.5);

        assert_eq!(v3, v1 + v2);
    }

    #[test]
    fn sub() {
        let v1 = Normal::new(1.0, 2.0, 3.0);
        let v2 = Normal::new(1.5, 4.5, 2.5);
        let v3 = Normal::new(-0.5, -2.5, 0.5);

        assert_eq!(v3, v1 - v2);
    }

    #[test]
    fn mul_scalar() {
        let v1 = Normal::new(1.0, 2.0, 3.0);
        let v2 = 2.0;
        let v3 = Normal::new(2.0, 4.0, 6.0);

        assert_eq!(v3, v1 * v2);
    }

    #[test]
    fn mul_matrix_1() {
        let n = Normal::new(1.0, 2.5, 4.0);
        let m = Matrix4x4::new_from_values(1.0,
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
        let nm = Normal::new(-19.258825, 5.717648, -1.770588);
        assert!(((n * m) - nm).length2() < 0.00001);
    }

    #[test]
    fn div() {
        let v1 = Normal::new(1.0, 2.0, 3.0);
        let v2 = 2.0;
        let v3 = Normal::new(0.5, 1.0, 1.5);

        assert_eq!(v3, v1 / v2);
    }

    #[test]
    fn length() {
        let n = Normal::new(1.0, 2.0, 3.0);
        assert!((n.length() - 3.7416573867739413).abs() < 0.000001);
    }

    #[test]
    fn length2() {
        let n = Normal::new(1.0, 2.0, 3.0);
        assert_eq!(n.length2(), 14.0);
    }

    #[test]
    fn normalized() {
        let n1 = Normal::new(1.0, 2.0, 3.0);
        let n2 = Normal::new(0.2672612419124244, 0.5345224838248488, 0.8017837257372732);
        let n3 = n1.normalized();
        assert!((n3[0] - n2[0]).abs() < 0.000001);
        assert!((n3[1] - n2[1]).abs() < 0.000001);
        assert!((n3[2] - n2[2]).abs() < 0.000001);
    }

    #[test]
    fn dot_test() {
        let v1 = Normal::new(1.0, 2.0, 3.0);
        let v2 = Normal::new(1.5, 4.5, 2.5);
        let v3 = 18.0f32;

        assert_eq!(v3, v1.dot(v2));
    }

    #[test]
    fn cross_test() {
        let v1 = Normal::new(1.0, 0.0, 0.0);
        let v2 = Normal::new(0.0, 1.0, 0.0);
        let v3 = Normal::new(0.0, 0.0, 1.0);

        assert_eq!(v3, v1.cross(v2));
    }

    #[test]
    fn lerp1() {
        let n1 = Normal::new(1.0, 2.0, 1.0);
        let n2 = Normal::new(-2.0, 1.0, -1.0);
        let n3 = Normal::new(1.0, 2.0, 1.0);

        assert_eq!(n3, n1.lerp(n2, 0.0));
    }

    #[test]
    fn lerp2() {
        let n1 = Normal::new(1.0, 2.0, 1.0);
        let n2 = Normal::new(-2.0, 1.0, -1.0);
        let n3 = Normal::new(-2.0, 1.0, -1.0);

        assert_eq!(n3, n1.lerp(n2, 1.0));
    }

    #[test]
    fn lerp3() {
        let n1 = Normal::new(1.0, 2.0, 1.0);
        let n2 = Normal::new(-2.0, 1.0, -1.0);
        let n3 = Normal::new(-0.5, 1.5, 0.0);

        assert_eq!(n3, n1.lerp(n2, 0.5));
    }
}
