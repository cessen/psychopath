//! Encoding/decoding for signed 48-bit trifloat numbers.
//!
//! The encoding uses 13 bits of mantissa and 1 sign bit per number, and 6
//! bits for the shared exponent. The bit layout is: [sign 1, mantissa 1,
//! sign 2, mantissa 2, sign 3, mantissa 3, exponent].  The exponent is stored
//! as an unsigned integer with a bias of 25.
//!
//! The largest representable number is `2^38 - 2^25`, and the smallest
//! representable positive number is `2^-38`.
//!
//! Since the exponent is shared between all three values, the precision
//! of all three values depends on the largest (in magnitude) of the three.
//! All integers in the range `[-8192, 8192]` can be represented exactly in the
//! largest value.

#![allow(clippy::cast_lossless)]

use crate::{fiddle_exp2, fiddle_log2};

/// Largest representable number.
pub const MAX: f32 = 274_844_352_512.0;

/// Smallest representable number.
///
/// Note this is not the smallest _magnitude_ number.  This is a negative
/// number of large magnitude.
pub const MIN: f32 = -274_844_352_512.0;

/// Smallest representable positive number.
///
/// This is the number with the smallest possible magnitude (aside from zero).
#[allow(clippy::excessive_precision)]
pub const MIN_POSITIVE: f32 = 0.000_000_000_003_637_978_807_091_713;

/// Difference between 1.0 and the next largest representable number.
pub const EPSILON: f32 = 1.0 / 4096.0;

const EXP_BIAS: i32 = 25;
const MIN_EXP: i32 = 0 - EXP_BIAS;
const MAX_EXP: i32 = 63 - EXP_BIAS;

/// Encodes three floating point values into a signed 48-bit trifloat.
///
/// Input floats that are larger than `MAX` or smaller than `MIN` will saturate
/// to `MAX` and `MIN` respectively, including +/- infinity.  Values are
/// converted to trifloat precision by rounding.
///
/// Only the lower 48 bits of the return value are used.  The highest 16 bits
/// will all be zero and can be safely discarded.
///
/// Warning: NaN's are _not_ supported by the trifloat format.  There are
/// debug-only assertions in place to catch such values in the input floats.
#[inline]
pub fn encode(floats: (f32, f32, f32)) -> u64 {
    debug_assert!(
        !floats.0.is_nan() && !floats.1.is_nan() && !floats.2.is_nan(),
        "trifloat::signed48::encode(): encoding to signed tri-floats only \
         works correctly for non-NaN numbers, but the numbers passed were: \
         ({}, {}, {})",
        floats.0,
        floats.1,
        floats.2
    );

    // Find the largest (in magnitude) of the three values.
    let largest_value = {
        let mut largest_value: f32 = 0.0;
        if floats.0.abs() > largest_value.abs() {
            largest_value = floats.0;
        }
        if floats.1.abs() > largest_value.abs() {
            largest_value = floats.1;
        }
        if floats.2.abs() > largest_value.abs() {
            largest_value = floats.2;
        }
        largest_value
    };

    // Calculate the exponent and 1.0/multiplier for encoding the values.
    let (exponent, inv_multiplier) = {
        let mut exponent = (fiddle_log2(largest_value) + 1).max(MIN_EXP).min(MAX_EXP);
        let mut inv_multiplier = fiddle_exp2(-exponent + 13);

        // Edge-case: make sure rounding pushes the largest value up
        // appropriately if needed.
        if (largest_value * inv_multiplier).abs() + 0.5 >= 8192.0 {
            exponent = (exponent + 1).min(MAX_EXP);
            inv_multiplier = fiddle_exp2(-exponent + 13);
        }
        (exponent, inv_multiplier)
    };

    // Quantize and encode values.
    let x = (floats.0.abs() * inv_multiplier + 0.5).min(8191.0) as u64 & 0b111_11111_11111;
    let x_sign = (floats.0.to_bits() >> 31) as u64;
    let y = (floats.1.abs() * inv_multiplier + 0.5).min(8191.0) as u64 & 0b111_11111_11111;
    let y_sign = (floats.1.to_bits() >> 31) as u64;
    let z = (floats.2.abs() * inv_multiplier + 0.5).min(8191.0) as u64 & 0b111_11111_11111;
    let z_sign = (floats.2.to_bits() >> 31) as u64;
    let e = (exponent + EXP_BIAS) as u64 & 0b111_111;

    // Pack values into a single u64 and return.
    (x_sign << 47) | (x << 34) | (y_sign << 33) | (y << 20) | (z_sign << 19) | (z << 6) | e
}

/// Decodes a signed 48-bit trifloat into three full floating point numbers.
///
/// This operation is lossless and cannot fail.  Only the lower 48 bits of the
/// input value are used--the upper 16 bits can safely be anything and are
/// ignored.
#[inline]
pub fn decode(trifloat: u64) -> (f32, f32, f32) {
    // Unpack values.
    let x = (trifloat >> 34) & 0b111_11111_11111;
    let y = (trifloat >> 20) & 0b111_11111_11111;
    let z = (trifloat >> 6) & 0b111_11111_11111;

    let x_sign = ((trifloat >> 16) & 0x8000_0000) as u32;
    let y_sign = ((trifloat >> 2) & 0x8000_0000) as u32;
    let z_sign = ((trifloat << 12) & 0x8000_0000) as u32;

    let e = trifloat & 0b111_111;

    let multiplier = fiddle_exp2(e as i32 - EXP_BIAS - 13);

    (
        f32::from_bits((x as f32 * multiplier).to_bits() | x_sign),
        f32::from_bits((y as f32 * multiplier).to_bits() | y_sign),
        f32::from_bits((z as f32 * multiplier).to_bits() | z_sign),
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

        assert_eq!(tri, 0);
        assert_eq!(fs, fs2);
    }

    #[test]
    fn powers_of_two() {
        let fs = (8.0f32, 128.0f32, 0.5f32);
        assert_eq!(round_trip(fs), fs);
    }

    #[test]
    fn signs() {
        let fs1 = (1.0f32, 1.0f32, 1.0f32);
        let fs2 = (1.0f32, 1.0f32, -1.0f32);
        let fs3 = (1.0f32, -1.0f32, 1.0f32);
        let fs4 = (1.0f32, -1.0f32, -1.0f32);
        let fs5 = (-1.0f32, 1.0f32, 1.0f32);
        let fs6 = (-1.0f32, 1.0f32, -1.0f32);
        let fs7 = (-1.0f32, -1.0f32, 1.0f32);
        let fs8 = (-1.0f32, -1.0f32, -1.0f32);

        assert_eq!(fs1, round_trip(fs1));
        assert_eq!(fs2, round_trip(fs2));
        assert_eq!(fs3, round_trip(fs3));
        assert_eq!(fs4, round_trip(fs4));
        assert_eq!(fs5, round_trip(fs5));
        assert_eq!(fs6, round_trip(fs6));
        assert_eq!(fs7, round_trip(fs7));
        assert_eq!(fs8, round_trip(fs8));
    }

    #[test]
    fn accuracy() {
        let mut n = 1.0;
        for _ in 0..256 {
            let (x, _, _) = round_trip((n, 0.0, 0.0));
            assert_eq!(n, x);
            n += 1.0 / 256.0;
        }
    }

    #[test]
    fn integers() {
        for n in -8192i32..=8192i32 {
            let (x, _, _) = round_trip((n as f32, 0.0, 0.0));
            assert_eq!(n as f32, x);
        }
    }

    #[test]
    fn rounding() {
        let fs = (7.0f32, 8193.0f32, -1.0f32);
        let fsn = (-7.0f32, -8193.0f32, 1.0f32);
        assert_eq!(round_trip(fs), (8.0, 8194.0, -2.0));
        assert_eq!(round_trip(fsn), (-8.0, -8194.0, 2.0));
    }

    #[test]
    fn rounding_edge_case() {
        let fs = (16383.0f32, 0.0f32, 0.0f32);

        assert_eq!(round_trip(fs), (16384.0, 0.0, 0.0),);
    }

    #[test]
    fn saturate() {
        let fs = (
            99_999_999_999_999.0,
            99_999_999_999_999.0,
            99_999_999_999_999.0,
        );
        let fsn = (
            -99_999_999_999_999.0,
            -99_999_999_999_999.0,
            -99_999_999_999_999.0,
        );

        assert_eq!(round_trip(fs), (MAX, MAX, MAX));
        assert_eq!(round_trip(fsn), (MIN, MIN, MIN));
        assert_eq!(decode(0x7FFD_FFF7_FFFF), (MAX, MAX, MAX));
        assert_eq!(decode(0xFFFF_FFFF_FFFF), (MIN, MIN, MIN));
    }

    #[test]
    fn inf_saturate() {
        use std::f32::INFINITY;
        let fs = (INFINITY, 0.0, 0.0);
        let fsn = (-INFINITY, 0.0, 0.0);

        assert_eq!(round_trip(fs), (MAX, 0.0, 0.0));
        assert_eq!(round_trip(fsn), (MIN, 0.0, 0.0));
        assert_eq!(encode(fs), 0x7FFC0000003F);
        assert_eq!(encode(fsn), 0xFFFC0000003F);
    }

    #[test]
    fn partial_saturate() {
        let fs = (99_999_999_999_999.0, 4294967296.0, -17179869184.0);
        let fsn = (-99_999_999_999_999.0, 4294967296.0, -17179869184.0);

        assert_eq!(round_trip(fs), (MAX, 4294967296.0, -17179869184.0));
        assert_eq!(round_trip(fsn), (MIN, 4294967296.0, -17179869184.0));
    }

    #[test]
    fn smallest_value() {
        let fs = (MIN_POSITIVE, MIN_POSITIVE * 0.5, MIN_POSITIVE * 0.49);
        let fsn = (-MIN_POSITIVE, -MIN_POSITIVE * 0.5, -MIN_POSITIVE * 0.49);

        assert_eq!(decode(0x600100000), (MIN_POSITIVE, -MIN_POSITIVE, 0.0));
        assert_eq!(round_trip(fs), (MIN_POSITIVE, MIN_POSITIVE, 0.0));
        assert_eq!(round_trip(fsn), (-MIN_POSITIVE, -MIN_POSITIVE, -0.0));
    }

    #[test]
    fn underflow() {
        let fs = (MIN_POSITIVE * 0.49, -MIN_POSITIVE * 0.49, 0.0);
        assert_eq!(encode(fs), 0x200000000);
        assert_eq!(round_trip(fs), (0.0, -0.0, 0.0));
    }

    #[test]
    fn garbage_upper_bits_decode() {
        let fs1 = (4.0, -623.53, 12.3);
        let fs2 = (-63456254.2, 5235423.53, 54353.3);
        let fs3 = (-0.000000634, 0.00000000005, 0.00000000892);

        let n1 = encode(fs1);
        let n2 = encode(fs2);
        let n3 = encode(fs3);

        assert_eq!(decode(n1), decode(n1 | 0xffff_0000_0000_0000));
        assert_eq!(decode(n2), decode(n2 | 0xffff_0000_0000_0000));
        assert_eq!(decode(n3), decode(n3 | 0xffff_0000_0000_0000));
    }

    #[test]
    #[should_panic]
    fn nans_01() {
        encode((std::f32::NAN, 1.0, -1.0));
    }

    #[test]
    #[should_panic]
    fn nans_02() {
        encode((1.0, std::f32::NAN, -1.0));
    }

    #[test]
    #[should_panic]
    fn nans_03() {
        encode((1.0, -1.0, std::f32::NAN));
    }
}
