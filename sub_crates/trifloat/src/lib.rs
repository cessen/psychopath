//! Encoding/decoding for a 32-bit shared-exponent representation of three
//! positive floating point numbers.
//!
//! This is useful for e.g. compactly storing HDR colors.  The encoding
//! uses 9 bits of mantissa per number, and 5 bits for the shared
//! exponent.
//!
//! The largest representable number is 2^21 - 4096, and the smallest
//! representable non-zero number is 2^-19.
//!
//! Since the exponent is shared between the three values, the precision
//! of all three values depends on the largest of the three.  All integers
//! up to 512 can be represented exactly in the largest value.

/// Largest representable number.
pub const MAX: f32 = 2093056.0;

/// Smallest representable non-zero number.
pub const MIN: f32 = 0.0000019073486;

/// Difference between 1.0 and the next largest representable number.
pub const EPSILON: f32 = 1.0 / 256.0;

/// Encodes three floating point values into the trifloat format.
///
/// Floats that are larger than the max representable value in trifloat
/// will saturate.  Values are converted to trifloat by rounding, so the
/// max error introduced by this function is epsilon / 2.
///
/// Warning: negative values and NaN's are _not_ supported by the trifloat
/// format.  There are debug-only assertions in place to catch such
/// values in the input floats.  Infinity is also not supported in the
/// format, but will simply saturate to the largest representable value.
#[inline]
pub fn encode(floats: (f32, f32, f32)) -> u32 {
    debug_assert!(
        floats.0 >= 0.0
            && floats.1 >= 0.0
            && floats.2 >= 0.0
            && !floats.0.is_nan()
            && !floats.1.is_nan()
            && !floats.2.is_nan(),
        "trifloat::encode(): encoding to tri-floats only works correctly for \
         positive, non-NaN numbers, but the numbers passed were: ({}, \
         {}, {})",
        floats.0,
        floats.1,
        floats.2
    );

    // Find the largest of the three values.
    let largest_value = floats.0.max(floats.1.max(floats.2));
    if largest_value <= 0.0 {
        return 0;
    }

    // Calculate the exponent and 1.0/multiplier for encoding the values.
    let (exponent, inv_multiplier) = {
        let mut exponent = if largest_value > MAX {
            21
        } else {
            (largest_value.log2() as i32 + 1).max(-10).min(21)
        };
        let mut inv_multiplier = fiddle_exp2(-exponent + 9);

        // Edge-case: make sure rounding pushes the largest value up
        // appropriately if needed.
        if (largest_value * inv_multiplier) + 0.5 >= 512.0 {
            exponent = (exponent + 1).max(-10).min(21);
            inv_multiplier = fiddle_exp2(-exponent + 9);
        }

        (exponent, inv_multiplier)
    };

    // Quantize and encode values.
    let x = (floats.0 * inv_multiplier + 0.5).min(511.0) as u32 & 0b111111111;
    let y = (floats.1 * inv_multiplier + 0.5).min(511.0) as u32 & 0b111111111;
    let z = (floats.2 * inv_multiplier + 0.5).min(511.0) as u32 & 0b111111111;
    let e = (exponent + 10) as u32 & 0b11111;

    // Pack values into a u32.
    (x << (5 + 9 + 9)) | (y << (5 + 9)) | (z << 5) | e
}

/// Decodes a trifloat into three full floating point numbers.
///
/// This operation is lossless and cannot fail.
#[inline]
pub fn decode(trifloat: u32) -> (f32, f32, f32) {
    // Unpack values.
    let x = (trifloat >> (5 + 9 + 9)) & 0b111111111;
    let y = (trifloat >> (5 + 9)) & 0b111111111;
    let z = (trifloat >> 5) & 0b111111111;
    let e = trifloat & 0b11111;

    let multiplier = fiddle_exp2(e as i32 - 10 - 9);

    (
        x as f32 * multiplier,
        y as f32 * multiplier,
        z as f32 * multiplier,
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
        let fs = (7.0f32, 513.0f32, 1.0f32);
        assert_eq!(round_trip(fs), (8.0, 514.0, 2.0));
    }

    #[test]
    fn rounding_edge_case() {
        let fs = (1023.0f32, 0.0f32, 0.0f32);

        assert_eq!(round_trip(fs), (1024.0, 0.0, 0.0),);
    }

    #[test]
    fn saturate() {
        let fs = (9999999999.0, 9999999999.0, 9999999999.0);

        assert_eq!(round_trip(fs), (MAX, MAX, MAX));
        assert_eq!(decode(0xFFFFFFFF), (MAX, MAX, MAX),);
    }

    #[test]
    fn inf_saturate() {
        use std::f32::INFINITY;
        let fs = (INFINITY, 0.0, 0.0);

        assert_eq!(round_trip(fs), (MAX, 0.0, 0.0));
        assert_eq!(encode(fs), 0xFF80001F,);
    }

    #[test]
    fn partial_saturate() {
        let fs = (9999999999.0, 4096.0, 262144.0);

        assert_eq!(round_trip(fs), (MAX, 4096.0, 262144.0));
    }

    #[test]
    fn smallest_value() {
        let fs = (MIN, MIN * 0.5, MIN * 0.49);
        assert_eq!(round_trip(fs), (MIN, MIN, 0.0));
        assert_eq!(decode(0x00_80_40_00), (MIN, MIN, 0.0));
    }

    #[test]
    fn underflow() {
        let fs = (MIN * 0.49, 0.0, 0.0);
        assert_eq!(encode(fs), 0);
        assert_eq!(round_trip(fs), (0.0, 0.0, 0.0));
    }
}
