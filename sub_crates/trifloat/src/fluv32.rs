//! Encoding/decoding for the 32-bit FLuv32 color format.
//!
//! This encoding is based on, but is slightly different than, the 32-bit
//! LogLuv format from the paper "Overcoming Gamut and Dynamic Range
//! Limitations in Digital Images" by Greg Ward:
//!
//! * It uses the same uv chroma storage approach, but with *very* slightly
//!   tweaked scales to allow perfect representation of E.
//! * It uses a floating point rather than log encoding to store luminance,
//!   mainly for the sake of faster decoding.
//! * Unlike LogLuv, this format's dynamic range is biased to put more of it
//!   above 1.0 (see Luminance details below).
//! * It omits the sign bit of LogLuv, foregoing negative luminance
//!   capabilities.
//!
//! This format has the same chroma precision, very slightly improved luminance
//! precision, and the same 127-stops of dynamic range as LogLuv.
//!
//! Like the LogLuv format, this is an absolute rather than relative color
//! encoding, and as such takes CIE XYZ triplets as input.  It is *not*
//! designed to take arbitrary floating point triplets, and will perform poorly
//! if e.g. passed RGB values.
//!
//! The bit layout is (from most to least significant bit):
//!
//! * 7 bits: luminance exponent (bias 42)
//! * 9 bits: luminance mantissa (implied leading 1, for 10 bits precision)
//! * 8 bits: u'
//! * 8 bits: v'
//!
//! ## Luminance details
//!
//! Quoting Greg Ward about luminance ranges:
//!
//! > The sun is about `10^8 cd/m^2`, and the underside of a rock on a moonless
//! > night is probably around `10^-6` or so [...]
//!
//! See also Wikipedia's
//! [list of luminance levels](https://en.wikipedia.org/wiki/Orders_of_magnitude_(luminance)).
//!
//! The luminance range of the original LogLuv is about `10^-19` to `10^19`,
//! splitting the range evenly above and below 1.0.  Given the massive dynamic
//! range, and the fact that all day-to-day luminance levels trivially fit
//! within that, that's a perfectly reasonable choice.
//!
//! However, there are some stellar events like supernovae that are trillions
//! of times brighter than the sun, and would exceed `10^19`.  Conversely,
//! there likely isn't much use for significantly smaller values than `10^-10`
//! or so.  So although recording supernovae in physical units with a graphics
//! format seems unlikely, it doesn't hurt to bias the range towards brighter
//! luminance levels.
//!
//! With that in mind, FLuv32 uses an exponent bias of 42, putting twice as
//! many stops of dynamic range above 1.0 as below it, giving a luminance range
//! of roughly `10^-13` to `10^25`.  It's the same dynamic range as
//! LogLuv (about 127 stops), but with more of that range placed above 1.0.
//!
//! Like typical floating point, the mantissa is treated as having an implicit
//! leading 1, giving it an extra bit of precision.  The smallest exponent
//! indicates a value of zero, and a valid encoding should also set the
//! mantissa to zero in that case (denormal numbers are not supported).  The
//! largest exponent is given no special treatment (no infinities, no NaN).

#![allow(clippy::cast_lossless)]

const EXP_BIAS: i32 = 42;
const BIAS_OFFSET: u32 = 127 - EXP_BIAS as u32;

/// The scale factor of the quantized U component.
pub const U_SCALE: f32 = 817.0 / 2.0;

/// The scale factor of the quantized V component.
pub const V_SCALE: f32 = 1235.0 / 3.0;

/// Largest representable Y component.
pub const Y_MAX: f32 = ((1u128 << (128 - EXP_BIAS)) - (1u128 << (128 - EXP_BIAS - 10))) as f32;

/// Smallest representable non-zero Y component.
pub const Y_MIN: f32 = 1.0 / (1u128 << (EXP_BIAS - 1)) as f32;

/// Difference between 1.0 and the next largest representable Y value.
pub const Y_EPSILON: f32 = 1.0 / 512.0;

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

    let y_bits = xyz.1.to_bits() & 0x7fffffff;

    if y_bits < ((BIAS_OFFSET + 1) << 23) {
        // Special case: black.
        encode_uv((1.0, 1.0, 1.0))
    } else if y_bits >= ((BIAS_OFFSET + 128) << 23) {
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
        (((y_bits - (BIAS_OFFSET << 23)) << 2) & 0xffff0000) | encode_uv(xyz)
    }
}

/// Decodes from 32-bit FloatLuv to CIE XYZ.
#[inline]
pub fn decode(fluv32: u32) -> (f32, f32, f32) {
    // Unpack values.
    let (y, u, v) = decode_yuv(fluv32);
    let u = u as f32;
    let v = v as f32;

    // Calculate x and z.
    // This is re-worked from the original equations, to allow a bunch of stuff
    // to cancel out and avoid operations.  It makes the underlying equations a
    // bit non-obvious.
    // We also roll the U/V_SCALE application into the final x and z formulas,
    // since some of that cancels out as well, and all of it can be avoided at
    // runtime that way.
    const VU_RATIO: f32 = V_SCALE / U_SCALE;
    let tmp = y / v;
    let x = tmp * ((2.25 * VU_RATIO) * u); // y * (9u / 4v)
    let z = tmp * ((3.0 * V_SCALE) - ((0.75 * VU_RATIO) * u) - (5.0 * v)); // y * ((12 - 3u - 20v) / 4v)

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
    let y = f32::from_bits(((fluv32 & 0xffff0000) >> 2) + (BIAS_OFFSET << 23));
    let u = (fluv32 >> 8) as u8;
    let v = fluv32 as u8;

    if fluv32 <= 0xffff {
        (0.0, u, v)
    } else {
        (y, u, v)
    }
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

        assert_eq!(fs.1, fs2.1);
        assert!((fs.0 - fs2.0).abs() < 0.0000001);
        assert!((fs.2 - fs2.2).abs() < 0.0000001);
        assert_eq!(0x540056c3, tri);
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
        for _ in 0..512 {
            let a = (n as f32, n as f32, n as f32);
            let b = round_trip(a);

            let rd0 = 2.0 * (a.0 - b.0).abs() / (a.0 + b.0);
            let rd2 = 2.0 * (a.2 - b.2).abs() / (a.2 + b.2);

            assert_eq!(a.1, b.1);
            assert!(rd0 < 0.01);
            assert!(rd2 < 0.01);

            n += 1.0 / 512.0;
        }
    }

    #[test]
    #[should_panic]
    fn accuracy_02() {
        let mut n = 1.0;
        for _ in 0..1024 {
            let a = (n as f32, n as f32, n as f32);
            let b = round_trip(a);
            assert_eq!(a.1, b.1);
            n += 1.0 / 1024.0;
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
        let fs = (1.0e+28, 1.0e+28, 1.0e+28);

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
    fn smallest_value_and_underflow() {
        let fs1 = (Y_MIN, Y_MIN, Y_MIN);
        let fs2 = (Y_MIN * 0.99, Y_MIN * 0.99, Y_MIN * 0.99);

        dbg!(Y_MIN);
        assert_eq!(fs1.1, round_trip(fs1).1);
        assert_eq!(0.0, round_trip(fs2).1);
        assert_eq!(0x000056c3, encode(fs2));
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
