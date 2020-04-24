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
