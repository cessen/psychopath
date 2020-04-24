//--------------------------------------------------------------------------
// x86/64 SSE
#[cfg(target_arch = "x86_64")]
// #[cfg(all(target_arch = "x86_64", target_feature = "sse4.1"))]
pub(crate) mod sse {
    use core::arch::x86_64::{
        __m128i,

        // SSE2 or less
        _mm_add_epi32,
        _mm_and_si128,
        _mm_cvtepi32_ps,
        _mm_mul_ps,
        _mm_or_si128,
        _mm_set1_epi32,
        _mm_set1_ps,
        _mm_setzero_si128,
        _mm_slli_epi32,
        _mm_srli_epi32,
        _mm_xor_si128,
    };

    use core::arch::x86_64::{
        // SSE3 / SSE4.1
        // Note: these aren't necessarily actually available on all
        // x86_64 platforms, so their use here isn't quite correct
        // with the platform guard above.
        // TODO: fix this at some point.
        _mm_loadu_si128,
        _mm_mullo_epi32,
    };

    #[derive(Debug, Copy, Clone)]
    pub(crate) struct Int4 {
        v: __m128i,
    }

    impl Int4 {
        #[inline(always)]
        pub fn zero() -> Int4 {
            Int4 {
                v: unsafe { _mm_setzero_si128() },
            }
        }

        /// Converts the full range of a 32 bit integer to a float in [0, 1).
        #[inline(always)]
        pub fn to_norm_floats(self) -> [f32; 4] {
            const ONE_OVER_31BITS: f32 = 1.0 / (1u64 << 31) as f32;
            let n4 = unsafe {
                _mm_mul_ps(
                    _mm_cvtepi32_ps(_mm_srli_epi32(self.v, 1)),
                    _mm_set1_ps(ONE_OVER_31BITS),
                )
            };

            unsafe { std::mem::transmute(n4) }
        }

        #[inline]
        pub fn reverse_bits(self) -> Int4 {
            let mut n = self.v;
            unsafe {
                let a = _mm_slli_epi32(n, 16);
                let b = _mm_srli_epi32(n, 16);
                n = _mm_or_si128(a, b);

                //----
                let a = _mm_and_si128(
                    _mm_slli_epi32(n, 8),
                    _mm_set1_epi32(std::mem::transmute(0xff00ff00u32)),
                );
                let b = _mm_and_si128(
                    _mm_srli_epi32(n, 8),
                    _mm_set1_epi32(std::mem::transmute(0x00ff00ffu32)),
                );
                n = _mm_or_si128(a, b);

                //----
                let a = _mm_and_si128(
                    _mm_slli_epi32(n, 4),
                    _mm_set1_epi32(std::mem::transmute(0xf0f0f0f0u32)),
                );
                let b = _mm_and_si128(
                    _mm_srli_epi32(n, 4),
                    _mm_set1_epi32(std::mem::transmute(0x0f0f0f0fu32)),
                );
                n = _mm_or_si128(a, b);

                //----
                let a = _mm_and_si128(
                    _mm_slli_epi32(n, 2),
                    _mm_set1_epi32(std::mem::transmute(0xccccccccu32)),
                );
                let b = _mm_and_si128(
                    _mm_srli_epi32(n, 2),
                    _mm_set1_epi32(std::mem::transmute(0x33333333u32)),
                );
                n = _mm_or_si128(a, b);

                //----
                let a = _mm_and_si128(
                    _mm_slli_epi32(n, 1),
                    _mm_set1_epi32(std::mem::transmute(0xaaaaaaaau32)),
                );
                let b = _mm_and_si128(
                    _mm_srli_epi32(n, 1),
                    _mm_set1_epi32(std::mem::transmute(0x55555555u32)),
                );
                n = _mm_or_si128(a, b);

                Int4 { v: n }
            }
        }
    }

    impl std::ops::MulAssign for Int4 {
        #[inline(always)]
        fn mul_assign(&mut self, other: Self) {
            *self = Int4 {
                v: unsafe { _mm_mullo_epi32(self.v, other.v) },
            };
        }
    }

    impl std::ops::AddAssign for Int4 {
        #[inline(always)]
        fn add_assign(&mut self, other: Self) {
            *self = Int4 {
                v: unsafe { _mm_add_epi32(self.v, other.v) },
            };
        }
    }

    impl std::ops::BitXorAssign for Int4 {
        #[inline(always)]
        fn bitxor_assign(&mut self, other: Self) {
            *self = Int4 {
                v: unsafe { _mm_xor_si128(self.v, other.v) },
            };
        }
    }

    impl From<[u32; 4]> for Int4 {
        #[inline(always)]
        fn from(v: [u32; 4]) -> Self {
            Int4 {
                v: unsafe { _mm_loadu_si128(std::mem::transmute(&v as *const u32)) },
            }
        }
    }
}
#[cfg(target_arch = "x86_64")]
pub(crate) use sse::Int4;

//--------------------------------------------------------------------------
// Fallback
#[cfg(not(target_arch = "x86_64"))]
// #[cfg(not(all(target_arch = "x86_64", target_feature = "sse4.1")))]
pub(crate) mod fallback {
    #[derive(Debug, Copy, Clone)]
    #[repr(align(16))]
    pub(crate) struct Int4 {
        v: [u32; 4],
    }

    impl Int4 {
        pub fn zero() -> Int4 {
            Int4 { v: [0, 0, 0, 0] }
        }

        /// Converts the full range of a 32 bit integer to a float in [0, 1).
        pub fn to_norm_floats(self) -> [f32; 4] {
            const ONE_OVER_32BITS: f32 = 1.0 / (1u64 << 32) as f32;
            [
                self.v[0] as f32 * ONE_OVER_32BITS,
                self.v[1] as f32 * ONE_OVER_32BITS,
                self.v[2] as f32 * ONE_OVER_32BITS,
                self.v[3] as f32 * ONE_OVER_32BITS,
            ]
        }

        pub fn reverse_bits(self) -> Int4 {
            Int4 {
                v: [
                    self.v[0].reverse_bits(),
                    self.v[1].reverse_bits(),
                    self.v[2].reverse_bits(),
                    self.v[3].reverse_bits(),
                ],
            }
        }
    }

    impl std::ops::MulAssign for Int4 {
        fn mul_assign(&mut self, other: Self) {
            *self = Int4 {
                v: [
                    self.v[0].wrapping_mul(other.v[0]),
                    self.v[1].wrapping_mul(other.v[1]),
                    self.v[2].wrapping_mul(other.v[2]),
                    self.v[3].wrapping_mul(other.v[3]),
                ],
            };
        }
    }

    impl std::ops::AddAssign for Int4 {
        fn add_assign(&mut self, other: Self) {
            *self = Int4 {
                v: [
                    self.v[0].wrapping_add(other.v[0]),
                    self.v[1].wrapping_add(other.v[1]),
                    self.v[2].wrapping_add(other.v[2]),
                    self.v[3].wrapping_add(other.v[3]),
                ],
            };
        }
    }

    impl std::ops::BitXorAssign for Int4 {
        fn bitxor_assign(&mut self, other: Self) {
            *self = Int4 {
                v: [
                    self.v[0] ^ other.v[0],
                    self.v[1] ^ other.v[1],
                    self.v[2] ^ other.v[2],
                    self.v[3] ^ other.v[3],
                ],
            };
        }
    }

    impl From<[u32; 4]> for Int4 {
        fn from(v: [u32; 4]) -> Self {
            Int4 { v: v }
        }
    }
}
#[cfg(not(target_arch = "x86_64"))]
pub(crate) use fallback::Int4;
