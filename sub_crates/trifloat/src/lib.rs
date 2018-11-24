//! Encoding/decoding for a shared-exponent representation of three
//! positive floating point numbers.
//!
//! This is useful for e.g. compactly storing HDR colors.  Encoding is
//! relatively slow due to edge cases that need to be handled correctly,
//! but decoding is quite efficient.
//!
//! The encoding uses 9 bits of mantissa per number, and 5 bits for
//! the shared exponent.
//!
//! The largest representable number is 2^21 - 4096, and the smallest
//! representable non-zero number is 2^-19.  Values larger than the max
//! representable value saturate.
//!
//! Since the exponent is shared between the three values, the precision
//! of all three values depends on the largest of the three.  Epsilon is
//! 1/256.  Values are converted to trifloat by rounding, so the max error
//! introduced by conversion is half of epsilon.
//!
//! Warning: negative values and NaN's are _not_ supported nor handled in
//! any kind useful way.  There are debug-only assertions in place for
//! catching such values in the input floats to `encode_trifloat()`.

pub const MAX: f32 = 2093056.0;
pub const MIN: f32 = 0.0000019073486;

#[inline]
pub fn encode_trifloat(floats: (f32, f32, f32)) -> u32 {
    debug_assert!(
        floats.0 >= 0.0
            && floats.1 >= 0.0
            && floats.2 >= 0.0
            && !floats.0.is_nan()
            && !floats.1.is_nan()
            && !floats.2.is_nan(),
        "encode_trifloat(): encoding to tri-floats only works correctly for \
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
        let mut inv_multiplier = 512.0 / (exponent as f32).exp2();

        // Edge-case: make sure rounding pushes the largest value up
        // appropriately if needed.
        if (largest_value * inv_multiplier) + 0.5 >= 512.0 {
            exponent = (exponent + 1).max(-10).min(21);
            inv_multiplier = 512.0 / (exponent as f32).exp2();
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

#[inline]
pub fn decode_trifloat(trifloat: u32) -> (f32, f32, f32) {
    // Unpack values.
    let x = (trifloat >> (5 + 9 + 9)) & 0b111111111;
    let y = (trifloat >> (5 + 9)) & 0b111111111;
    let z = (trifloat >> 5) & 0b111111111;
    let e = trifloat & 0b11111;

    let multiplier = ((e as i32 - 10) as f32).exp2() * (1.0 / 512.0);

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
        decode_trifloat(encode_trifloat(floats))
    }

    #[test]
    fn all_zeros() {
        let fs = (0.0f32, 0.0f32, 0.0f32);

        let tri = encode_trifloat(fs);
        let fs2 = decode_trifloat(tri);

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
        assert_eq!(decode_trifloat(0xFFFFFFFF), (MAX, MAX, MAX),);
    }

    #[test]
    fn inf_saturate() {
        use std::f32::INFINITY;
        let fs = (INFINITY, 0.0, 0.0);

        assert_eq!(round_trip(fs), (MAX, 0.0, 0.0));
        assert_eq!(encode_trifloat(fs), 0xFF80001F,);
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
        assert_eq!(decode_trifloat(0x00_80_40_00), (MIN, MIN, 0.0));
    }

    #[test]
    fn underflow() {
        let fs = (MIN * 0.49, 0.0, 0.0);
        assert_eq!(encode_trifloat(fs), 0);
        assert_eq!(round_trip(fs), (0.0, 0.0, 0.0));
    }
}
