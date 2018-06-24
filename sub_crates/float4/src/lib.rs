#![allow(dead_code)]

/// Implementation of Float4 for x86_64 platforms with sse support
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
mod x86_64_sse {
    use std::arch::x86_64::__m128;
    use std::cmp::PartialEq;
    use std::ops::{Add, AddAssign, BitAnd, BitOr, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

    #[derive(Debug, Copy, Clone)]
    pub struct Float4 {
        data: __m128,
    }

    impl Float4 {
        #[inline(always)]
        pub fn new(a: f32, b: f32, c: f32, d: f32) -> Float4 {
            use std::arch::x86_64::_mm_set_ps;
            Float4 {
                data: unsafe { _mm_set_ps(d, c, b, a) },
            }
        }

        #[inline(always)]
        pub fn splat(n: f32) -> Float4 {
            use std::arch::x86_64::_mm_set1_ps;
            Float4 {
                data: unsafe { _mm_set1_ps(n) },
            }
        }

        #[inline]
        pub fn h_sum(&self) -> f32 {
            (self.get_0() + self.get_1()) + (self.get_2() + self.get_3())
        }

        #[inline]
        pub fn h_product(&self) -> f32 {
            (self.get_0() * self.get_1()) * (self.get_2() * self.get_3())
        }

        #[inline]
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

        #[inline]
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

        #[inline(always)]
        pub fn v_min(&self, other: Float4) -> Float4 {
            use std::arch::x86_64::_mm_min_ps;
            Float4 {
                data: unsafe { _mm_min_ps(self.data, other.data) },
            }
        }

        #[inline(always)]
        pub fn v_max(&self, other: Float4) -> Float4 {
            use std::arch::x86_64::_mm_max_ps;
            Float4 {
                data: unsafe { _mm_max_ps(self.data, other.data) },
            }
        }

        #[inline(always)]
        pub fn lt(&self, other: Float4) -> Bool4 {
            use std::arch::x86_64::_mm_cmplt_ps;
            Bool4 {
                data: unsafe { _mm_cmplt_ps(self.data, other.data) },
            }
        }

        #[inline(always)]
        pub fn lte(&self, other: Float4) -> Bool4 {
            use std::arch::x86_64::_mm_cmple_ps;
            Bool4 {
                data: unsafe { _mm_cmple_ps(self.data, other.data) },
            }
        }

        #[inline(always)]
        pub fn gt(&self, other: Float4) -> Bool4 {
            use std::arch::x86_64::_mm_cmpgt_ps;
            Bool4 {
                data: unsafe { _mm_cmpgt_ps(self.data, other.data) },
            }
        }

        #[inline(always)]
        pub fn gte(&self, other: Float4) -> Bool4 {
            use std::arch::x86_64::_mm_cmpge_ps;
            Bool4 {
                data: unsafe { _mm_cmpge_ps(self.data, other.data) },
            }
        }

        /// Set the nth element to the given value.
        #[inline(always)]
        pub fn set_n(&mut self, n: usize, v: f32) {
            use std::mem::transmute;
            assert!(
                n <= 3,
                "Attempted to set element of Float4 outside of bounds."
            );

            unsafe { *transmute::<*mut __m128, *mut f32>(&mut self.data).offset(n as isize) = v }
        }

        /// Set the 0th element to the given value.
        #[inline(always)]
        pub fn set_0(&mut self, v: f32) {
            self.set_n(0, v);
        }

        /// Set the 1th element to the given value.
        #[inline(always)]
        pub fn set_1(&mut self, v: f32) {
            self.set_n(1, v);
        }

        /// Set the 2th element to the given value.
        #[inline(always)]
        pub fn set_2(&mut self, v: f32) {
            self.set_n(2, v);
        }

        /// Set the 3th element to the given value.
        #[inline(always)]
        pub fn set_3(&mut self, v: f32) {
            self.set_n(3, v);
        }

        /// Returns the value of the nth element.
        #[inline(always)]
        pub fn get_n(&self, n: usize) -> f32 {
            use std::mem::transmute;
            assert!(
                n <= 3,
                "Attempted to access element of Float4 outside of bounds."
            );

            unsafe { *transmute::<*const __m128, *const f32>(&self.data).offset(n as isize) }
        }

        /// Returns the value of the 0th element.
        #[inline(always)]
        pub fn get_0(&self) -> f32 {
            self.get_n(0)
        }

        /// Returns the value of the 1th element.
        #[inline(always)]
        pub fn get_1(&self) -> f32 {
            self.get_n(1)
        }

        /// Returns the value of the 2th element.
        #[inline(always)]
        pub fn get_2(&self) -> f32 {
            self.get_n(2)
        }

        /// Returns the value of the 3th element.
        #[inline(always)]
        pub fn get_3(&self) -> f32 {
            self.get_n(3)
        }
    }

    impl PartialEq for Float4 {
        #[inline]
        fn eq(&self, other: &Float4) -> bool {
            self.get_0() == other.get_0()
                && self.get_1() == other.get_1()
                && self.get_2() == other.get_2()
                && self.get_3() == other.get_3()
        }
    }

    impl Add for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn add(self, other: Float4) -> Float4 {
            use std::arch::x86_64::_mm_add_ps;
            Float4 {
                data: unsafe { _mm_add_ps(self.data, other.data) },
            }
        }
    }

    impl AddAssign for Float4 {
        #[inline(always)]
        fn add_assign(&mut self, rhs: Float4) {
            *self = *self + rhs;
        }
    }

    impl Sub for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn sub(self, other: Float4) -> Float4 {
            use std::arch::x86_64::_mm_sub_ps;
            Float4 {
                data: unsafe { _mm_sub_ps(self.data, other.data) },
            }
        }
    }

    impl SubAssign for Float4 {
        #[inline(always)]
        fn sub_assign(&mut self, rhs: Float4) {
            *self = *self - rhs;
        }
    }

    impl Mul for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn mul(self, other: Float4) -> Float4 {
            use std::arch::x86_64::_mm_mul_ps;
            Float4 {
                data: unsafe { _mm_mul_ps(self.data, other.data) },
            }
        }
    }

    impl Mul<f32> for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn mul(self, other: f32) -> Float4 {
            self * Float4::splat(other)
        }
    }

    impl MulAssign for Float4 {
        #[inline(always)]
        fn mul_assign(&mut self, rhs: Float4) {
            *self = *self * rhs;
        }
    }

    impl MulAssign<f32> for Float4 {
        #[inline(always)]
        fn mul_assign(&mut self, rhs: f32) {
            *self = *self * rhs;
        }
    }

    impl Div for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn div(self, other: Float4) -> Float4 {
            use std::arch::x86_64::_mm_div_ps;
            Float4 {
                data: unsafe { _mm_div_ps(self.data, other.data) },
            }
        }
    }

    impl Div<f32> for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn div(self, other: f32) -> Float4 {
            self / Float4::splat(other)
        }
    }

    impl DivAssign for Float4 {
        #[inline(always)]
        fn div_assign(&mut self, rhs: Float4) {
            *self = *self / rhs;
        }
    }

    impl DivAssign<f32> for Float4 {
        #[inline(always)]
        fn div_assign(&mut self, rhs: f32) {
            *self = *self / rhs;
        }
    }

    #[inline(always)]
    pub fn v_min(a: Float4, b: Float4) -> Float4 {
        a.v_min(b)
    }

    #[inline(always)]
    pub fn v_max(a: Float4, b: Float4) -> Float4 {
        a.v_max(b)
    }

    /// Essentially a tuple of four bools, which will use SIMD operations
    /// where possible on a platform.
    #[derive(Debug, Copy, Clone)]
    pub struct Bool4 {
        data: __m128,
    }

    impl Bool4 {
        /// Returns the value of the nth element.
        #[inline(always)]
        pub fn get_n(&self, n: usize) -> bool {
            use std::mem::transmute;
            assert!(
                n <= 3,
                "Attempted to access element of Bool4 outside of bounds."
            );

            0 != unsafe { *transmute::<*const __m128, *const u32>(&self.data).offset(n as isize) }
        }

        /// Returns the value of the 0th element.
        #[inline(always)]
        pub fn get_0(&self) -> bool {
            self.get_n(0)
        }

        /// Returns the value of the 1th element.
        #[inline(always)]
        pub fn get_1(&self) -> bool {
            self.get_n(1)
        }

        /// Returns the value of the 2th element.
        #[inline(always)]
        pub fn get_2(&self) -> bool {
            self.get_n(2)
        }

        /// Returns the value of the 3th element.
        #[inline(always)]
        pub fn get_3(&self) -> bool {
            self.get_n(3)
        }

        #[inline]
        pub fn to_bitmask(&self) -> u8 {
            use std::mem::transmute;
            let a = unsafe { *transmute::<*const __m128, *const u8>(&self.data).offset(0) };
            let b = unsafe { *transmute::<*const __m128, *const u8>(&self.data).offset(4) };
            let c = unsafe { *transmute::<*const __m128, *const u8>(&self.data).offset(8) };
            let d = unsafe { *transmute::<*const __m128, *const u8>(&self.data).offset(12) };
            (a & 0b00000001) | (b & 0b00000010) | (c & 0b00000100) | (d & 0b00001000)
        }
    }

    impl BitAnd for Bool4 {
        type Output = Bool4;

        #[inline(always)]
        fn bitand(self, rhs: Bool4) -> Bool4 {
            use std::arch::x86_64::_mm_and_ps;
            Bool4 {
                data: unsafe { _mm_and_ps(self.data, rhs.data) },
            }
        }
    }

    impl BitOr for Bool4 {
        type Output = Bool4;

        #[inline(always)]
        fn bitor(self, rhs: Bool4) -> Bool4 {
            use std::arch::x86_64::_mm_or_ps;
            Bool4 {
                data: unsafe { _mm_or_ps(self.data, rhs.data) },
            }
        }
    }
}

//===========================================================================

/// Implementation fo Float4 for any platform, foregoing any
/// platform-specific optimizations.
mod fallback {
    use std::cmp::PartialEq;
    use std::ops::{Add, AddAssign, BitAnd, BitOr, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

    #[derive(Debug, Copy, Clone)]
    pub struct Float4 {
        data: [f32; 4],
    }

    impl Float4 {
        #[inline(always)]
        pub fn new(a: f32, b: f32, c: f32, d: f32) -> Float4 {
            Float4 { data: [a, b, c, d] }
        }

        #[inline(always)]
        pub fn splat(n: f32) -> Float4 {
            Float4 { data: [n, n, n, n] }
        }

        #[inline]
        pub fn h_sum(&self) -> f32 {
            (self.get_0() + self.get_1()) + (self.get_2() + self.get_3())
        }

        #[inline]
        pub fn h_product(&self) -> f32 {
            (self.get_0() * self.get_1()) * (self.get_2() * self.get_3())
        }

        #[inline]
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

        #[inline]
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

        #[inline(always)]
        pub fn v_min(&self, other: Float4) -> Float4 {
            Float4::new(
                if self.get_0() < other.get_0() {
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
                },
            )
        }

        #[inline(always)]
        pub fn v_max(&self, other: Float4) -> Float4 {
            Float4::new(
                if self.get_0() > other.get_0() {
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
                },
            )
        }

        #[inline(always)]
        pub fn lt(&self, other: Float4) -> Bool4 {
            Bool4 {
                data: [
                    self.data[0] < other.data[0],
                    self.data[1] < other.data[1],
                    self.data[2] < other.data[2],
                    self.data[3] < other.data[3],
                ],
            }
        }

        #[inline(always)]
        pub fn lte(&self, other: Float4) -> Bool4 {
            Bool4 {
                data: [
                    self.data[0] <= other.data[0],
                    self.data[1] <= other.data[1],
                    self.data[2] <= other.data[2],
                    self.data[3] <= other.data[3],
                ],
            }
        }

        #[inline(always)]
        pub fn gt(&self, other: Float4) -> Bool4 {
            Bool4 {
                data: [
                    self.data[0] > other.data[0],
                    self.data[1] > other.data[1],
                    self.data[2] > other.data[2],
                    self.data[3] > other.data[3],
                ],
            }
        }

        #[inline(always)]
        pub fn gte(&self, other: Float4) -> Bool4 {
            Bool4 {
                data: [
                    self.data[0] >= other.data[0],
                    self.data[1] >= other.data[1],
                    self.data[2] >= other.data[2],
                    self.data[3] >= other.data[3],
                ],
            }
        }

        /// Set the nth element to the given value.
        #[inline(always)]
        pub fn set_n(&mut self, n: usize, v: f32) {
            assert!(
                n <= 3,
                "Attempted to set element of Float4 outside of bounds."
            );
            unsafe {
                *self.data.get_unchecked_mut(n) = v;
            }
        }

        /// Set the 0th element to the given value.
        #[inline(always)]
        pub fn set_0(&mut self, v: f32) {
            self.set_n(0, v);
        }

        /// Set the 1th element to the given value.
        #[inline(always)]
        pub fn set_1(&mut self, v: f32) {
            self.set_n(1, v);
        }

        /// Set the 2th element to the given value.
        #[inline(always)]
        pub fn set_2(&mut self, v: f32) {
            self.set_n(2, v);
        }

        /// Set the 3th element to the given value.
        #[inline(always)]
        pub fn set_3(&mut self, v: f32) {
            self.set_n(3, v);
        }

        /// Returns the value of the nth element.
        #[inline(always)]
        pub fn get_n(&self, n: usize) -> f32 {
            assert!(
                n <= 3,
                "Attempted to access element of Float4 outside of bounds."
            );
            unsafe { *self.data.get_unchecked(n) }
        }

        /// Returns the value of the 0th element.
        #[inline(always)]
        pub fn get_0(&self) -> f32 {
            self.get_n(0)
        }

        /// Returns the value of the 1th element.
        #[inline(always)]
        pub fn get_1(&self) -> f32 {
            self.get_n(1)
        }

        /// Returns the value of the 2th element.
        #[inline(always)]
        pub fn get_2(&self) -> f32 {
            self.get_n(2)
        }

        /// Returns the value of the 3th element.
        #[inline(always)]
        pub fn get_3(&self) -> f32 {
            self.get_n(3)
        }
    }

    impl PartialEq for Float4 {
        #[inline]
        fn eq(&self, other: &Float4) -> bool {
            self.get_0() == other.get_0()
                && self.get_1() == other.get_1()
                && self.get_2() == other.get_2()
                && self.get_3() == other.get_3()
        }
    }

    impl Add for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn add(self, other: Float4) -> Float4 {
            Float4 {
                data: [
                    self.get_0() + other.get_0(),
                    self.get_1() + other.get_1(),
                    self.get_2() + other.get_2(),
                    self.get_3() + other.get_3(),
                ],
            }
        }
    }

    impl AddAssign for Float4 {
        #[inline(always)]
        fn add_assign(&mut self, rhs: Float4) {
            *self = *self + rhs;
        }
    }

    impl Sub for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn sub(self, other: Float4) -> Float4 {
            Float4 {
                data: [
                    self.get_0() - other.get_0(),
                    self.get_1() - other.get_1(),
                    self.get_2() - other.get_2(),
                    self.get_3() - other.get_3(),
                ],
            }
        }
    }

    impl SubAssign for Float4 {
        #[inline(always)]
        fn sub_assign(&mut self, rhs: Float4) {
            *self = *self - rhs;
        }
    }

    impl Mul for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn mul(self, other: Float4) -> Float4 {
            Float4 {
                data: [
                    self.get_0() * other.get_0(),
                    self.get_1() * other.get_1(),
                    self.get_2() * other.get_2(),
                    self.get_3() * other.get_3(),
                ],
            }
        }
    }

    impl Mul<f32> for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn mul(self, other: f32) -> Float4 {
            Float4 {
                data: [
                    self.get_0() * other,
                    self.get_1() * other,
                    self.get_2() * other,
                    self.get_3() * other,
                ],
            }
        }
    }

    impl MulAssign for Float4 {
        #[inline(always)]
        fn mul_assign(&mut self, rhs: Float4) {
            *self = *self * rhs;
        }
    }

    impl MulAssign<f32> for Float4 {
        #[inline(always)]
        fn mul_assign(&mut self, rhs: f32) {
            *self = *self * rhs;
        }
    }

    impl Div for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn div(self, other: Float4) -> Float4 {
            Float4 {
                data: [
                    self.get_0() / other.get_0(),
                    self.get_1() / other.get_1(),
                    self.get_2() / other.get_2(),
                    self.get_3() / other.get_3(),
                ],
            }
        }
    }

    impl Div<f32> for Float4 {
        type Output = Float4;

        #[inline(always)]
        fn div(self, other: f32) -> Float4 {
            Float4 {
                data: [
                    self.get_0() / other,
                    self.get_1() / other,
                    self.get_2() / other,
                    self.get_3() / other,
                ],
            }
        }
    }

    impl DivAssign for Float4 {
        #[inline(always)]
        fn div_assign(&mut self, rhs: Float4) {
            *self = *self / rhs;
        }
    }

    impl DivAssign<f32> for Float4 {
        #[inline(always)]
        fn div_assign(&mut self, rhs: f32) {
            *self = *self / rhs;
        }
    }

    #[inline(always)]
    pub fn v_min(a: Float4, b: Float4) -> Float4 {
        a.v_min(b)
    }

    #[inline(always)]
    pub fn v_max(a: Float4, b: Float4) -> Float4 {
        a.v_max(b)
    }

    /// Essentially a tuple of four bools, which will use SIMD operations
    /// where possible on a platform.
    #[cfg(feature = "simd_perf")]
    #[derive(Debug, Copy, Clone)]
    pub struct Bool4 {
        data: bool32fx4,
    }

    #[cfg(not(feature = "simd_perf"))]
    #[derive(Debug, Copy, Clone)]
    pub struct Bool4 {
        data: [bool; 4],
    }

    impl Bool4 {
        /// Returns the value of the nth element.
        #[inline(always)]
        pub fn get_n(&self, n: usize) -> bool {
            assert!(
                n <= 3,
                "Attempted to access element of Bool4 outside of bounds."
            );
            unsafe { *self.data.get_unchecked(n) }
        }

        /// Returns the value of the 0th element.
        #[inline(always)]
        pub fn get_0(&self) -> bool {
            self.get_n(0)
        }

        /// Returns the value of the 1th element.
        #[inline(always)]
        pub fn get_1(&self) -> bool {
            self.get_n(1)
        }

        /// Returns the value of the 2th element.
        #[inline(always)]
        pub fn get_2(&self) -> bool {
            self.get_n(2)
        }

        /// Returns the value of the 3th element.
        #[inline(always)]
        pub fn get_3(&self) -> bool {
            self.get_n(3)
        }

        #[inline]
        pub fn to_bitmask(&self) -> u8 {
            (self.get_0() as u8)
                | ((self.get_1() as u8) << 1)
                | ((self.get_2() as u8) << 2)
                | ((self.get_3() as u8) << 3)
        }
    }

    impl BitAnd for Bool4 {
        type Output = Bool4;

        #[inline(always)]
        fn bitand(self, rhs: Bool4) -> Bool4 {
            Bool4 {
                data: [
                    self.data[0] && rhs.data[0],
                    self.data[1] && rhs.data[1],
                    self.data[2] && rhs.data[2],
                    self.data[3] && rhs.data[3],
                ],
            }
        }
    }

    impl BitOr for Bool4 {
        type Output = Bool4;

        #[inline(always)]
        fn bitor(self, rhs: Bool4) -> Bool4 {
            Bool4 {
                data: [
                    self.data[0] || rhs.data[0],
                    self.data[1] || rhs.data[1],
                    self.data[2] || rhs.data[2],
                    self.data[3] || rhs.data[3],
                ],
            }
        }
    }
}

//===========================================================================

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
pub use x86_64_sse::{v_max, v_min, Bool4, Float4};

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
pub use fallback::{v_max, v_min, Bool4, Float4};

//===========================================================================

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

    #[test]
    fn lt() {
        let f1 = Float4::new(1.0, 2.0, 3.0, 4.0);
        let f2 = Float4::new(0.5, 2.0, 3.5, 2.0);

        let r = f1.lt(f2);

        assert_eq!(r.get_0(), false);
        assert_eq!(r.get_1(), false);
        assert_eq!(r.get_2(), true);
        assert_eq!(r.get_3(), false);
    }

    #[test]
    fn gt() {
        let f1 = Float4::new(1.0, 2.0, 3.0, 4.0);
        let f2 = Float4::new(0.5, 2.0, 3.5, 2.0);

        let r = f1.gt(f2);

        assert_eq!(r.get_0(), true);
        assert_eq!(r.get_1(), false);
        assert_eq!(r.get_2(), false);
        assert_eq!(r.get_3(), true);
    }

    #[test]
    fn bool4_bitmask_01() {
        let f1 = Float4::new(0.0, 0.0, 0.0, 0.0);
        let f2 = Float4::new(-1.0, -1.0, 1.0, -1.0);
        let r = f1.lt(f2).to_bitmask();

        assert_eq!(r, 0b00000100);
    }

    #[test]
    fn bool4_bitmask_02() {
        let f1 = Float4::new(0.0, 0.0, 0.0, 0.0);
        let f2 = Float4::new(1.0, -1.0, 1.0, -1.0);
        let r = f1.lt(f2).to_bitmask();

        assert_eq!(r, 0b00000101);
    }

    #[test]
    fn bool4_bitmask_03() {
        let f1 = Float4::new(0.0, 0.0, 0.0, 0.0);
        let f2 = Float4::new(-1.0, 1.0, -1.0, 1.0);
        let r = f1.lt(f2).to_bitmask();

        assert_eq!(r, 0b00001010);
    }
}
