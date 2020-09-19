//! Encoding/decoding for the 32-bit FloatLuv color format.
//!
//! This encoding is based on, but is slightly different than, the 32-bit
//! LogLuv format from the paper "Overcoming Gamut and Dynamic Range
//! Limitations in Digital Images" by Greg Ward.  It uses the same uv chroma
//! storage, but uses a floating point rather than log encoding to store
//! luminance, mainly for the sake of faster decoding.  It also omits the sign
//! bit of LogLuv, foregoing negative luminance capabilities.
//!
//! Compared to LogLuv, this format's chroma precision is identical and its
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
const UV_SCALE: f32 = 410.0;

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
        "trifloat::yuv32::encode(): encoding to yuv32 only \
         works correctly for positive, non-NaN numbers, but the numbers passed \
         were: ({}, {}, {})",
        xyz.0,
        xyz.1,
        xyz.2
    );

    // Calculates the 16-bit encoding of the UV values for the given XYZ input.
    fn encode_uv(xyz: (f32, f32, f32)) -> u32 {
        let s = xyz.0 + (15.0 * xyz.1) + (3.0 * xyz.2);
        let u = ((4.0 * UV_SCALE) * xyz.0 / s).max(0.0).min(255.0) as u32;
        let v = ((9.0 * UV_SCALE) * xyz.1 / s).max(0.0).min(255.0) as u32;
        (u << 8) | v
    };

    // Special case: if Y is infinite, saturate to the brightest
    // white, since with infinities we have no reasonable basis
    // for determining chromaticity.
    if xyz.1.is_infinite() {
        return 0xffff0000 | encode_uv((1.0, 1.0, 1.0));
    }

    let (l_exp, l_mant) = {
        let n = xyz.1.to_bits();
        let exp = (n >> 23) as i32 - 127 + EXP_BIAS;
        if exp <= 0 {
            return encode_uv((1.0, 1.0, 1.0));
        } else if exp > 63 {
            (63, 0b11_1111_1111)
        } else {
            (exp as u32, (n & 0x7fffff) >> 13)
        }
    };

    (l_exp << 26) | (l_mant << 16) | encode_uv(xyz)
}

/// Decodes from 32-bit FloatLuv to CIE XYZ.
#[inline]
pub fn decode(luv32: u32) -> (f32, f32, f32) {
    // Unpack values.
    let l_exp = luv32 >> 26;
    let l_mant = (luv32 >> 16) & 0b11_1111_1111;
    let u = ((luv32 >> 8) & 0xff) as f32 * (1.0 / UV_SCALE);
    let v4 = (luv32 & 0xff).max(1) as f32 * (4.0 / UV_SCALE); // 4 * v

    if l_exp == 0 {
        return (0.0, 0.0, 0.0);
    }

    let y = f32::from_bits(((l_exp + 127 - EXP_BIAS as u32) << 23) | (l_mant << 13));
    let x = y * u * (9.0 / v4); // y * (9u / 4v)
    let z = (y / v4) * (12.0 - (3.0 * u) - (5.0 * v4)); // y * ((12 - 3u - 20v) / 4v)

    (x, y, z.max(0.0))
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

        assert_eq!(0x000056c2, tri);
        assert_eq!(fs, fs2);
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
        let a = (2049.0f32, 2049.0f32, 2049.0f32);
        let b = round_trip(a);
        assert_eq!(2048.0, b.1);
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
        assert_eq!(0xffff56c2, encode(fs));
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
        assert_eq!(0x000056c2, encode(fs));
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
