#![allow(dead_code)]

use std::ops::{Index, IndexMut, Add, Sub, Mul, Div};
use std::cmp::PartialEq;

/// Essentially a tuple of four floats, which will use SIMD operations
/// where possible on a platform.
#[derive(Debug, Copy, Clone)]
pub struct Float4 {
    data: [f32; 4],
}

impl Float4 {
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Float4 {
        Float4 { data: [a, b, c, d] }
    }

    pub fn h_sum(&self) -> f32 {
        unsafe {
            *self.data.get_unchecked(0) + *self.data.get_unchecked(1) +
            *self.data.get_unchecked(2) + *self.data.get_unchecked(3)
        }
    }

    pub fn h_product(&self) -> f32 {
        unsafe {
            *self.data.get_unchecked(0) * *self.data.get_unchecked(1) *
            *self.data.get_unchecked(2) * *self.data.get_unchecked(3)
        }
    }

    pub fn h_min(&self) -> f32 {
        unsafe {
            self.data
                .get_unchecked(0)
                .min(*self.data.get_unchecked(1))
                .min(self.data.get_unchecked(2).min(*self.data.get_unchecked(3)))
        }
    }

    pub fn h_max(&self) -> f32 {
        unsafe {
            self.data
                .get_unchecked(0)
                .max(*self.data.get_unchecked(1))
                .max(self.data.get_unchecked(2).max(*self.data.get_unchecked(3)))
        }
    }

    pub fn v_min(&self, other: Float4) -> Float4 {
        unsafe {
            Float4::new(self.data.get_unchecked(0).min(*other.data.get_unchecked(0)),
                        self.data.get_unchecked(1).min(*other.data.get_unchecked(1)),
                        self.data.get_unchecked(2).min(*other.data.get_unchecked(2)),
                        self.data.get_unchecked(3).min(*other.data.get_unchecked(3)))
        }
    }

    pub fn v_max(&self, other: Float4) -> Float4 {
        unsafe {
            Float4::new(self.data.get_unchecked(0).max(*other.data.get_unchecked(0)),
                        self.data.get_unchecked(1).max(*other.data.get_unchecked(1)),
                        self.data.get_unchecked(2).max(*other.data.get_unchecked(2)),
                        self.data.get_unchecked(3).max(*other.data.get_unchecked(3)))
        }
    }

    pub fn set_0(&mut self, n: f32) {
        unsafe {
            *self.data.get_unchecked_mut(0) = n;
        }
    }

    pub fn set_1(&mut self, n: f32) {
        unsafe {
            *self.data.get_unchecked_mut(1) = n;
        }
    }

    pub fn set_2(&mut self, n: f32) {
        unsafe {
            *self.data.get_unchecked_mut(2) = n;
        }
    }

    pub fn set_3(&mut self, n: f32) {
        unsafe {
            *self.data.get_unchecked_mut(3) = n;
        }
    }
}


impl Index<usize> for Float4 {
    type Output = f32;

    fn index(&self, index: usize) -> &f32 {
        &self.data[index]
    }
}

impl IndexMut<usize> for Float4 {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.data[index]
    }
}


impl PartialEq for Float4 {
    fn eq(&self, other: &Float4) -> bool {
        unsafe {
            *self.data.get_unchecked(0) == *other.data.get_unchecked(0) &&
            *self.data.get_unchecked(1) == *other.data.get_unchecked(1) &&
            *self.data.get_unchecked(2) == *other.data.get_unchecked(2) &&
            *self.data.get_unchecked(3) == *other.data.get_unchecked(3)
        }
    }
}


impl Add for Float4 {
    type Output = Float4;

    fn add(self, other: Float4) -> Float4 {
        unsafe {
            Float4 {
                data: [*self.data.get_unchecked(0) + *other.data.get_unchecked(0),
                       *self.data.get_unchecked(1) + *other.data.get_unchecked(1),
                       *self.data.get_unchecked(2) + *other.data.get_unchecked(2),
                       *self.data.get_unchecked(3) + *other.data.get_unchecked(3)],
            }
        }
    }
}


impl Sub for Float4 {
    type Output = Float4;

    fn sub(self, other: Float4) -> Float4 {
        unsafe {
            Float4 {
                data: [*self.data.get_unchecked(0) - *other.data.get_unchecked(0),
                       *self.data.get_unchecked(1) - *other.data.get_unchecked(1),
                       *self.data.get_unchecked(2) - *other.data.get_unchecked(2),
                       *self.data.get_unchecked(3) - *other.data.get_unchecked(3)],
            }
        }
    }
}


impl Mul for Float4 {
    type Output = Float4;

    fn mul(self, other: Float4) -> Float4 {
        unsafe {
            Float4 {
                data: [*self.data.get_unchecked(0) * *other.data.get_unchecked(0),
                       *self.data.get_unchecked(1) * *other.data.get_unchecked(1),
                       *self.data.get_unchecked(2) * *other.data.get_unchecked(2),
                       *self.data.get_unchecked(3) * *other.data.get_unchecked(3)],
            }
        }
    }
}

impl Mul<f32> for Float4 {
    type Output = Float4;

    fn mul(self, other: f32) -> Float4 {
        unsafe {
            Float4 {
                data: [*self.data.get_unchecked(0) * other,
                       *self.data.get_unchecked(1) * other,
                       *self.data.get_unchecked(2) * other,
                       *self.data.get_unchecked(3) * other],
            }
        }
    }
}


impl Div for Float4 {
    type Output = Float4;

    fn div(self, other: Float4) -> Float4 {
        unsafe {
            Float4 {
                data: [*self.data.get_unchecked(0) / *other.data.get_unchecked(0),
                       *self.data.get_unchecked(1) / *other.data.get_unchecked(1),
                       *self.data.get_unchecked(2) / *other.data.get_unchecked(2),
                       *self.data.get_unchecked(3) / *other.data.get_unchecked(3)],
            }
        }
    }
}

impl Div<f32> for Float4 {
    type Output = Float4;

    fn div(self, other: f32) -> Float4 {
        unsafe {
            Float4 {
                data: [*self.data.get_unchecked(0) / other,
                       *self.data.get_unchecked(1) / other,
                       *self.data.get_unchecked(2) / other,
                       *self.data.get_unchecked(3) / other],
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index() {
        let f = Float4::new(1.0, 2.0, 3.0, 4.0);

        assert_eq!(f[0], 1.0);
        assert_eq!(f[1], 2.0);
        assert_eq!(f[2], 3.0);
        assert_eq!(f[3], 4.0);
    }

    #[test]
    fn index_mut() {
        let mut f = Float4::new(1.0, 2.0, 3.0, 4.0);
        f[0] = 5.0;
        f[1] = 6.0;
        f[2] = 7.0;
        f[3] = 8.0;

        assert_eq!(f[0], 5.0);
        assert_eq!(f[1], 6.0);
        assert_eq!(f[2], 7.0);
        assert_eq!(f[3], 8.0);
    }

    #[test]
    fn partial_eq_1() {
        let f1 = Float4::new(1.0, 2.0, 3.0, 4.0);
        let f2 = Float4::new(1.0, 2.0, 3.0, 4.0);

        assert!(f1 == f2);
    }

    #[test]
    fn partial_eq_2() {
        let f1 = Float4::new(1.0, 2.0, 3.0, 4.0);
        let f2 = Float4::new(1.0, 2.1, 3.0, 4.0);

        assert!(!(f1 == f2));
    }

    #[test]
    fn h_sum() {
        let f = Float4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(f.h_sum(), 10.0);
    }

    #[test]
    fn h_product() {
        let f = Float4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(f.h_product(), 24.0);
    }

    #[test]
    fn h_min() {
        let f = Float4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(f.h_min(), 1.0);
    }

    #[test]
    fn h_max() {
        let f = Float4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(f.h_max(), 4.0);
    }

    #[test]
    fn add() {
        let f1 = Float4::new(1.0, 2.0, 3.0, 4.0);
        let f2 = Float4::new(2.0, 3.0, 4.0, 5.0);
        let f3 = Float4::new(3.0, 5.0, 7.0, 9.0);

        assert_eq!(f1 + f2, f3);
    }

    #[test]
    fn sub() {
        let f1 = Float4::new(1.0, 2.0, 3.0, 4.0);
        let f2 = Float4::new(2.0, 3.0, 4.0, 5.0);
        let f3 = Float4::new(-1.0, -1.0, -1.0, -1.0);

        assert_eq!(f1 - f2, f3);
    }

    #[test]
    fn mul_component() {
        let f1 = Float4::new(1.0, 2.0, 3.0, 4.0);
        let f2 = Float4::new(2.0, 3.0, 4.0, 5.0);
        let f3 = Float4::new(2.0, 6.0, 12.0, 20.0);

        assert_eq!(f1 * f2, f3);
    }

    #[test]
    fn mul_scalar() {
        let f1 = Float4::new(1.0, 2.0, 3.0, 4.0);
        let v = 3.0;
        let f2 = Float4::new(3.0, 6.0, 9.0, 12.0);

        assert_eq!(f1 * v, f2);
    }

    #[test]
    fn div_component() {
        let f1 = Float4::new(1.0, 3.0, 3.0, 6.0);
        let f2 = Float4::new(2.0, 2.0, 4.0, 8.0);
        let f3 = Float4::new(0.5, 1.5, 0.75, 0.75);

        assert_eq!(f1 / f2, f3);
    }

    #[test]
    fn div_scalar() {
        let f1 = Float4::new(1.0, 2.0, 3.0, 4.0);
        let v = 2.0;
        let f2 = Float4::new(0.5, 1.0, 1.5, 2.0);

        assert_eq!(f1 / v, f2);
    }
}
