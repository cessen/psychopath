#![allow(dead_code)]

use std::{
    cmp::PartialEq,
    ops::{Add, Mul, Sub},
};

use glam::Vec3A;

use super::{Transform, Vector};

/// A position in 3d homogeneous space.
#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub co: Vec3A,
}

impl Point {
    #[inline(always)]
    pub fn new(x: f32, y: f32, z: f32) -> Point {
        Point {
            co: Vec3A::new(x, y, z),
        }
    }

    #[inline(always)]
    pub fn min(&self, other: Point) -> Point {
        let n1 = self;
        let n2 = other;

        Point {
            co: n1.co.min(n2.co),
        }
    }

    #[inline(always)]
    pub fn max(&self, other: Point) -> Point {
        let n1 = self;
        let n2 = other;

        Point {
            co: n1.co.max(n2.co),
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

impl PartialEq for Point {
    #[inline(always)]
    fn eq(&self, other: &Point) -> bool {
        self.co == other.co
    }
}

impl Add<Vector> for Point {
    type Output = Point;

    #[inline(always)]
    fn add(self, other: Vector) -> Point {
        Point {
            co: self.co + other.co,
        }
    }
}

impl Sub for Point {
    type Output = Vector;

    #[inline(always)]
    fn sub(self, other: Point) -> Vector {
        Vector {
            co: self.co - other.co,
        }
    }
}

impl Sub<Vector> for Point {
    type Output = Point;

    #[inline(always)]
    fn sub(self, other: Vector) -> Point {
        Point {
            co: self.co - other.co,
        }
    }
}

impl Mul<Transform> for Point {
    type Output = Point;

    #[inline]
    fn mul(self, other: Transform) -> Point {
        Point {
            co: other.0.transform_point3a(self.co),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Transform, Vector};
    use super::*;

    #[test]
    fn add() {
        let p1 = Point::new(1.0, 2.0, 3.0);
        let v1 = Vector::new(1.5, 4.5, 2.5);
        let p2 = Point::new(2.5, 6.5, 5.5);

        assert_eq!(p2, p1 + v1);
    }

    #[test]
    fn sub() {
        let p1 = Point::new(1.0, 2.0, 3.0);
        let p2 = Point::new(1.5, 4.5, 2.5);
        let v1 = Vector::new(-0.5, -2.5, 0.5);

        assert_eq!(v1, p1 - p2);
    }

    #[test]
    fn mul_matrix_1() {
        let p = Point::new(1.0, 2.5, 4.0);
        let m = Transform::new_from_values(
            1.0, 2.0, 2.0, 1.5, 3.0, 6.0, 7.0, 8.0, 9.0, 2.0, 11.0, 12.0,
        );
        let pm = Point::new(15.5, 54.0, 70.0);
        assert_eq!(p * m, pm);
    }

    #[test]
    fn mul_matrix_2() {
        let p = Point::new(1.0, 2.5, 4.0);
        let m = Transform::new_from_values(
            1.0, 2.0, 2.0, 1.5, 3.0, 6.0, 7.0, 8.0, 9.0, 2.0, 11.0, 12.0,
        );
        let pm = Point::new(15.5, 54.0, 70.0);
        assert_eq!(p * m, pm);
    }

    #[test]
    fn mul_matrix_3() {
        // Make sure matrix multiplication composes the way one would expect
        let p = Point::new(1.0, 2.5, 4.0);
        let m1 = Transform::new_from_values(
            1.0, 2.0, 2.0, 1.5, 3.0, 6.0, 7.0, 8.0, 9.0, 2.0, 11.0, 12.0,
        );
        let m2 =
            Transform::new_from_values(4.0, 1.0, 2.0, 3.5, 3.0, 6.0, 5.0, 2.0, 2.0, 2.0, 4.0, 12.0);
        println!("{:?}", m1 * m2);

        let pmm1 = p * (m1 * m2);
        let pmm2 = (p * m1) * m2;

        assert!((pmm1 - pmm2).length2() <= 0.00001); // Assert pmm1 and pmm2 are roughly equal
    }
}
