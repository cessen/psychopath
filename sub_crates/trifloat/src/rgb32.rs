//! Encoding/decoding for specialized HDR RGB 32-bit storage format.
//!
//! The motivation for this format is to separate out the luma of
//! the color from its chromaticity, in the same spirit as most
//! image and video compression approaches, and then allocate more
//! data to the luma component since that's what the human eye is
//! most sensitive to.
//!
//! This encoding first transforms into YCoCg colorspace, and then
//! fiddles the resulting Y, Co, and Cg components into a special
//! 32-bit format.  The Y component is stored as an unsigned float,
//! with 6 bits of exponent and 10 bits of mantissa.  The Co and Cg
//! components are stored as 8-bit integers.
//!
//! The layout is:
//!
//! 1. Y-exponent: 6 bits
//! 2. Y-mantissa: 10 bits
//! 3. Co: 8 bits
//! 4. Cg: 8 bits
//!
//! The Y component follows the convention of a mantissa with an
//! implicit leading one, giving it 11 bits of precision.  The
//! exponent has a bias of 24.

/// Encodes three floating point RGB values into a packed 32-bit format.
///
/// Warning: negative values and NaN's are _not_ supported.  There are
/// debug-only assertions in place to catch such values in the input
/// floats.
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

    // Convert to YCoCg colorspace.
    let y = (floats.0 * 0.25) + (floats.1 * 0.5) + (floats.2 * 0.25);
    let co = (floats.0 * 0.5) + (floats.2 * -0.5);
    let cg = (floats.0 * -0.25) + (floats.1 * 0.5) + (floats.2 * -0.25);

    if y <= 0.0 {
        // Corner case: black.
        return 0;
    } else if y.is_infinite() {
        // Corner case: infinite white.
        return 0xffff7f7f;
    }

    // Encode Co and Cg as 8-bit integers.
    // Note that the max values for each of these will get clamped
    // very slightly, but that represents extremely saturated
    // colors, where the human eye is not very sensitive to chroma
    // differences anyway.  And the trade-off is that we can
    // represent 0.0 (completely unsaturated, no chroma) exactly.
    let inv_y = 1.0 / y;
    let co_8bit = ((co * inv_y * 63.5) + 127.5).min(255.0).max(0.0) as u8;
    let cg_8bit = ((cg * inv_y * 127.0) + 127.5).min(255.0).max(0.0) as u8;

    // Bit-fiddle to get the float components of Y.
    // This assumes we're working with a standard 32-bit IEEE float.
    let y_ieee_bits = y.to_bits();
    let y_mantissa = (y_ieee_bits >> 13) & 0b11_1111_1111;
    let y_exp = ((y_ieee_bits >> 23) & 0b1111_1111) as i32 - 127;

    // Pack values into a u32 and return.
    if y_exp <= -24 {
        // Corner-case:
        // Luma is so dark that it will be zero at our precision,
        // and hence black.
        0
    } else if y_exp >= 40 {
        dbg!();
        // Corner-case:
        // Luma is so bright that it exceeds our max value, so saturate
        // the luma.
        0xffff0000 | ((co_8bit as u32) << 8) | cg_8bit as u32
    } else {
        // Common case.
        let exp = (y_exp + 24) as u32;
        (exp << 26) | (y_mantissa << 16) | ((co_8bit as u32) << 8) | cg_8bit as u32
    }
}

/// Decodes a packed HDR RGB 32-bit format into three full
/// floating point RGB numbers.
///
/// This operation is lossless and cannot fail.
#[inline]
pub fn decode(packed_rgb: u32) -> (f32, f32, f32) {
    // Reconstruct Y, Co, and Cg from the packed bits.
    let y = {
        let exp = (packed_rgb & 0xfc00_0000) >> 26;
        if exp == 0 {
            0.0
        } else {
            f32::from_bits(((exp + 103) << 23) | ((packed_rgb & 0x03ff_0000) >> 3))
        }
    };
    let co = {
        let co_8bit = (packed_rgb >> 8) & 0xff;
        ((co_8bit as f32) - 127.0) * (1.0 / 63.5) * y
    };
    let cg = {
        let cg_8bit = packed_rgb & 0xff;
        ((cg_8bit as f32) - 127.0) * (1.0 / 127.0) * y
    };

    // Convert back to RGB.
    let tmp = y - cg;
    let r = (tmp + co).max(0.0);
    let g = (y + cg).max(0.0);
    let b = (tmp - co).max(0.0);

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
    fn full_saturation() {
        let fs1 = (1.0, 0.0, 0.0);
        let fs2 = (0.0, 1.0, 0.0);
        let fs3 = (0.0, 0.0, 1.0);

        assert_eq!(fs1, round_trip(fs1));
        assert_eq!(fs2, round_trip(fs2));
        assert_eq!(fs3, round_trip(fs3));
    }

    #[test]
    fn saturate() {
        let fs = (10000000000000.0, 10000000000000.0, 10000000000000.0);

        assert_eq!(
            (1098974760000.0, 1098974760000.0, 1098974760000.0),
            round_trip(fs)
        );
    }

    #[test]
    fn inf_saturate() {
        use std::f32::INFINITY;
        let fs = (INFINITY, INFINITY, INFINITY);

        assert_eq!(
            (1098974760000.0, 1098974760000.0, 1098974760000.0),
            round_trip(fs)
        );
    }

    #[test]
    fn partial_saturate() {
        let fs1 = (10000000000000.0, 0.0, 0.0);
        let fs2 = (0.0, 10000000000000.0, 0.0);
        let fs3 = (0.0, 0.0, 10000000000000.0);

        assert_eq!(round_trip(fs1), (4395899000000.0, 0.0, 0.0));
        assert_eq!(round_trip(fs2), (0.0, 2197949500000.0, 0.0));
        assert_eq!(round_trip(fs3), (0.0, 0.0, 4395899000000.0));
    }

    // #[test]
    // fn accuracy() {
    //     let mut n = 1.0;
    //     for _ in 0..256 {
    //         let (x, _, _) = round_trip((n, 0.0, 0.0));
    //         assert_eq!(n, x);
    //         n += 1.0 / 256.0;
    //     }
    // }

    // #[test]
    // fn rounding() {
    //     let fs = (7.0f32, 513.0f32, 1.0f32);
    //     assert_eq!(round_trip(fs), (8.0, 514.0, 2.0));
    // }

    // #[test]
    // fn rounding_edge_case() {
    //     let fs = (1023.0f32, 0.0f32, 0.0f32);

    //     assert_eq!(round_trip(fs), (1024.0, 0.0, 0.0),);
    // }

    // #[test]
    // fn smallest_value() {
    //     let fs = (MIN, MIN * 0.5, MIN * 0.49);
    //     assert_eq!(round_trip(fs), (MIN, MIN, 0.0));
    //     assert_eq!(decode(0x00_80_40_00), (MIN, MIN, 0.0));
    // }

    // #[test]
    // fn underflow() {
    //     let fs = (MIN * 0.49, 0.0, 0.0);
    //     assert_eq!(encode(fs), 0);
    //     assert_eq!(round_trip(fs), (0.0, 0.0, 0.0));
    // }

    // #[test]
    // #[should_panic]
    // fn nans_01() {
    //     encode((std::f32::NAN, 0.0, 0.0));
    // }

    // #[test]
    // #[should_panic]
    // fn nans_02() {
    //     encode((0.0, std::f32::NAN, 0.0));
    // }

    // #[test]
    // #[should_panic]
    // fn nans_03() {
    //     encode((0.0, 0.0, std::f32::NAN));
    // }

    // #[test]
    // #[should_panic]
    // fn negative_01() {
    //     encode((-1.0, 0.0, 0.0));
    // }

    // #[test]
    // #[should_panic]
    // fn negative_02() {
    //     encode((0.0, -1.0, 0.0));
    // }

    // #[test]
    // #[should_panic]
    // fn negative_03() {
    //     encode((0.0, 0.0, -1.0));
    // }

    // #[test]
    // fn negative_04() {
    //     encode((-0.0, -0.0, -0.0));
    // }
}
