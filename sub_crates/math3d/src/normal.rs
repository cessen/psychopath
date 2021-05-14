#![allow(dead_code)]

use std::{
    cmp::PartialEq,
    ops::{Add, Div, Mul, Neg, Sub},
};

use glam::Vec3A;

use super::{CrossProduct, DotProduct, Transform, Vector};

/// A surface normal in 3d homogeneous space.
#[derive(Debug, Copy, Clone)]
pub struct Normal {
    pub co: Vec3A,
}

impl Normal {
    #[inline(always)]
    pub fn new(x: f32, y: f32, z: f32) -> Normal {
        Normal {
            co: Vec3A::new(x, y, z),
        }
    }

    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.co.length()
    }

    #[inline(always)]
    pub fn length2(&self) -> f32 {
        self.co.length_squared()
    }

    #[inline(always)]
    pub fn normalized(&self) -> Normal {
        Normal {
            co: self.co.normalize(),
        }
    }

    #[inline(always)]
    pub fn into_vector(self) -> Vector {
        Vector { co: self.co }
    }

    #[inline(always)]
    pub fn get_n(&self, n: usize) -> f32 {
        match n {
            0 => self.x(),
            1 => self.y(),
            2 => self.z(),
            _ => panic!("Attempt to access dimension beyond z."),
        }
    }

    #[inline(always)]
    pub fn x(&self) -> f32 {
        self.co[0]
    }

    #[inline(always)]
    pub fn y(&self) -> f32 {
        self.co[1]
    }

    #[inline(always)]
    pub fn z(&self) -> f32 {
        self.co[2]
    }

    #[inline(always)]
    pub fn set_x(&mut self, x: f32) {
        self.co[0] = x;
    }

    #[inline(always)]
    pub fn set_y(&mut self, y: f32) {
        self.co[1] = y;
    }

    #[inline(always)]
    pub fn set_z(&mut self, z: f32) {
        self.co[2] = z;
    }
}

impl PartialEq for Normal {
    #[inline(always)]
    fn eq(&self, other: &Normal) -> bool {
        self.co == other.co
    }
}

impl Add for Normal {
    type Output = Normal;

    #[inline(always)]
    fn add(self, other: Normal) -> Normal {
        Normal {
            co: self.co + other.co,
        }
    }
}

impl Sub for Normal {
    type Output = Normal;

    #[inline(always)]
    fn sub(self, other: Normal) -> Normal {
        Normal {
            co: self.co - other.co,
        }
    }
}

impl Mul<f32> for Normal {
    type Output = Normal;

    #[inline(always)]
    fn mul(self, other: f32) -> Normal {
        Normal {
            co: self.co * other,
        }
    }
}

impl Mul<Transform> for Normal {
    type Output = Normal;

    #[inline]
    fn mul(self, other: Transform) -> Normal {
        Normal {
            co: other.0.matrix3.inverse().transpose().mul_vec3a(self.co),
        }
    }
}

impl Div<f32> for Normal {
    type Output = Normal;

    #[inline(always)]
    fn div(self, other: f32) -> Normal {
        Normal {
            co: self.co / other,
        }
    }
}

impl Neg for Normal {
    type Output = Normal;

    #[inline(always)]
    fn neg(self) -> Normal {
        Normal { co: self.co * -1.0 }
    }
}

impl DotProduct for Normal {
    #[inline(always)]
    fn dot(self, other: Normal) -> f32 {
        self.co.dot(other.co)
    }
}

impl CrossProduct for Normal {
    #[inline]
    fn cross(self, other: Normal) -> Normal {
        Normal {
            co: self.co.cross(other.co),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{CrossProduct, DotProduct, Transform};
    use super::*;
    use approx::assert_ulps_eq;

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
        let m = Transform::new_from_values(
            1.0, 2.0, 2.0, 1.5, 3.0, 6.0, 7.0, 8.0, 9.0, 2.0, 11.0, 12.0,
        );
        let nm = n * m;
        let nm2 = Normal::new(-4.0625, 1.78125, -0.03125);
        for i in 0..3 {
            assert_ulps_eq!(nm.co[i], nm2.co[i], max_ulps = 4);
        }
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
        assert!((n3.x() - n2.x()).abs() < 0.000001);
        assert!((n3.y() - n2.y()).abs() < 0.000001);
        assert!((n3.z() - n2.z()).abs() < 0.000001);
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
}
