//! Encoding/decoding for a 48-bit shared-exponent representation of three
//! signed floating point numbers.
//!
//! This is useful for e.g. compactly storing HDR colors.  The encoding
//! uses 14 bits of mantissa per number (including the sign bit for each) and 6
//! bits for the shared exponent. The bit layout is [mantissa 1, mantissa 2,
//! mantissa 3, exponent].  The exponent is stored as an unsigned integer with
//! a bias of 32.  The mantissas are stored as a single leading sign bit and 13
//! bits of unsigned integer.
//!
//! The largest representable number is ?, and the smallest
//! representable positive number is ?.
//!
//! Since the exponent is shared between the three values, the precision
//! of all three values depends on the largest (in absolute value) of the
//! three.  All integers in the range [-8191, 8191] can be represented exactly
//! in the largest value.

/// Largest representable number.
pub const MAX: f32 = 35_180_077_121_536.0;

/// Smallest representable non-zero number.
pub const MIN_POSITIVE: f32 = 0.000_000_000_465_661_287;

pub const MIN: f32 = -35_180_077_121_536.0;

/// Difference between 1.0 and the next largest representable number.
pub const EPSILON: f32 = 1.0 / 4096.0;

const EXP_BIAS: i32 = 31 - 13;
const MIN_EXP: i32 = 0 - EXP_BIAS;
const MAX_EXP: i32 = 63 - EXP_BIAS;

/// Encodes three floating point values into a 48-bit trifloat format.
///
/// Note that even though the return value is a u64, only the lower 48
/// bits are used.
///
/// Floats that are larger than the max representable value in trifloat
/// will saturate.  Values are converted to trifloat by rounding, so the
/// max error introduced by this function is epsilon / 2.
///
/// Warning: NaN's are _not_ supported by the trifloat
/// format.  There are debug-only assertions in place to catch such
/// values in the input floats.  Infinity is also not supported in the
/// format, but will simply saturate to the largest-magnitude representable
/// value.
#[inline]
pub fn encode(floats: (f32, f32, f32)) -> u64 {
    debug_assert!(
        !floats.0.is_nan() && !floats.1.is_nan() && !floats.2.is_nan(),
        "trifloat::s48::encode(): encoding to signed 48-bit tri-floats only works correctly for \
         non-NaN numbers, but the numbers passed were: ({}, \
         {}, {})",
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
        if (largest_value * inv_multiplier).abs() + 0.5 >= 8191.0 {
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

/// Decodes a 48-bit trifloat into three full floating point numbers.
///
/// This operation is lossless and cannot fail.
#[inline]
pub fn decode(trifloat: u64) -> (f32, f32, f32) {
    // Unpack values.
    let x_sign = (trifloat >> 47) as u32;
    let x = (trifloat >> 34) & 0b111_11111_11111;
    let y_sign = ((trifloat >> 33) & 1) as u32;
    let y = (trifloat >> 20) & 0b111_11111_11111;
    let z_sign = ((trifloat >> 19) & 1) as u32;
    let z = (trifloat >> 6) & 0b111_11111_11111;
    let e = trifloat & 0b111_111;

    let multiplier = fiddle_exp2(e as i32 - EXP_BIAS - 13);

    (
        f32::from_bits((x as f32 * multiplier).to_bits() | (x_sign << 31)),
        f32::from_bits((y as f32 * multiplier).to_bits() | (y_sign << 31)),
        f32::from_bits((z as f32 * multiplier).to_bits() | (z_sign << 31)),
    )
}

/// Calculates 2.0^exp using IEEE bit fiddling.
///
/// Only works for integer exponents in the range [-126, 127]
/// due to IEEE 32-bit float limits.
#[inline(always)]
fn fiddle_exp2(exp: i32) -> f32 {
    use std::f32;
    f32::from_bits(((exp + 127) as u32) << 23)
}

/// Calculates a floor(log2(n)) using IEEE bit fiddling.
///
/// Because of IEEE floating point format, infinity and NaN
/// floating point values return 128, and subnormal numbers always
/// return -127.  These particular behaviors are not, of course,
/// mathemetically correct, but are actually desireable for the
/// calculations in this library.
#[inline(always)]
fn fiddle_log2(n: f32) -> i32 {
    use std::f32;
    ((f32::to_bits(n) >> 23) & 0b1111_1111) as i32 - 127
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
        for n in 0..=512 {
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

        assert_eq!(round_trip(fs), (MIN_POSITIVE, MIN_POSITIVE, 0.0));
        assert_eq!(round_trip(fsn), (-MIN_POSITIVE, -MIN_POSITIVE, -0.0));
        assert_eq!(decode(0x600100000), (MIN_POSITIVE, -MIN_POSITIVE, 0.0));
    }

    #[test]
    fn underflow() {
        let fs = (MIN_POSITIVE * 0.49, -MIN_POSITIVE * 0.49, 0.0);
        assert_eq!(encode(fs), 0x200000000);
        assert_eq!(round_trip(fs), (0.0, -0.0, 0.0));
    }
}
