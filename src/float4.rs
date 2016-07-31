#![allow(dead_code)]

use std::ops::{Add, Sub, Mul, Div};
use std::cmp::PartialEq;

#[cfg(feature = "simd_perf")]
use simd::f32x4;

/// Essentially a tuple of four floats, which will use SIMD operations
/// where possible on a platform.
#[cfg(feature = "simd_perf")]
#[derive(Debug, Copy, Clone)]
pub struct Float4 {
    data: f32x4,
}

#[cfg(not(feature = "simd_perf"))]
#[derive(Debug, Copy, Clone)]
pub struct Float4 {
    data: [f32; 4],
}

impl Float4 {
    #[cfg(feature = "simd_perf")]
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Float4 {
        Float4 { data: f32x4::new(a, b, c, d) }
    }
    #[cfg(not(feature = "simd_perf"))]
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Float4 {
        Float4 { data: [a, b, c, d] }
    }

    #[cfg(feature = "simd_perf")]
    pub fn splat(n: f32) -> Float4 {
        Float4 { data: f32x4::splat(n) }
    }
    #[cfg(not(feature = "simd_perf"))]
    pub fn splat(n: f32) -> Float4 {
        Float4 { data: [n, n, n, n] }
    }

    pub fn h_sum(&self) -> f32 {
        (self.get_0() + self.get_1()) + (self.get_2() + self.get_3())
    }

    pub fn h_product(&self) -> f32 {
        (self.get_0() * self.get_1()) * (self.get_2() * self.get_3())
    }

    pub fn h_min(&self) -> f32 {
        let n1 = if self.get_0() < self.get_1() {
            self.get_0()
        } else {
            self.get_1()
        };
        let n2 = if self.get_2() < self.get_3() {
            self.get_2()
        } else {
            self.get_3()
        };
        if n1 < n2 {
            n1
        } else {
            n2
        }
    }

    pub fn h_max(&self) -> f32 {
        let n1 = if self.get_0() > self.get_1() {
            self.get_0()
        } else {
            self.get_1()
        };
        let n2 = if self.get_2() > self.get_3() {
            self.get_2()
        } else {
            self.get_3()
        };
        if n1 > n2 {
            n1
        } else {
            n2
        }
    }

    #[cfg(feature = "simd_perf")]
    pub fn v_min(&self, other: Float4) -> Float4 {
        Float4 { data: self.data.min(other.data) }
    }
    #[cfg(not(feature = "simd_perf"))]
    pub fn v_min(&self, other: Float4) -> Float4 {
        Float4::new(if self.get_0() < other.get_0() {
                        self.get_0()
                    } else {
                        other.get_0()
                    },
                    if self.get_1() < other.get_1() {
                        self.get_1()
                    } else {
                        other.get_1()
                    },
                    if self.get_2() < other.get_2() {
                        self.get_2()
                    } else {
                        other.get_2()
                    },
                    if self.get_3() < other.get_3() {
                        self.get_3()
                    } else {
                        other.get_3()
                    })

    }

    #[cfg(feature = "simd_perf")]
    pub fn v_max(&self, other: Float4) -> Float4 {
        Float4 { data: self.data.max(other.data) }
    }
    #[cfg(not(feature = "simd_perf"))]
    pub fn v_max(&self, other: Float4) -> Float4 {
        Float4::new(if self.get_0() > other.get_0() {
                        self.get_0()
                    } else {
                        other.get_0()
                    },
                    if self.get_1() > other.get_1() {
                        self.get_1()
                    } else {
                        other.get_1()
                    },
                    if self.get_2() > other.get_2() {
                        self.get_2()
                    } else {
                        other.get_2()
                    },
                    if self.get_3() > other.get_3() {
                        self.get_3()
                    } else {
                        other.get_3()
                    })
    }

    /// Set the nth element to the given value.
    #[inline]
    pub fn set_n(&mut self, n: usize, v: f32) {
        match n {
            0 => self.set_0(v),
            1 => self.set_1(v),
            2 => self.set_2(v),
            3 => self.set_3(v),
            _ => panic!("Attempted to set element of Float4 outside of bounds."),
        }
    }

    /// Set the 0th element to the given value.
    #[cfg(feature = "simd_perf")]
    #[inline(always)]
    pub fn set_0(&mut self, n: f32) {
        self.data = self.data.replace(0, n);
    }
    #[inline(always)]
    #[cfg(not(feature = "simd_perf"))]
    pub fn set_0(&mut self, n: f32) {
        unsafe {
            *self.data.get_unchecked_mut(0) = n;
        }
    }

    /// Set the 1th element to the given value.
    #[cfg(feature = "simd_perf")]
    #[inline(always)]
    pub fn set_1(&mut self, n: f32) {
        self.data = self.data.replace(1, n);
    }
    #[cfg(not(feature = "simd_perf"))]
    #[inline(always)]
    pub fn set_1(&mut self, n: f32) {
        unsafe {
            *self.data.get_unchecked_mut(1) = n;
        }
    }

    /// Set the 2th element to the given value.
    #[cfg(feature = "simd_perf")]
    #[inline(always)]
    pub fn set_2(&mut self, n: f32) {
        self.data = self.data.replace(2, n);
    }
    #[cfg(not(feature = "simd_perf"))]
    #[inline(always)]
    pub fn set_2(&mut self, n: f32) {
        unsafe {
            *self.data.get_unchecked_mut(2) = n;
        }
    }

    /// Set the 3th element to the given value.
    #[cfg(feature = "simd_perf")]
    #[inline(always)]
    pub fn set_3(&mut self, n: f32) {
        self.data = self.data.replace(3, n);
    }
    #[cfg(not(feature = "simd_perf"))]
    #[inline(always)]
    pub fn set_3(&mut self, n: f32) {
        unsafe {
            *self.data.get_unchecked_mut(3) = n;
        }
    }

    /// Returns the value of the nth element.
    #[inline]
    pub fn get_n(&self, n: usize) -> f32 {
        match n {
            0 => self.get_0(),
            1 => self.get_1(),
            2 => self.get_2(),
            3 => self.get_3(),
            _ => panic!("Attempted to access element of Float4 outside of bounds."),
        }
    }

    /// Returns the value of the 0th element.
    #[cfg(feature = "simd_perf")]
    #[inline(always)]
    pub fn get_0(&self) -> f32 {
        self.data.extract(0)
    }
    #[cfg(not(feature = "simd_perf"))]
    #[inline(always)]
    pub fn get_0(&self) -> f32 {
        unsafe { *self.data.get_unchecked(0) }
    }

    /// Returns the value of the 1th element.
    #[cfg(feature = "simd_perf")]
    #[inline(always)]
    pub fn get_1(&self) -> f32 {
        self.data.extract(1)
    }
    #[cfg(not(feature = "simd_perf"))]
    #[inline(always)]
    pub fn get_1(&self) -> f32 {
        unsafe { *self.data.get_unchecked(1) }
    }

    /// Returns the value of the 2th element.
    #[cfg(feature = "simd_perf")]
    #[inline(always)]
    pub fn get_2(&self) -> f32 {
        self.data.extract(2)
    }
    #[cfg(not(feature = "simd_perf"))]
    #[inline(always)]
    pub fn get_2(&self) -> f32 {
        unsafe { *self.data.get_unchecked(2) }
    }

    /// Returns the value of the 3th element.
    #[cfg(feature = "simd_perf")]
    #[inline(always)]
    pub fn get_3(&self) -> f32 {
        self.data.extract(3)
    }
    #[cfg(not(feature = "simd_perf"))]
    #[inline(always)]
    pub fn get_3(&self) -> f32 {
        unsafe { *self.data.get_unchecked(3) }
    }
}


impl PartialEq for Float4 {
    fn eq(&self, other: &Float4) -> bool {
        self.get_0() == other.get_0() && self.get_1() == other.get_1() &&
        self.get_2() == other.get_2() && self.get_3() == other.get_3()
    }
}


impl Add for Float4 {
    type Output = Float4;

    #[cfg(feature = "simd_perf")]
    fn add(self, other: Float4) -> Float4 {
        Float4 { data: self.data + other.data }
    }
    #[cfg(not(feature = "simd_perf"))]
    fn add(self, other: Float4) -> Float4 {
        Float4 {
            data: [self.get_0() + other.get_0(),
                   self.get_1() + other.get_1(),
                   self.get_2() + other.get_2(),
                   self.get_3() + other.get_3()],
        }
    }
}


impl Sub for Float4 {
    type Output = Float4;

    #[cfg(feature = "simd_perf")]
    fn sub(self, other: Float4) -> Float4 {
        Float4 { data: self.data - other.data }
    }
    #[cfg(not(feature = "simd_perf"))]
    fn sub(self, other: Float4) -> Float4 {
        Float4 {
            data: [self.get_0() - other.get_0(),
                   self.get_1() - other.get_1(),
                   self.get_2() - other.get_2(),
                   self.get_3() - other.get_3()],
        }
    }
}


impl Mul for Float4 {
    type Output = Float4;

    #[cfg(feature = "simd_perf")]
    fn mul(self, other: Float4) -> Float4 {
        Float4 { data: self.data * other.data }
    }
    #[cfg(not(feature = "simd_perf"))]
    fn mul(self, other: Float4) -> Float4 {
        Float4 {
            data: [self.get_0() * other.get_0(),
                   self.get_1() * other.get_1(),
                   self.get_2() * other.get_2(),
                   self.get_3() * other.get_3()],
        }
    }
}

impl Mul<f32> for Float4 {
    type Output = Float4;

    #[cfg(feature = "simd_perf")]
    fn mul(self, other: f32) -> Float4 {
        Float4 { data: self.data * f32x4::splat(other) }
    }
    #[cfg(not(feature = "simd_perf"))]
    fn mul(self, other: f32) -> Float4 {
        Float4 {
            data: [self.get_0() * other,
                   self.get_1() * other,
                   self.get_2() * other,
                   self.get_3() * other],
        }
    }
}


impl Div for Float4 {
    type Output = Float4;

    #[cfg(feature = "simd_perf")]
    fn div(self, other: Float4) -> Float4 {
        Float4 { data: self.data / other.data }
    }
    #[cfg(not(feature = "simd_perf"))]
    fn div(self, other: Float4) -> Float4 {
        Float4 {
            data: [self.get_0() / other.get_0(),
                   self.get_1() / other.get_1(),
                   self.get_2() / other.get_2(),
                   self.get_3() / other.get_3()],
        }
    }
}

impl Div<f32> for Float4 {
    type Output = Float4;

    #[cfg(feature = "simd_perf")]
    fn div(self, other: f32) -> Float4 {
        Float4 { data: self.data / f32x4::splat(other) }
    }
    #[cfg(not(feature = "simd_perf"))]
    fn div(self, other: f32) -> Float4 {
        Float4 {
            data: [self.get_0() / other,
                   self.get_1() / other,
                   self.get_2() / other,
                   self.get_3() / other],
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get() {
        let f = Float4::new(1.0, 2.0, 3.0, 4.0);

        assert_eq!(f.get_0(), 1.0);
        assert_eq!(f.get_1(), 2.0);
        assert_eq!(f.get_2(), 3.0);
        assert_eq!(f.get_3(), 4.0);
    }

    #[test]
    fn get_n() {
        let f = Float4::new(1.0, 2.0, 3.0, 4.0);

        assert_eq!(f.get_n(0), 1.0);
        assert_eq!(f.get_n(1), 2.0);
        assert_eq!(f.get_n(2), 3.0);
        assert_eq!(f.get_n(3), 4.0);
    }

    #[test]
    fn set() {
        let mut f = Float4::new(1.0, 2.0, 3.0, 4.0);
        f.set_0(5.0);
        f.set_1(6.0);
        f.set_2(7.0);
        f.set_3(8.0);

        assert_eq!(f.get_0(), 5.0);
        assert_eq!(f.get_1(), 6.0);
        assert_eq!(f.get_2(), 7.0);
        assert_eq!(f.get_3(), 8.0);
    }

    #[test]
    fn set_n() {
        let mut f = Float4::new(1.0, 2.0, 3.0, 4.0);
        f.set_n(0, 5.0);
        f.set_n(1, 6.0);
        f.set_n(2, 7.0);
        f.set_n(3, 8.0);

        assert_eq!(f.get_0(), 5.0);
        assert_eq!(f.get_1(), 6.0);
        assert_eq!(f.get_2(), 7.0);
        assert_eq!(f.get_3(), 8.0);
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
