#![allow(dead_code)]

use std::ops::{Add, Sub, Mul, Div, Neg};
use std::cmp::PartialEq;

use lerp::Lerp;
use float4::Float4;

use super::{DotProduct, CrossProduct};
use super::{Matrix4x4, Normal};

/// A direction vector in 3d homogeneous space.
#[derive(Debug, Copy, Clone)]
pub struct Vector {
    pub co: Float4,
}

impl Vector {
    pub fn new(x: f32, y: f32, z: f32) -> Vector {
        Vector { co: Float4::new(x, y, z, 0.0) }
    }

    pub fn length(&self) -> f32 {
        (self.co * self.co).h_sum().sqrt()
    }

    pub fn length2(&self) -> f32 {
        (self.co * self.co).h_sum()
    }

    pub fn normalized(&self) -> Vector {
        *self / self.length()
    }

    pub fn into_normal(self) -> Normal {
        Normal::new(self.x(), self.y(), self.z())
    }

    pub fn get_n(&self, n: usize) -> f32 {
        match n {
            0 => self.x(),
            1 => self.y(),
            2 => self.z(),
            _ => panic!("Attempt to access dimension beyond z."),
        }
    }

    pub fn x(&self) -> f32 {
        self.co.get_0()
    }

    pub fn y(&self) -> f32 {
        self.co.get_1()
    }

    pub fn z(&self) -> f32 {
        self.co.get_2()
    }

    pub fn set_x(&mut self, x: f32) {
        self.co.set_0(x);
    }

    pub fn set_y(&mut self, y: f32) {
        self.co.set_1(y);
    }

    pub fn set_z(&mut self, z: f32) {
        self.co.set_2(z);
    }
}


impl PartialEq for Vector {
    fn eq(&self, other: &Vector) -> bool {
        self.co == other.co
    }
}


impl Add for Vector {
    type Output = Vector;

    fn add(self, other: Vector) -> Vector {
        Vector { co: self.co + other.co }
    }
}


impl Sub for Vector {
    type Output = Vector;

    fn sub(self, other: Vector) -> Vector {
        Vector { co: self.co - other.co }
    }
}


impl Mul<f32> for Vector {
    type Output = Vector;

    fn mul(self, other: f32) -> Vector {
        Vector { co: self.co * other }
    }
}


impl Mul<Matrix4x4> for Vector {
    type Output = Vector;

    fn mul(self, other: Matrix4x4) -> Vector {
        Vector {
            co: Float4::new((self.co * other[0]).h_sum(),
                            (self.co * other[1]).h_sum(),
                            (self.co * other[2]).h_sum(),
                            (self.co * other[3]).h_sum()),
        }
    }
}


impl Div<f32> for Vector {
    type Output = Vector;

    fn div(self, other: f32) -> Vector {
        Vector { co: self.co / other }
    }
}


impl Neg for Vector {
    type Output = Vector;

    fn neg(self) -> Vector {
        Vector { co: self.co * -1.0 }
    }
}


impl Lerp for Vector {
    fn lerp(self, other: Vector, alpha: f32) -> Vector {
        (self * (1.0 - alpha)) + (other * alpha)
    }
}


impl DotProduct for Vector {
    fn dot(self, other: Vector) -> f32 {
        (self.co * other.co).h_sum()
    }
}


impl CrossProduct for Vector {
    fn cross(self, other: Vector) -> Vector {
        Vector {
            co: Float4::new((self.co.get_1() * other.co.get_2()) -
                            (self.co.get_2() * other.co.get_1()),
                            (self.co.get_2() * other.co.get_0()) -
                            (self.co.get_0() * other.co.get_2()),
                            (self.co.get_0() * other.co.get_1()) -
                            (self.co.get_1() * other.co.get_0()),
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
        let mut vm = Vector::new(14.0, 46.0, 58.0);
        vm.co.set_3(90.5);
        assert_eq!(v * m, vm);
    }

    #[test]
    fn mul_matrix_2() {
        let v = Vector::new(1.0, 2.5, 4.0);
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
                                           0.0,
                                           0.0,
                                           0.0,
                                           1.0);
        let vm = Vector::new(14.0, 46.0, 58.0);
        assert_eq!(v * m, vm);
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

    #[test]
    fn lerp1() {
        let v1 = Vector::new(1.0, 2.0, 1.0);
        let v2 = Vector::new(-2.0, 1.0, -1.0);
        let v3 = Vector::new(1.0, 2.0, 1.0);

        assert_eq!(v3, v1.lerp(v2, 0.0));
    }

    #[test]
    fn lerp2() {
        let v1 = Vector::new(1.0, 2.0, 1.0);
        let v2 = Vector::new(-2.0, 1.0, -1.0);
        let v3 = Vector::new(-2.0, 1.0, -1.0);

        assert_eq!(v3, v1.lerp(v2, 1.0));
    }

    #[test]
    fn lerp3() {
        let v1 = Vector::new(1.0, 2.0, 1.0);
        let v2 = Vector::new(-2.0, 1.0, -1.0);
        let v3 = Vector::new(-0.5, 1.5, 0.0);

        assert_eq!(v3, v1.lerp(v2, 0.5));
    }
}
