//! Encoding/decoding for unsigned 32-bit trifloat numbers.
//!
//! The encoding uses 9 bits of mantissa per number, and 5 bits for the shared
//! exponent.  The bit layout is [mantissa 1, mantissa 2, mantissa 3, exponent].
//! The exponent is stored as an unsigned integer with a bias of 11.
//!
//! The largest representable number is `2^21 - 4096`, and the smallest
//! representable non-zero number is `2^-19`.
//!
//! Since the exponent is shared between the three values, the precision
//! of all three values depends on the largest of the three.  All integers
//! up to 512 can be represented exactly in the largest value.

use super::{fiddle_exp2, fiddle_log2};

/// Largest representable number.
pub const MAX: f32 = ((1u64 << (32 - EXP_BIAS)) - (1 << (32 - EXP_BIAS - 9))) as f32;

/// Smallest representable non-zero number.
pub const MIN: f32 = 1.0 / (1 << (EXP_BIAS + 8)) as f32;

/// Difference between 1.0 and the next largest representable number.
pub const EPSILON: f32 = 1.0 / 256.0;

const EXP_BIAS: i32 = 11;

/// Encodes three floating point values into an unsigned 32-bit trifloat.
///
/// Input floats larger than `MAX` will saturate to `MAX`, including infinity.
/// Values are converted to trifloat precision by rounding down.
///
/// Warning: negative values and NaN's are _not_ supported by the trifloat
/// format.  There are debug-only assertions in place to catch such
/// values in the input floats.
#[inline]
pub fn encode(floats: (f32, f32, f32)) -> u32 {
    debug_assert!(
        floats.0 >= 0.0
            && floats.1 >= 0.0
            && floats.2 >= 0.0
            && !floats.0.is_nan()
            && !floats.1.is_nan()
            && !floats.2.is_nan(),
        "trifloat::unsigned32::encode(): encoding to unsigned tri-floats only \
         works correctly for positive, non-NaN numbers, but the numbers passed \
         were: ({}, {}, {})",
        floats.0,
        floats.1,
        floats.2
    );

    let largest = floats.0.max(floats.1.max(floats.2));

    if largest < MIN {
        return 0;
    } else {
        let e = fiddle_log2(largest).max(-EXP_BIAS).min(31 - EXP_BIAS);
        let inv_multiplier = fiddle_exp2(-e + 8);
        let x = (floats.0 * inv_multiplier).min(511.0) as u32;
        let y = (floats.1 * inv_multiplier).min(511.0) as u32;
        let z = (floats.2 * inv_multiplier).min(511.0) as u32;

        (x << (9 + 9 + 5)) | (y << (9 + 5)) | (z << 5) | (e + EXP_BIAS) as u32
    }
}

/// Decodes an unsigned 32-bit trifloat into three full floating point numbers.
///
/// This operation is lossless and cannot fail.
#[inline]
pub fn decode(trifloat: u32) -> (f32, f32, f32) {
    // Unpack values.
    let x = trifloat >> (9 + 9 + 5);
    let y = (trifloat >> (9 + 5)) & 0b1_1111_1111;
    let z = (trifloat >> 5) & 0b1_1111_1111;
    let e = trifloat & 0b1_1111;

    let multiplier = fiddle_exp2(e as i32 - EXP_BIAS - 8);

    (
        x as f32 * multiplier,
        y as f32 * multiplier,
        z as f32 * multiplier,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(floats: (f32, f32, f32)) -> (f32, f32, f32) {
        decode(encode(floats))
    }

    #[test]
    fn all_zeros() {
        let fs = (0.0f32, 0.0f32, 0.0f32);

        let tri = encode(fs);
        let fs2 = decode(tri);

        assert_eq!(tri, 0u32);
        assert_eq!(fs, fs2);
    }

    #[test]
    fn powers_of_two() {
        let fs = (8.0f32, 128.0f32, 0.5f32);
        assert_eq!(fs, round_trip(fs));
    }

    #[test]
    fn accuracy_01() {
        let mut n = 1.0;
        for _ in 0..256 {
            let (x, _, _) = round_trip((n, 0.0, 0.0));
            assert_eq!(n, x);
            n += 1.0 / 256.0;
        }
    }

    #[test]
    #[should_panic]
    fn accuracy_02() {
        let mut n = 1.0;
        for _ in 0..512 {
            let (x, _, _) = round_trip((n, 0.0, 0.0));
            assert_eq!(n, x);
            n += 1.0 / 512.0;
        }
    }

    #[test]
    fn integers() {
        for n in 0..=512 {
            let (x, _, _) = round_trip((n as f32, 0.0, 0.0));
            assert_eq!(n as f32, x);
        }
    }

    #[test]
    fn precision_floor() {
        let fs = (7.0f32, 513.0f32, 1.0f32);
        assert_eq!((6.0, 512.0, 0.0), round_trip(fs));
    }

    #[test]
    fn saturate() {
        let fs = (9999999999.0, 9999999999.0, 9999999999.0);

        assert_eq!((MAX, MAX, MAX), round_trip(fs));
        assert_eq!((MAX, MAX, MAX), decode(0xFFFFFFFF));
    }

    #[test]
    fn inf_saturate() {
        use std::f32::INFINITY;
        let fs = (INFINITY, 0.0, 0.0);

        assert_eq!((MAX, 0.0, 0.0), round_trip(fs));
        assert_eq!(0xFF80001F, encode(fs));
    }

    #[test]
    fn partial_saturate() {
        let fs = (9999999999.0, 4096.0, 262144.0);

        assert_eq!((MAX, 4096.0, 262144.0), round_trip(fs));
    }

    #[test]
    fn smallest_value() {
        let fs = (MIN * 1.5, MIN, MIN * 0.5);
        assert_eq!((MIN, MIN, 0.0), round_trip(fs));
        assert_eq!((MIN, MIN, 0.0), decode(0x00_80_40_00));
    }

    #[test]
    fn underflow() {
        let fs = (MIN * 0.99, 0.0, 0.0);
        assert_eq!(0, encode(fs));
        assert_eq!((0.0, 0.0, 0.0), round_trip(fs));
    }

    #[test]
    #[should_panic]
    fn nans_01() {
        encode((std::f32::NAN, 0.0, 0.0));
    }

    #[test]
    #[should_panic]
    fn nans_02() {
        encode((0.0, std::f32::NAN, 0.0));
    }

    #[test]
    #[should_panic]
    fn nans_03() {
        encode((0.0, 0.0, std::f32::NAN));
    }

    #[test]
    #[should_panic]
    fn negative_01() {
        encode((-1.0, 0.0, 0.0));
    }

    #[test]
    #[should_panic]
    fn negative_02() {
        encode((0.0, -1.0, 0.0));
    }

    #[test]
    #[should_panic]
    fn negative_03() {
        encode((0.0, 0.0, -1.0));
    }

    #[test]
    fn negative_04() {
        encode((-0.0, -0.0, -0.0));
    }
}
