#![allow(dead_code)]

use std::ops::{Index, IndexMut, Add, Sub, Mul, Div};
use std::cmp::PartialEq;

use lerp::Lerp;
use math::{DotProduct, CrossProduct};
use float4::Float4;

/// A direction vector in 3d homogeneous space.
#[derive(Debug, Copy, Clone)]
pub struct Vector {
    co: Float4,
}

impl Vector {
    pub fn new(x: f32, y: f32, z: f32) -> Vector {
        Vector { co: Float4::new(x, y, z, 0.0) }
    }
}


impl Index<usize> for Vector {
    type Output = f32;

    fn index(&self, index: usize) -> &f32 {
        debug_assert!(index < 3);

        &self.co[index]
    }
}

impl IndexMut<usize> for Vector {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        debug_assert!(index < 3);

        &mut self.co[index]
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


impl Div<f32> for Vector {
    type Output = Vector;

    fn div(self, other: f32) -> Vector {
        Vector { co: self.co / other }
    }
}


impl Lerp for Vector {
    fn lerp(self, other: Vector, alpha: f32) -> Vector {
        self + ((other - self) * alpha)
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
    use math::*;

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
    fn mul() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = 2.0;
        let v3 = Vector::new(2.0, 4.0, 6.0);

        assert_eq!(v3, v1 * v2);
    }

    #[test]
    fn div() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = 2.0;
        let v3 = Vector::new(0.5, 1.0, 1.5);

        assert_eq!(v3, v1 / v2);
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
