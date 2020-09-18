//! Encoding/decoding for unsigned 40-bit trifloat numbers.
//!
//! The encoding uses 11 bits of mantissa per number, and 7 bits for the shared
//! exponent.  The bit layout is [mantissa 1, mantissa 2, mantissa 3, exponent].
//! The exponent is stored as an unsigned integer with a bias of 32.
//!
//! The largest representable number is just under `2^96`, and the smallest
//! representable non-zero number is `2^-42`.
//!
//! Since the exponent is shared between the three values, the precision
//! of all three values depends on the largest of the three.  All integers
//! up to 2048 can be represented exactly in the largest value.

use crate::{fiddle_exp2, fiddle_log2};

/// Largest representable number.
pub const MAX: f32 = ((1u128 << (128 - EXP_BIAS)) - (1 << (128 - EXP_BIAS - 11))) as f32;

/// Smallest representable non-zero number.
pub const MIN: f32 = 1.0 / (1u128 << (EXP_BIAS + 10)) as f32;

/// Difference between 1.0 and the next largest representable number.
pub const EPSILON: f32 = 1.0 / 1024.0;

const EXP_BIAS: i32 = 32;

/// Encodes three floating point values into an unsigned 40-bit trifloat.
///
/// Input floats larger than `MAX` will saturate to `MAX`, including infinity.
/// Values are converted to trifloat precision by rounding down.
///
/// Only the lower 40 bits of the return value are used.  The highest 24 bits
/// will all be zero and can be safely discarded.
///
/// Warning: negative values and NaN's are _not_ supported by the trifloat
/// format.  There are debug-only assertions in place to catch such
/// values in the input floats.
#[inline]
pub fn encode(floats: (f32, f32, f32)) -> u64 {
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
        let e = fiddle_log2(largest).max(-EXP_BIAS).min(127 - EXP_BIAS);
        let inv_multiplier = fiddle_exp2(-e + 10);
        let x = (floats.0 * inv_multiplier).min(2047.0) as u64;
        let y = (floats.1 * inv_multiplier).min(2047.0) as u64;
        let z = (floats.2 * inv_multiplier).min(2047.0) as u64;

        (x << (11 + 11 + 7)) | (y << (11 + 7)) | (z << 7) | (e + EXP_BIAS) as u64
    }
}

/// Decodes an unsigned 40-bit trifloat into three full floating point numbers.
///
/// This operation is lossless and cannot fail.  Only the lower 40 bits of the
/// input value are used--the upper 24 bits can safely be anything and are
/// ignored.
#[inline]
pub fn decode(trifloat: u64) -> (f32, f32, f32) {
    // Unpack values.
    let x = trifloat >> (11 + 11 + 7);
    let y = (trifloat >> (11 + 7)) & 0b111_1111_1111;
    let z = (trifloat >> 7) & 0b111_1111_1111;
    let e = trifloat & 0b111_1111;

    let multiplier = fiddle_exp2(e as i32 - EXP_BIAS - 10);

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

        assert_eq!(tri, 0u64);
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
        for _ in 0..1024 {
            let (x, _, _) = round_trip((n, 0.0, 0.0));
            assert_eq!(n, x);
            n += 1.0 / 1024.0;
        }
    }

    #[test]
    #[should_panic]
    fn accuracy_02() {
        let mut n = 1.0;
        for _ in 0..2048 {
            let (x, _, _) = round_trip((n, 0.0, 0.0));
            assert_eq!(n, x);
            n += 1.0 / 2048.0;
        }
    }

    #[test]
    fn integers() {
        for n in 0..=2048 {
            let (x, _, _) = round_trip((n as f32, 0.0, 0.0));
            assert_eq!(n as f32, x);
        }
    }

    #[test]
    fn precision_floor() {
        let fs = (7.0f32, 2049.0f32, 1.0f32);
        assert_eq!((6.0, 2048.0, 0.0), round_trip(fs));
    }

    #[test]
    fn saturate() {
        let fs = (1.0e+30, 1.0e+30, 1.0e+30);

        assert_eq!((MAX, MAX, MAX), round_trip(fs));
        assert_eq!((MAX, MAX, MAX), decode(0xff_ffff_ffff));
    }

    #[test]
    fn inf_saturate() {
        use std::f32::INFINITY;
        let fs = (INFINITY, 0.0, 0.0);

        assert_eq!((MAX, 0.0, 0.0), round_trip(fs));
        assert_eq!(0xffe000007f, encode(fs));
    }

    #[test]
    fn partial_saturate() {
        let fs = (
            1.0e+30,
            (1u128 << (128 - EXP_BIAS - 11)) as f32,
            (1u128 << (128 - EXP_BIAS - 12)) as f32,
        );

        assert_eq!(
            (MAX, (1u128 << (128 - EXP_BIAS - 11)) as f32, 0.0),
            round_trip(fs)
        );
    }

    #[test]
    fn smallest_value() {
        let fs = (MIN * 1.5, MIN, MIN * 0.5);
        assert_eq!((MIN, MIN, 0.0), round_trip(fs));
        assert_eq!((MIN, MIN, 0.0), decode(0x20_04_00_00));
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
