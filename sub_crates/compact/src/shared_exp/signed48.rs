//! Encoding/decoding for signed 48-bit trifloat numbers.
//!
//! The encoding uses 13 bits of mantissa and 1 sign bit per number, and 6
//! bits for the shared exponent. The bit layout is: [sign 1, mantissa 1,
//! sign 2, mantissa 2, sign 3, mantissa 3, exponent].  The exponent is stored
//! as an unsigned integer with a bias of 26.
//!
//! The largest representable number is just under `2^38`, and the smallest
//! representable positive number is `2^-38`.
//!
//! Since the exponent is shared between all three values, the precision
//! of all three values depends on the largest (in magnitude) of the three.
//! All integers in the range `[-8192, 8192]` can be represented exactly in the
//! largest value.

#![allow(clippy::cast_lossless)]

use super::{fiddle_exp2, fiddle_log2};

/// Largest representable number.
pub const MAX: f32 = ((1u128 << (64 - EXP_BIAS)) - (1 << (64 - EXP_BIAS - 13))) as f32;

/// Smallest representable number.
///
/// Note this is not the smallest _magnitude_ number.  This is a negative
/// number of large magnitude.
pub const MIN: f32 = -MAX;

/// Smallest representable positive number.
///
/// This is the number with the smallest possible magnitude (aside from zero).
pub const MIN_POSITIVE: f32 = 1.0 / (1u128 << (EXP_BIAS + 12)) as f32;

/// Difference between 1.0 and the next largest representable number.
pub const EPSILON: f32 = 1.0 / 4096.0;

const EXP_BIAS: i32 = 26;

/// Encodes three floating point values into a signed 48-bit trifloat.
///
/// Input floats that are larger than `MAX` or smaller than `MIN` will saturate
/// to `MAX` and `MIN` respectively, including +/- infinity.  Values are
/// converted to trifloat precision by rounding towards zero.
///
/// Warning: NaN's are _not_ supported by the trifloat format.  There are
/// debug-only assertions in place to catch such values in the input floats.
#[inline]
pub fn encode(floats: (f32, f32, f32)) -> [u8; 6] {
    u64_to_bytes(encode_64(floats))
}

/// Decodes a signed 48-bit trifloat into three full floating point numbers.
///
/// This operation is lossless and cannot fail.
#[inline]
pub fn decode(trifloat: [u8; 6]) -> (f32, f32, f32) {
    decode_64(bytes_to_u64(trifloat))
}

// Workhorse encode function, which operates on u64.
#[inline(always)]
fn encode_64(floats: (f32, f32, f32)) -> u64 {
    debug_assert!(
        !floats.0.is_nan() && !floats.1.is_nan() && !floats.2.is_nan(),
        "trifloat::signed48::encode(): encoding to signed tri-floats only \
         works correctly for non-NaN numbers, but the numbers passed were: \
         ({}, {}, {})",
        floats.0,
        floats.1,
        floats.2
    );

    let floats_abs = (floats.0.abs(), floats.1.abs(), floats.2.abs());

    let largest_abs = floats_abs.0.max(floats_abs.1.max(floats_abs.2));

    if largest_abs < MIN_POSITIVE {
        0
    } else {
        let e = fiddle_log2(largest_abs).max(-EXP_BIAS).min(63 - EXP_BIAS);
        let inv_multiplier = fiddle_exp2(-e + 12);

        let x_sign = (floats.0.to_bits() >> 31) as u64;
        let x = (floats_abs.0 * inv_multiplier).min(8191.0) as u64;
        let y_sign = (floats.1.to_bits() >> 31) as u64;
        let y = (floats_abs.1 * inv_multiplier).min(8191.0) as u64;
        let z_sign = (floats.2.to_bits() >> 31) as u64;
        let z = (floats_abs.2 * inv_multiplier).min(8191.0) as u64;

        (x_sign << 47)
            | (x << 34)
            | (y_sign << 33)
            | (y << 20)
            | (z_sign << 19)
            | (z << 6)
            | (e + EXP_BIAS) as u64
    }
}

// Workhorse decode function, which operates on u64.
#[inline(always)]
fn decode_64(trifloat: u64) -> (f32, f32, f32) {
    // Unpack values.
    let x = (trifloat >> 34) & 0b111_11111_11111;
    let y = (trifloat >> 20) & 0b111_11111_11111;
    let z = (trifloat >> 6) & 0b111_11111_11111;

    let x_sign = ((trifloat >> 16) & 0x8000_0000) as u32;
    let y_sign = ((trifloat >> 2) & 0x8000_0000) as u32;
    let z_sign = ((trifloat << 12) & 0x8000_0000) as u32;

    let e = trifloat & 0b111_111;

    let multiplier = fiddle_exp2(e as i32 - EXP_BIAS - 12);

    (
        f32::from_bits((x as f32 * multiplier).to_bits() | x_sign),
        f32::from_bits((y as f32 * multiplier).to_bits() | y_sign),
        f32::from_bits((z as f32 * multiplier).to_bits() | z_sign),
    )
}

#[inline(always)]
fn u64_to_bytes(n: u64) -> [u8; 6] {
    let a = n.to_ne_bytes();
    let mut b = [0u8; 6];
    if cfg!(target_endian = "big") {
        (&mut b[..]).copy_from_slice(&a[2..8]);
    } else {
        (&mut b[..]).copy_from_slice(&a[0..6]);
    }
    b
}

#[inline(always)]
fn bytes_to_u64(a: [u8; 6]) -> u64 {
    let mut b = [0u8; 8];
    if cfg!(target_endian = "big") {
        (&mut b[2..8]).copy_from_slice(&a[..]);
    } else {
        (&mut b[0..6]).copy_from_slice(&a[..]);
    }
    u64::from_ne_bytes(b)
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

        let tri = encode_64(fs);
        let fs2 = decode_64(tri);

        assert_eq!(tri, 0);
        assert_eq!(fs, fs2);
    }

    #[test]
    fn powers_of_two() {
        let fs = (8.0f32, 128.0f32, 0.5f32);
        assert_eq!(fs, round_trip(fs));
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
    fn precision_floor() {
        let fs = (7.0f32, 8193.0f32, -1.0f32);
        let fsn = (-7.0f32, -8193.0f32, 1.0f32);
        assert_eq!((6.0, 8192.0, -0.0), round_trip(fs));
        assert_eq!((-6.0, -8192.0, 0.0), round_trip(fsn));
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

        assert_eq!((MAX, MAX, MAX), round_trip(fs));
        assert_eq!((MIN, MIN, MIN), round_trip(fsn));
        assert_eq!((MAX, MAX, MAX), decode_64(0x7FFD_FFF7_FFFF));
        assert_eq!((MIN, MIN, MIN), decode_64(0xFFFF_FFFF_FFFF));
    }

    #[test]
    fn inf_saturate() {
        use std::f32::INFINITY;
        let fs = (INFINITY, 0.0, 0.0);
        let fsn = (-INFINITY, 0.0, 0.0);

        assert_eq!((MAX, 0.0, 0.0), round_trip(fs));
        assert_eq!((MIN, 0.0, 0.0), round_trip(fsn));
        assert_eq!(0x7FFC0000003F, encode_64(fs));
        assert_eq!(0xFFFC0000003F, encode_64(fsn));
    }

    #[test]
    fn partial_saturate() {
        let fs = (99_999_999_999_999.0, 4294967296.0, -17179869184.0);
        let fsn = (-99_999_999_999_999.0, 4294967296.0, -17179869184.0);

        assert_eq!((MAX, 4294967296.0, -17179869184.0), round_trip(fs));
        assert_eq!((MIN, 4294967296.0, -17179869184.0), round_trip(fsn));
    }

    #[test]
    fn smallest_value() {
        let fs = (MIN_POSITIVE * 1.5, MIN_POSITIVE, MIN_POSITIVE * 0.50);
        let fsn = (-MIN_POSITIVE * 1.5, -MIN_POSITIVE, -MIN_POSITIVE * 0.50);

        assert_eq!((MIN_POSITIVE, -MIN_POSITIVE, 0.0), decode_64(0x600100000));
        assert_eq!((MIN_POSITIVE, MIN_POSITIVE, 0.0), round_trip(fs));
        assert_eq!((-MIN_POSITIVE, -MIN_POSITIVE, -0.0), round_trip(fsn));
    }

    #[test]
    fn underflow() {
        let fs = (MIN_POSITIVE * 0.5, -MIN_POSITIVE * 0.5, MIN_POSITIVE);
        assert_eq!(0x200000040, encode_64(fs));
        assert_eq!((0.0, -0.0, MIN_POSITIVE), round_trip(fs));
    }

    #[test]
    fn garbage_upper_bits_decode() {
        let fs1 = (4.0, -623.53, 12.3);
        let fs2 = (-63456254.2, 5235423.53, 54353.3);
        let fs3 = (-0.000000634, 0.00000000005, 0.00000000892);

        let n1 = encode_64(fs1);
        let n2 = encode_64(fs2);
        let n3 = encode_64(fs3);

        assert_eq!(decode_64(n1), decode_64(n1 | 0xffff_0000_0000_0000));
        assert_eq!(decode_64(n2), decode_64(n2 | 0xffff_0000_0000_0000));
        assert_eq!(decode_64(n3), decode_64(n3 | 0xffff_0000_0000_0000));
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
