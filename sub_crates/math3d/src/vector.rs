#![allow(dead_code)]

use std::{
    cmp::PartialEq,
    ops::{Add, Div, Mul, Neg, Sub},
};

use glam::Vec3A;

use super::{CrossProduct, DotProduct, Normal, Point, Transform};

/// A direction vector in 3d homogeneous space.
#[derive(Debug, Copy, Clone)]
pub struct Vector {
    pub co: Vec3A,
}

impl Vector {
    #[inline(always)]
    pub fn new(x: f32, y: f32, z: f32) -> Vector {
        Vector {
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
    pub fn normalized(&self) -> Vector {
        Vector {
            co: self.co.normalize(),
        }
    }

    #[inline(always)]
    pub fn abs(&self) -> Vector {
        Vector {
            co: self.co * self.co.signum(),
        }
    }

    #[inline(always)]
    pub fn into_point(self) -> Point {
        Point { co: self.co }
    }

    #[inline(always)]
    pub fn into_normal(self) -> Normal {
        Normal { co: self.co }
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

impl PartialEq for Vector {
    #[inline(always)]
    fn eq(&self, other: &Vector) -> bool {
        self.co == other.co
    }
}

impl Add for Vector {
    type Output = Vector;

    #[inline(always)]
    fn add(self, other: Vector) -> Vector {
        Vector {
            co: self.co + other.co,
        }
    }
}

impl Sub for Vector {
    type Output = Vector;

    #[inline(always)]
    fn sub(self, other: Vector) -> Vector {
        Vector {
            co: self.co - other.co,
        }
    }
}

impl Mul<f32> for Vector {
    type Output = Vector;

    #[inline(always)]
    fn mul(self, other: f32) -> Vector {
        Vector {
            co: self.co * other,
        }
    }
}

impl Mul<Transform> for Vector {
    type Output = Vector;

    #[inline]
    fn mul(self, other: Transform) -> Vector {
        Vector {
            co: other.0.transform_vector3a(self.co),
        }
    }
}

impl Div<f32> for Vector {
    type Output = Vector;

    #[inline(always)]
    fn div(self, other: f32) -> Vector {
        Vector {
            co: self.co / other,
        }
    }
}

impl Neg for Vector {
    type Output = Vector;

    #[inline(always)]
    fn neg(self) -> Vector {
        Vector { co: self.co * -1.0 }
    }
}

impl DotProduct for Vector {
    #[inline(always)]
    fn dot(self, other: Vector) -> f32 {
        self.co.dot(other.co)
    }
}

impl CrossProduct for Vector {
    #[inline]
    fn cross(self, other: Vector) -> Vector {
        Vector {
            co: self.co.cross(other.co),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{CrossProduct, DotProduct, Transform};
    use super::*;

    #[test]
    fn add() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = Vector::new(1.5, 4.5, 2.5);
        let v3 = Vector::new(2.5, 6.5, 5.5);

        assert_eq!(v3, v1 + v2);
    }

    #[test]
    fn sub() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = Vector::new(1.5, 4.5, 2.5);
        let v3 = Vector::new(-0.5, -2.5, 0.5);

        assert_eq!(v3, v1 - v2);
    }

    #[test]
    fn mul_scalar() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = 2.0;
        let v3 = Vector::new(2.0, 4.0, 6.0);

        assert_eq!(v3, v1 * v2);
    }

    #[test]
    fn mul_matrix_1() {
        let v = Vector::new(1.0, 2.5, 4.0);
        let m = Transform::new_from_values(
            1.0, 2.0, 2.0, 1.5, 3.0, 6.0, 7.0, 8.0, 9.0, 2.0, 11.0, 12.0,
        );
        assert_eq!(v * m, Vector::new(14.0, 46.0, 58.0));
    }

    #[test]
    fn mul_matrix_2() {
        let v = Vector::new(1.0, 2.5, 4.0);
        let m = Transform::new_from_values(
            1.0, 2.0, 2.0, 1.5, 3.0, 6.0, 7.0, 8.0, 9.0, 2.0, 11.0, 12.0,
        );
        assert_eq!(v * m, Vector::new(14.0, 46.0, 58.0));
    }

    #[test]
    fn div() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = 2.0;
        let v3 = Vector::new(0.5, 1.0, 1.5);

        assert_eq!(v3, v1 / v2);
    }

    #[test]
    fn length() {
        let v = Vector::new(1.0, 2.0, 3.0);
        assert!((v.length() - 3.7416573867739413).abs() < 0.000001);
    }

    #[test]
    fn length2() {
        let v = Vector::new(1.0, 2.0, 3.0);
        assert_eq!(v.length2(), 14.0);
    }

    #[test]
    fn normalized() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = Vector::new(0.2672612419124244, 0.5345224838248488, 0.8017837257372732);
        let v3 = v1.normalized();
        assert!((v3.x() - v2.x()).abs() < 0.000001);
        assert!((v3.y() - v2.y()).abs() < 0.000001);
        assert!((v3.z() - v2.z()).abs() < 0.000001);
    }

    #[test]
    fn dot_test() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = Vector::new(1.5, 4.5, 2.5);
        let v3 = 18.0f32;

        assert_eq!(v3, v1.dot(v2));
    }

    #[test]
    fn cross_test() {
        let v1 = Vector::new(1.0, 0.0, 0.0);
        let v2 = Vector::new(0.0, 1.0, 0.0);
        let v3 = Vector::new(0.0, 0.0, 1.0);

        assert_eq!(v3, v1.cross(v2));
    }
}
