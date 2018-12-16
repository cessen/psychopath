#![allow(dead_code)]

/// Implementation of Float4 for x86_64 platforms with SSE support.
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
mod x86_64_sse {
    use std::{
        arch::x86_64::__m128,
        cmp::PartialEq,
        ops::{Add, AddAssign, BitAnd, BitOr, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    };

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
            #[cfg(target_feature = "sse3")]
            {
                use std::arch::x86_64::{
                    _mm_add_ps, _mm_add_ss, _mm_cvtss_f32, _mm_movehdup_ps, _mm_movehl_ps,
                };
                unsafe {
                    let v = self.data;
                    let shuf = _mm_movehdup_ps(v);
                    let sums = _mm_add_ps(v, shuf);
                    let shuf = _mm_movehl_ps(shuf, sums);
                    let sums = _mm_add_ss(sums, shuf);
                    _mm_cvtss_f32(sums)
                }
            }
            #[cfg(not(target_feature = "sse3"))]
            {
                use std::arch::x86_64::{
                    _mm_add_ps, _mm_add_ss, _mm_cvtss_f32, _mm_movehl_ps, _mm_shuffle_ps,
                };
                unsafe {
                    let v = self.data;
                    let shuf = _mm_shuffle_ps(v, v, (2 << 6) | (3 << 4) | 1);
                    let sums = _mm_add_ps(v, shuf);
                    let shuf = _mm_movehl_ps(shuf, sums);
                    let sums = _mm_add_ss(sums, shuf);
                    _mm_cvtss_f32(sums)
                }
            }
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
            assert!(
                n <= 3,
                "Attempted to set element of Float4 outside of bounds."
            );

            unsafe { *(&mut self.data as *mut std::arch::x86_64::__m128 as *mut f32).add(n) = v }
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

            unsafe { *(&self.data as *const std::arch::x86_64::__m128 as *const f32).add(n) }
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

    // Free functions for Float4

    #[inline(always)]
    pub fn v_min(a: Float4, b: Float4) -> Float4 {
        a.v_min(b)
    }

    #[inline(always)]
    pub fn v_max(a: Float4, b: Float4) -> Float4 {
        a.v_max(b)
    }

    /// Transposes a 4x4 matrix in-place.
    #[inline(always)]
    pub fn transpose(matrix: &mut [Float4; 4]) {
        use std::arch::x86_64::_MM_TRANSPOSE4_PS;

        // The weird &mut/*mut gymnastics below are to get around
        // the borrow-checker.  We know statically that these references
        // are non-overlapping, so it's safe.
        unsafe {
            _MM_TRANSPOSE4_PS(
                &mut *(&mut matrix[0].data as *mut __m128),
                &mut *(&mut matrix[1].data as *mut __m128),
                &mut *(&mut matrix[2].data as *mut __m128),
                &mut *(&mut matrix[3].data as *mut __m128),
            )
        };
    }

    /// Inverts a 4x4 matrix and returns the determinate.
    #[inline(always)]
    pub fn invert(matrix: &mut [Float4; 4]) -> f32 {
        // Code pulled from "Streaming SIMD Extensions - Inverse of 4x4 Matrix"
        // by Intel.
        // ftp://download.intel.com/design/PentiumIII/sml/24504301.pdf
        // Ported to Rust.

        // TODO: once __m64 and accompanying intrinsics are stabilized, switch
        // to using those, commented out in the code below.
        use std::arch::x86_64::{
            _mm_add_ps,
            _mm_add_ss,
            _mm_cvtss_f32,
            _mm_mul_ps,
            _mm_mul_ss,
            _mm_rcp_ss,
            // _mm_loadh_pi,
            // _mm_loadl_pi,
            // _mm_storeh_pi,
            // _mm_storel_pi,
            _mm_set_ps,
            _mm_shuffle_ps,
            _mm_sub_ps,
            _mm_sub_ss,
        };
        use std::mem::transmute;

        let mut minor0: __m128;
        let mut minor1: __m128;
        let mut minor2: __m128;
        let mut minor3: __m128;
        let row0: __m128;
        let mut row1: __m128;
        let mut row2: __m128;
        let mut row3: __m128;
        let mut det: __m128;
        let mut tmp1: __m128;

        unsafe {
            // tmp1 = _mm_loadh_pi(_mm_loadl_pi(tmp1, (__m64*)(src)), (__m64*)(src+ 4));
            tmp1 = _mm_set_ps(
                matrix[1].get_1(),
                matrix[1].get_0(),
                matrix[0].get_1(),
                matrix[0].get_0(),
            );

            // row1 = _mm_loadh_pi(_mm_loadl_pi(row1, (__m64*)(src+8)), (__m64*)(src+12));
            row1 = _mm_set_ps(
                matrix[3].get_1(),
                matrix[3].get_0(),
                matrix[2].get_1(),
                matrix[2].get_0(),
            );

            row0 = _mm_shuffle_ps(tmp1, row1, 0x88);
            row1 = _mm_shuffle_ps(row1, tmp1, 0xDD);

            // tmp1 = _mm_loadh_pi(_mm_loadl_pi(tmp1, (__m64*)(src+ 2)), (__m64*)(src+ 6));
            tmp1 = _mm_set_ps(
                matrix[1].get_3(),
                matrix[1].get_2(),
                matrix[0].get_3(),
                matrix[0].get_2(),
            );

            // row3 = _mm_loadh_pi(_mm_loadl_pi(row3, (__m64*)(src+10)), (__m64*)(src+14));
            row3 = _mm_set_ps(
                matrix[3].get_3(),
                matrix[3].get_2(),
                matrix[2].get_3(),
                matrix[2].get_2(),
            );

            row2 = _mm_shuffle_ps(tmp1, row3, 0x88);
            row3 = _mm_shuffle_ps(row3, tmp1, 0xDD);
            // -----------------------------------------------
            tmp1 = _mm_mul_ps(row2, row3);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0xB1);
            minor0 = _mm_mul_ps(row1, tmp1);
            minor1 = _mm_mul_ps(row0, tmp1);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0x4E);
            minor0 = _mm_sub_ps(_mm_mul_ps(row1, tmp1), minor0);
            minor1 = _mm_sub_ps(_mm_mul_ps(row0, tmp1), minor1);
            minor1 = _mm_shuffle_ps(minor1, minor1, 0x4E);
            // -----------------------------------------------
            tmp1 = _mm_mul_ps(row1, row2);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0xB1);
            minor0 = _mm_add_ps(_mm_mul_ps(row3, tmp1), minor0);
            minor3 = _mm_mul_ps(row0, tmp1);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0x4E);
            minor0 = _mm_sub_ps(minor0, _mm_mul_ps(row3, tmp1));
            minor3 = _mm_sub_ps(_mm_mul_ps(row0, tmp1), minor3);
            minor3 = _mm_shuffle_ps(minor3, minor3, 0x4E);
            // -----------------------------------------------
            tmp1 = _mm_mul_ps(_mm_shuffle_ps(row1, row1, 0x4E), row3);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0xB1);
            row2 = _mm_shuffle_ps(row2, row2, 0x4E);
            minor0 = _mm_add_ps(_mm_mul_ps(row2, tmp1), minor0);
            minor2 = _mm_mul_ps(row0, tmp1);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0x4E);
            minor0 = _mm_sub_ps(minor0, _mm_mul_ps(row2, tmp1));
            minor2 = _mm_sub_ps(_mm_mul_ps(row0, tmp1), minor2);
            minor2 = _mm_shuffle_ps(minor2, minor2, 0x4E);
            // -----------------------------------------------
            tmp1 = _mm_mul_ps(row0, row1);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0xB1);
            minor2 = _mm_add_ps(_mm_mul_ps(row3, tmp1), minor2);
            minor3 = _mm_sub_ps(_mm_mul_ps(row2, tmp1), minor3);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0x4E);
            minor2 = _mm_sub_ps(_mm_mul_ps(row3, tmp1), minor2);
            minor3 = _mm_sub_ps(minor3, _mm_mul_ps(row2, tmp1));
            // -----------------------------------------------
            tmp1 = _mm_mul_ps(row0, row3);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0xB1);
            minor1 = _mm_sub_ps(minor1, _mm_mul_ps(row2, tmp1));
            minor2 = _mm_add_ps(_mm_mul_ps(row1, tmp1), minor2);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0x4E);
            minor1 = _mm_add_ps(_mm_mul_ps(row2, tmp1), minor1);
            minor2 = _mm_sub_ps(minor2, _mm_mul_ps(row1, tmp1));
            // -----------------------------------------------
            tmp1 = _mm_mul_ps(row0, row2);
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0xB1);
            minor1 = _mm_add_ps(_mm_mul_ps(row3, tmp1), minor1);
            minor3 = _mm_sub_ps(minor3, _mm_mul_ps(row1, tmp1));
            tmp1 = _mm_shuffle_ps(tmp1, tmp1, 0x4E);
            minor1 = _mm_sub_ps(minor1, _mm_mul_ps(row3, tmp1));
            minor3 = _mm_add_ps(_mm_mul_ps(row1, tmp1), minor3);
            // -----------------------------------------------
            det = _mm_mul_ps(row0, minor0);
            det = _mm_add_ps(_mm_shuffle_ps(det, det, 0x4E), det);
            det = _mm_add_ss(_mm_shuffle_ps(det, det, 0xB1), det);
            tmp1 = _mm_rcp_ss(det);
            det = _mm_sub_ss(
                _mm_add_ss(tmp1, tmp1),
                _mm_mul_ss(det, _mm_mul_ss(tmp1, tmp1)),
            );
            det = _mm_shuffle_ps(det, det, 0x00);

            minor0 = _mm_mul_ps(det, minor0);

            // _mm_storel_pi((__m64*)(src), minor0);
            // _mm_storeh_pi((__m64*)(src+2), minor0);
            let minor0 = transmute::<__m128, [f32; 4]>(minor0);
            matrix[0].data = _mm_set_ps(minor0[3], minor0[2], minor0[1], minor0[0]);

            minor1 = _mm_mul_ps(det, minor1);

            // _mm_storel_pi((__m64*)(src+4), minor1);
            // _mm_storeh_pi((__m64*)(src+6), minor1);
            let minor1 = transmute::<__m128, [f32; 4]>(minor1);
            matrix[1].data = _mm_set_ps(minor1[3], minor1[2], minor1[1], minor1[0]);

            minor2 = _mm_mul_ps(det, minor2);

            // _mm_storel_pi((__m64*)(src+ 8), minor2);
            // _mm_storeh_pi((__m64*)(src+10), minor2);
            let minor2 = transmute::<__m128, [f32; 4]>(minor2);
            matrix[2].data = _mm_set_ps(minor2[3], minor2[2], minor2[1], minor2[0]);

            minor3 = _mm_mul_ps(det, minor3);

            // _mm_storel_pi((__m64*)(src+12), minor3);
            // _mm_storeh_pi((__m64*)(src+14), minor3);
            let minor3 = transmute::<__m128, [f32; 4]>(minor3);
            matrix[3].data = _mm_set_ps(minor3[3], minor3[2], minor3[1], minor3[0]);

            _mm_cvtss_f32(det)
        }
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
            assert!(
                n <= 3,
                "Attempted to access element of Bool4 outside of bounds."
            );

            0 != unsafe { *(&self.data as *const std::arch::x86_64::__m128 as *const u32).add(n) }
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
            let a = unsafe { *(&self.data as *const __m128 as *const u8).offset(0) };
            let b = unsafe { *(&self.data as *const __m128 as *const u8).offset(4) };
            let c = unsafe { *(&self.data as *const __m128 as *const u8).offset(8) };
            let d = unsafe { *(&self.data as *const __m128 as *const u8).offset(12) };
            (a & 0b0000_0001) | (b & 0b0000_0010) | (c & 0b0000_0100) | (d & 0b0000_1000)
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
    use std::{
        cmp::PartialEq,
        ops::{Add, AddAssign, BitAnd, BitOr, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    };

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

    // Free functions for Float4
    #[inline(always)]
    pub fn v_min(a: Float4, b: Float4) -> Float4 {
        a.v_min(b)
    }

    #[inline(always)]
    pub fn v_max(a: Float4, b: Float4) -> Float4 {
        a.v_max(b)
    }

    /// Transposes a 4x4 matrix in-place
    #[inline(always)]
    pub fn transpose(matrix: &mut [Float4; 4]) {
        let m = [
            Float4::new(
                matrix[0].get_0(),
                matrix[1].get_0(),
                matrix[2].get_0(),
                matrix[3].get_0(),
            ),
            Float4::new(
                matrix[0].get_1(),
                matrix[1].get_1(),
                matrix[2].get_1(),
                matrix[3].get_1(),
            ),
            Float4::new(
                matrix[0].get_2(),
                matrix[1].get_2(),
                matrix[2].get_2(),
                matrix[3].get_2(),
            ),
            Float4::new(
                matrix[0].get_3(),
                matrix[1].get_3(),
                matrix[2].get_3(),
                matrix[3].get_3(),
            ),
        ];

        *matrix = m;
    }

    /// Inverts a 4x4 matrix and returns the determinate.
    #[inline(always)]
    pub fn invert(matrix: &mut [Float4; 4]) -> f32 {
        let m = *matrix;

        let s0 = (m[0].get_0() * m[1].get_1()) - (m[1].get_0() * m[0].get_1());
        let s1 = (m[0].get_0() * m[1].get_2()) - (m[1].get_0() * m[0].get_2());
        let s2 = (m[0].get_0() * m[1].get_3()) - (m[1].get_0() * m[0].get_3());
        let s3 = (m[0].get_1() * m[1].get_2()) - (m[1].get_1() * m[0].get_2());
        let s4 = (m[0].get_1() * m[1].get_3()) - (m[1].get_1() * m[0].get_3());
        let s5 = (m[0].get_2() * m[1].get_3()) - (m[1].get_2() * m[0].get_3());

        let c5 = (m[2].get_2() * m[3].get_3()) - (m[3].get_2() * m[2].get_3());
        let c4 = (m[2].get_1() * m[3].get_3()) - (m[3].get_1() * m[2].get_3());
        let c3 = (m[2].get_1() * m[3].get_2()) - (m[3].get_1() * m[2].get_2());
        let c2 = (m[2].get_0() * m[3].get_3()) - (m[3].get_0() * m[2].get_3());
        let c1 = (m[2].get_0() * m[3].get_2()) - (m[3].get_0() * m[2].get_2());
        let c0 = (m[2].get_0() * m[3].get_1()) - (m[3].get_0() * m[2].get_1());

        // We don't check for 0.0 determinant, as that is expected to be handled
        // by the calling code.
        let det = (s0 * c5) - (s1 * c4) + (s2 * c3) + (s3 * c2) - (s4 * c1) + (s5 * c0);
        let invdet = 1.0 / det;

        *matrix = [
            Float4::new(
                ((m[1].get_1() * c5) - (m[1].get_2() * c4) + (m[1].get_3() * c3)) * invdet,
                ((-m[0].get_1() * c5) + (m[0].get_2() * c4) - (m[0].get_3() * c3)) * invdet,
                ((m[3].get_1() * s5) - (m[3].get_2() * s4) + (m[3].get_3() * s3)) * invdet,
                ((-m[2].get_1() * s5) + (m[2].get_2() * s4) - (m[2].get_3() * s3)) * invdet,
            ),
            Float4::new(
                ((-m[1].get_0() * c5) + (m[1].get_2() * c2) - (m[1].get_3() * c1)) * invdet,
                ((m[0].get_0() * c5) - (m[0].get_2() * c2) + (m[0].get_3() * c1)) * invdet,
                ((-m[3].get_0() * s5) + (m[3].get_2() * s2) - (m[3].get_3() * s1)) * invdet,
                ((m[2].get_0() * s5) - (m[2].get_2() * s2) + (m[2].get_3() * s1)) * invdet,
            ),
            Float4::new(
                ((m[1].get_0() * c4) - (m[1].get_1() * c2) + (m[1].get_3() * c0)) * invdet,
                ((-m[0].get_0() * c4) + (m[0].get_1() * c2) - (m[0].get_3() * c0)) * invdet,
                ((m[3].get_0() * s4) - (m[3].get_1() * s2) + (m[3].get_3() * s0)) * invdet,
                ((-m[2].get_0() * s4) + (m[2].get_1() * s2) - (m[2].get_3() * s0)) * invdet,
            ),
            Float4::new(
                ((-m[1].get_0() * c3) + (m[1].get_1() * c1) - (m[1].get_2() * c0)) * invdet,
                ((m[0].get_0() * c3) - (m[0].get_1() * c1) + (m[0].get_2() * c0)) * invdet,
                ((-m[3].get_0() * s3) + (m[3].get_1() * s1) - (m[3].get_2() * s0)) * invdet,
                ((m[2].get_0() * s3) - (m[2].get_1() * s1) + (m[2].get_2() * s0)) * invdet,
            ),
        ];

        det
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
        pub fn get_n(self, n: usize) -> bool {
            assert!(
                n <= 3,
                "Attempted to access element of Bool4 outside of bounds."
            );
            unsafe { *self.data.get_unchecked(n) }
        }

        /// Returns the value of the 0th element.
        #[inline(always)]
        pub fn get_0(self) -> bool {
            self.get_n(0)
        }

        /// Returns the value of the 1th element.
        #[inline(always)]
        pub fn get_1(self) -> bool {
            self.get_n(1)
        }

        /// Returns the value of the 2th element.
        #[inline(always)]
        pub fn get_2(self) -> bool {
            self.get_n(2)
        }

        /// Returns the value of the 3th element.
        #[inline(always)]
        pub fn get_3(self) -> bool {
            self.get_n(3)
        }

        #[inline]
        pub fn to_bitmask(self) -> u8 {
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
pub use crate::x86_64_sse::{invert, transpose, v_max, v_min, Bool4, Float4};

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
pub use fallback::{invert, transpose, v_max, v_min, Bool4, Float4};

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
    fn matrix_transpose() {
        let mut m1 = [
            Float4::new(1.0, 2.0, 3.0, 4.0),
            Float4::new(5.0, 6.0, 7.0, 8.0),
            Float4::new(9.0, 10.0, 11.0, 12.0),
            Float4::new(13.0, 14.0, 15.0, 16.0),
        ];
        let m2 = [
            Float4::new(1.0, 5.0, 9.0, 13.0),
            Float4::new(2.0, 6.0, 10.0, 14.0),
            Float4::new(3.0, 7.0, 11.0, 15.0),
            Float4::new(4.0, 8.0, 12.0, 16.0),
        ];

        transpose(&mut m1);

        assert_eq!(m1, m2);
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
