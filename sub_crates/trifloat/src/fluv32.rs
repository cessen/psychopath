//! Encoding/decoding for the 32-bit FloatLuv color format.
//!
//! This encoding is based on, but is slightly different than, the 32-bit
//! LogLuv format from the paper "Overcoming Gamut and Dynamic Range
//! Limitations in Digital Images" by Greg Ward:
//!
//! * It uses the same uv chroma storage approach, but with *very* slightly
//!   tweaked scales to allow perfect representation of E.
//! * It uses uses a floating point rather than log encoding to store
//!   luminance, mainly for the sake of faster decoding.
//! * It also omits the sign bit of LogLuv, foregoing negative luminance
//!   capabilities.
//!
//! Compared to LogLuv, this format's chroma precision is the same and its
//! luminance precision is better, but its luminance *range* is smaller.
//! The supported luminance range is still substantial, however (see
//! "Luminance details" below).
//!
//! Like the LogLuv format, this is an absolute rather than relative color
//! encoding, and as such takes CIE XYZ triplets as input.  It is *not*
//! designed to take arbitrary floating point triplets, and will perform poorly
//! if e.g. passed RGB values.
//!
//! The bit layout is:
//!
//! 1. luminance exponent (6 bits, bias 27)
//! 2. luminance mantissa (10 stored bits, 11 bits precision)
//! 3. u (8 bits)
//! 4. v (8 bits)
//!
//! ## Luminance details
//!
//! Quoting Greg Ward about luminance ranges:
//!
//! > The sun is about `10^8 cd/m^2`, and the underside of a rock on a moonless
//! > night is probably around `10^-6` or so [...]
//!
//! The luminance range of this format is from about `10^11` on the brightest
//! end, to about `10^-8` on the darkest (excluding zero itself, which can also
//! be stored).
//!
//! That gives this format almost five orders of magnitude more dynamic range
//! than is likely to be needed for any practical situation.  Moreover, that
//! extra range is split between both the high and low end, giving a
//! comfortable buffer on both ends for extreme situations.
//!
//! Like the LogLuv format, the input CIE Y value is taken directly as the
//! luminance value.

#![allow(clippy::cast_lossless)]

const EXP_BIAS: i32 = 27;

/// The scale factor of the quantized U component.
pub const U_SCALE: f32 = 817.0 / 2.0;

/// The scale factor of the quantized V component.
pub const V_SCALE: f32 = 1235.0 / 3.0;

/// Largest representable Y component.
pub const Y_MAX: f32 = ((1u64 << (64 - EXP_BIAS)) - (1u64 << (64 - EXP_BIAS - 11))) as f32;

/// Smallest representable non-zero Y component.
pub const Y_MIN: f32 = 1.0 / (1u64 << (EXP_BIAS - 1)) as f32;

/// Difference between 1.0 and the next largest representable Y value.
pub const Y_EPSILON: f32 = 1.0 / 1024.0;

/// Encodes from CIE XYZ to 32-bit FloatLuv.
#[inline]
pub fn encode(xyz: (f32, f32, f32)) -> u32 {
    debug_assert!(
        xyz.0 >= 0.0
            && xyz.1 >= 0.0
            && xyz.2 >= 0.0
            && !xyz.0.is_nan()
            && !xyz.1.is_nan()
            && !xyz.2.is_nan(),
        "trifloat::fluv32::encode(): encoding to fluv32 only \
         works correctly for positive, non-NaN numbers, but the numbers passed \
         were: ({}, {}, {})",
        xyz.0,
        xyz.1,
        xyz.2
    );

    // Calculates the 16-bit encoding of the UV values for the given XYZ input.
    #[inline(always)]
    fn encode_uv(xyz: (f32, f32, f32)) -> u32 {
        let s = xyz.0 + (15.0 * xyz.1) + (3.0 * xyz.2);

        // The `+ 0.5` is for rounding, and is not part of the normal equation.
        // The minimum value of 1.0 for v is to avoid a possible divide by zero
        // when decoding.  A value less than 1.0 is outside the real colors,
        // so we don't need to store it anyway.
        let u = (((4.0 * U_SCALE) * xyz.0 / s) + 0.5).max(0.0).min(255.0);
        let v = (((9.0 * V_SCALE) * xyz.1 / s) + 0.5).max(1.0).min(255.0);

        ((u as u32) << 8) | (v as u32)
    };

    let y_bits = xyz.1.to_bits();
    let exp = (y_bits >> 23) as i32 - 127 + EXP_BIAS;

    if exp <= 0 {
        // Special case: black.
        encode_uv((1.0, 1.0, 1.0))
    } else if exp > 63 {
        if xyz.1.is_infinite() {
            // Special case: infinity.  In this case, we don't have any
            // reasonable basis for calculating chroma, so just return
            // the brightest white.
            0xffff0000 | encode_uv((1.0, 1.0, 1.0))
        } else {
            // Special case: non-infinite, but brighter luma than can be
            // represented.  Return the correct chroma, and the brightest luma.
            0xffff0000 | encode_uv(xyz)
        }
    } else {
        // Common case.
        ((exp as u32) << 26) | ((y_bits & 0x07fe000) << 3) | encode_uv(xyz)
    }
}

/// Decodes from 32-bit FloatLuv to CIE XYZ.
#[inline]
pub fn decode(fluv32: u32) -> (f32, f32, f32) {
    // Check for zero.
    if fluv32 & 0xffff0000 == 0 {
        return (0.0, 0.0, 0.0);
    }

    // Unpack values.
    let l_exp = fluv32 >> 26;
    let l_mant = (fluv32 >> 16) & 0x3ff;
    let u = ((fluv32 >> 8) & 0xff) as f32; // Range 0.0-255.0
    let v = (fluv32 & 0xff) as f32; // Range 1.0-255.0

    // Calculate y.
    let y = f32::from_bits(((l_exp + 127 - EXP_BIAS as u32) << 23) | (l_mant << 13));

    // Calculate x and z.
    // This is re-worked from the original equations, to allow a bunch of stuff
    // to cancel out and avoid operations.  It makes the underlying equations a
    // bit non-obvious.
    // We also roll the U/V_SCALE application into the final x and z formulas,
    // since some of that cancels out as well, and all of it can be avoided at
    // runtime that way.
    let tmp = y / v;
    let x = tmp * ((2.25 * V_SCALE / U_SCALE) * u); // y * (9u / 4v)
    let z = tmp * ((3.0 * V_SCALE) - ((0.75 * V_SCALE / U_SCALE) * u) - (5.0 * v)); // y * ((12 - 3u - 20v) / 4v)

    (x, y, z.max(0.0))
}

/// Decodes from 32-bit FloatLuv to Yuv.
///
/// The Y component is the luminance, and is simply the Y from CIE XYZ.
///
/// The u and v components are the CIE LUV u' and v' chromaticity coordinates,
/// but returned as `u8`s, and scaled by `U_SCALE` and `V_SCALE` respectively
/// to fit the range 0-255.
#[inline]
pub fn decode_yuv(fluv32: u32) -> (f32, u8, u8) {
    // Check for zero.
    if fluv32 & 0xffff0000 == 0 {
        return (0.0, (fluv32 >> 8) as u8, fluv32 as u8);
    }

    // Calculate y.
    let l_exp = fluv32 >> 26;
    let l_mant = (fluv32 >> 16) & 0x3ff;
    let y = f32::from_bits(((l_exp + 127 - EXP_BIAS as u32) << 23) | (l_mant << 13));

    (y, (fluv32 >> 8) as u8, fluv32 as u8)
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

        assert_eq!(0x000056c3, tri);
        assert_eq!(fs, fs2);
    }

    #[test]
    fn all_ones() {
        let fs = (1.0f32, 1.0f32, 1.0f32);

        let tri = encode(fs);
        let fs2 = decode(tri);

        assert_eq!(0x6c0056c3, tri);

        assert!((fs.0 - fs2.0).abs() < 0.0000001);
        assert_eq!(fs.1, fs2.1);
        assert!((fs.2 - fs2.2).abs() < 0.0000001);
    }

    #[test]
    fn powers_of_two() {
        let mut n = 0.25;
        for _ in 0..20 {
            let a = (n as f32, n as f32, n as f32);
            let b = round_trip(a);

            let rd0 = 2.0 * (a.0 - b.0).abs() / (a.0 + b.0);
            let rd2 = 2.0 * (a.2 - b.2).abs() / (a.2 + b.2);

            assert_eq!(a.1, b.1);
            assert!(rd0 < 0.01);
            assert!(rd2 < 0.01);

            n *= 2.0;
        }
    }

    #[test]
    fn accuracy_01() {
        let mut n = 1.0;
        for _ in 0..1024 {
            let a = (n as f32, n as f32, n as f32);
            let b = round_trip(a);

            let rd0 = 2.0 * (a.0 - b.0).abs() / (a.0 + b.0);
            let rd2 = 2.0 * (a.2 - b.2).abs() / (a.2 + b.2);

            assert_eq!(a.1, b.1);
            assert!(rd0 < 0.01);
            assert!(rd2 < 0.01);

            n += 1.0 / 1024.0;
        }
    }

    #[test]
    #[should_panic]
    fn accuracy_02() {
        let mut n = 1.0;
        for _ in 0..2048 {
            let a = (n as f32, n as f32, n as f32);
            let b = round_trip(a);
            assert_eq!(a.1, b.1);
            n += 1.0 / 2048.0;
        }
    }

    #[test]
    fn integers() {
        for n in 1..=512 {
            let a = (n as f32, n as f32, n as f32);
            let b = round_trip(a);

            let rd0 = 2.0 * (a.0 - b.0).abs() / (a.0 + b.0);
            let rd2 = 2.0 * (a.2 - b.2).abs() / (a.2 + b.2);

            assert_eq!(a.1, b.1);
            assert!(rd0 < 0.01);
            assert!(rd2 < 0.01);
        }
    }

    #[test]
    fn precision_floor() {
        let fs = (2049.0f32, 2049.0f32, 2049.0f32);
        assert_eq!(2048.0, round_trip(fs).1);
    }

    #[test]
    fn decode_yuv_01() {
        let fs = (1.0, 1.0, 1.0);
        let a = encode(fs);

        assert_eq!((1.0, 0x56, 0xc3), decode_yuv(a));
    }

    #[test]
    fn saturate_y() {
        let fs = (1.0e+20, 1.0e+20, 1.0e+20);

        assert_eq!(Y_MAX, round_trip(fs).1);
        assert_eq!(Y_MAX, decode(0xFFFFFFFF).1);
    }

    #[test]
    fn inf_saturate() {
        use std::f32::INFINITY;
        let fs = (INFINITY, INFINITY, INFINITY);

        assert_eq!(Y_MAX, round_trip(fs).1);
        assert_eq!(0xffff56c3, encode(fs));
    }

    #[test]
    fn smallest_value() {
        let a = (Y_MIN, Y_MIN, Y_MIN);
        let b = (Y_MIN * 0.99, Y_MIN * 0.99, Y_MIN * 0.99);
        assert_eq!(Y_MIN, round_trip(a).1);
        assert_eq!(0.0, round_trip(b).1);
    }

    #[test]
    fn underflow() {
        let fs = (Y_MIN * 0.99, Y_MIN * 0.99, Y_MIN * 0.99);
        assert_eq!(0x000056c3, encode(fs));
        assert_eq!((0.0, 0.0, 0.0), round_trip(fs));
    }

    #[test]
    fn negative_z_impossible() {
        for y in 0..1024 {
            let fs = (1.0, 1.0 + (y as f32 / 4096.0), 0.0);
            let fs2 = round_trip(fs);
            assert!(fs2.2 >= 0.0);
        }
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
