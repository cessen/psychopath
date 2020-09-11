//! Encoding/decoding for a specialized HDR RGB 32-bit storage format.
//!
//! The motivation for this format is to separate out the luma of
//! the color from its chromaticity, in the same spirit as most
//! image and video compression approaches, and then allocate more
//! bits to storing the luma component since that's what the human
//! eye is most sensitive to.
//!
//! This encoding first transforms the color into a Y (luma) component
//! and two chroma components (green-magenta and red-blue), and then
//! fiddles those components into a special 32-bit format.
//! The Y component is stored as an unsigned float, with 6 bits of
//! exponent and 10 bits of mantissa.  The two chroma components are
//! each stored as 8-bit integers.
//!
//! The layout is:
//!
//! 1. Y-exponent: 6 bits
//! 2. Y-mantissa: 10 bits
//! 3. Green-Magenta: 8 bits
//! 4. Red-Blue: 8 bits
//!
//! The Y-mantissa has an implicit leading one, giving 11 bits of
//! precision.

use crate::clamp_0_1;

const EXP_BIAS: i32 = 23;

/// The largest value this format can store.
///
/// More precisely, this is the largest value that can be *reliably*
/// stored.
///
/// This can be exceeded by individual channels in limited cases due
/// to the color transform used.  But values *at least* this large
/// can be relied on.
pub const MAX: f32 = ((1u64 << (63 - EXP_BIAS)) - (1 << (52 - EXP_BIAS))) as f32;

/// The smallest non-zero value this format can store.
///
/// Note that since this is effectively a shared-exponent format,
/// the numerical precision of all channels depends on the magnitude
/// of the over-all RGB color.
pub const MIN: f32 = 1.0 / (1 << (EXP_BIAS - 2)) as f32;

/// Encodes three floating point RGB values into a packed 32-bit format.
///
/// Warning: negative values and NaN's are _not_ supported.  There are
/// debug-only assertions in place to catch such values in the input
/// floats.  Infinity in any channel will saturate the whole color to
/// the brightest representable white.
#[inline]
pub fn encode(floats: (f32, f32, f32)) -> u32 {
    debug_assert!(
        floats.0 >= 0.0
            && floats.1 >= 0.0
            && floats.2 >= 0.0
            && !floats.0.is_nan()
            && !floats.1.is_nan()
            && !floats.2.is_nan(),
        "trifloat::rgb32::encode(): encoding to unsigned tri-floats only \
         works correctly for positive, non-NaN numbers, but the numbers passed \
         were: ({}, {}, {})",
        floats.0,
        floats.1,
        floats.2
    );

    // Convert to Y/Green-Magenta/Red-Blue components.
    let u = floats.0 + floats.2;
    let y = (u * 0.5) + floats.1;
    let green_magenta = clamp_0_1(floats.1 / y);
    let red_blue = if u > 0.0 {
        clamp_0_1(floats.0 / u)
    } else {
        0.5
    };

    // Bit-fiddle to get the float components of Y.
    // This assumes we're working with a standard 32-bit IEEE float.
    let y_ieee_bits = y.to_bits();
    let y_mantissa = (y_ieee_bits >> 13) & 0b11_1111_1111;
    let y_exp = ((y_ieee_bits >> 23) & 0b1111_1111) as i32 - 127;

    // Encode Cg and Cr as 8-bit integers.
    let gm_8bit = ((green_magenta * 254.0) + 0.5) as u8;
    let rb_8bit = ((red_blue * 254.0) + 0.5) as u8;

    // Pack values into a u32 and return.
    if y_exp <= (0 - EXP_BIAS) {
        // Early-out corner-case:
        // Luma is so dark that it will be zero at our precision,
        // and hence black.
        0
    } else if y_exp >= (63 - EXP_BIAS) {
        // Corner-case:
        // Luma is so bright that it exceeds our max value, so saturate
        // the luma.
        if y.is_infinite() {
            // If luma is infinity, our chroma values are bogus, so
            // just go with white.
            0xffff7f7f
        } else {
            0xffff0000 | ((gm_8bit as u32) << 8) | rb_8bit as u32
        }
    } else {
        // Common case.
        let exp = (y_exp + EXP_BIAS) as u32;
        (exp << 26) | (y_mantissa << 16) | ((gm_8bit as u32) << 8) | rb_8bit as u32
    }
}

/// Decodes a packed HDR RGB 32-bit format into three full
/// floating point RGB numbers.
#[inline]
pub fn decode(packed_rgb: u32) -> (f32, f32, f32) {
    // Pull out Y, Green-Magenta, and Red-Blue from the packed
    // bits.
    let y = {
        let exp = (packed_rgb & 0xfc00_0000) >> 26;
        if exp == 0 {
            0.0
        } else {
            f32::from_bits(
                ((exp + (127 - EXP_BIAS as u32)) << 23) | ((packed_rgb & 0x03ff_0000) >> 3),
            )
        }
    };
    let green_magenta = {
        let gm_8bit = (packed_rgb >> 8) & 0xff;
        (gm_8bit as f32) * (1.0 / 254.0)
    };
    let red_blue = {
        let rb_8bit = packed_rgb & 0xff;
        (rb_8bit as f32) * (1.0 / 254.0)
    };

    // Convert back to RGB.
    let g = y * green_magenta;
    let u = (y - g) * 2.0;
    let r = u * red_blue;
    let b = u - r;

    (r, g, b)
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
        let mut n = 1.0f32 / 65536.0;
        for _ in 0..48 {
            let fs = (n, n, n);

            assert_eq!(fs, round_trip(fs));
            n *= 2.0;
        }
    }

    #[test]
    fn integers() {
        let mut n = 1.0f32;
        for _ in 0..2048 {
            let fs = (n, n, n);

            assert_eq!(fs, round_trip(fs));
            n += 1.0;
        }
    }

    #[test]
    fn color_saturation() {
        let fs1 = (1.0, 0.0, 0.0);
        let fs2 = (0.0, 1.0, 0.0);
        let fs3 = (0.0, 0.0, 1.0);

        assert_eq!(fs1, round_trip(fs1));
        assert_eq!(fs2, round_trip(fs2));
        assert_eq!(fs3, round_trip(fs3));
    }

    #[test]
    fn num_saturate() {
        let fs = (10000000000000.0, 10000000000000.0, 10000000000000.0);

        assert_eq!((MAX, MAX, MAX), round_trip(fs));
    }

    #[test]
    fn num_inf_saturate() {
        use std::f32::INFINITY;
        let fs = (INFINITY, INFINITY, INFINITY);

        assert_eq!((MAX, MAX, MAX), round_trip(fs));
    }

    #[test]
    fn num_partial_saturate() {
        let fs1 = (10000000000000.0, 0.0, 0.0);
        let fs2 = (0.0, 10000000000000.0, 0.0);
        let fs3 = (0.0, 0.0, 10000000000000.0);

        assert_eq!((MAX * 4.0, 0.0, 0.0), round_trip(fs1));
        assert_eq!((0.0, MAX * 2.0, 0.0), round_trip(fs2));
        assert_eq!((0.0, 0.0, MAX * 4.0), round_trip(fs3));
    }

    #[test]
    fn largest_value() {
        let fs1 = (MAX, MAX, MAX);
        let fs2 = (MAX, 0.0, 0.0);
        let fs3 = (0.0, MAX, 0.0);
        let fs4 = (0.0, 0.0, MAX);

        assert_eq!(fs1, round_trip(fs1));
        assert_eq!(fs2, round_trip(fs2));
        assert_eq!(fs3, round_trip(fs3));
        assert_eq!(fs4, round_trip(fs4));
    }

    #[test]
    fn smallest_value() {
        let fs1 = (MIN, MIN, MIN);
        let fs2 = (MIN, 0.0, 0.0);
        let fs3 = (0.0, MIN, 0.0);
        let fs4 = (0.0, 0.0, MIN);

        assert_eq!(fs1, round_trip(fs1));
        assert_eq!(fs2, round_trip(fs2));
        assert_eq!(fs3, round_trip(fs3));
        assert_eq!(fs4, round_trip(fs4));
    }

    #[test]
    fn underflow() {
        let fs1 = (MIN * 0.5, 0.0, 0.0);
        let fs2 = (0.0, MIN * 0.25, 0.0);
        let fs3 = (0.0, 0.0, MIN * 0.5);

        assert_eq!(round_trip(fs1), (0.0, 0.0, 0.0));
        assert_eq!(round_trip(fs2), (0.0, 0.0, 0.0));
        assert_eq!(round_trip(fs3), (0.0, 0.0, 0.0));
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
