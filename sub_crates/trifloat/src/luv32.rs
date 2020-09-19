//! Encoding/decoding for 32-bit HDR Luv color format.
//!
//! This encoding is based on the ideas behind the SGI LogLUV format,
//! but using a floating point rather than log encoding to store the L
//! component for the sake of faster encoding/decoding.
//!
//! The encoding uses 16 bits for the L component, and 8 bits each for the
//! u and v components.  The L component's 16 bits are split into 10 bits of
//! mantissa and 6 bits of exponent.  The mantissa uses an implicit-leading-1
//! format, giving it 11 bits of precision, and the exponent bias is 26.
//!
//! The format encodes from, and decodes to, CIE XYZ color values.
//!
//! This format is explicitly designed to support HDR color, with a supported
//! dynamic range of about 63 stops.  Specifically, the largest supported input
//! Y value is just under `2^38`, and the smallest (non-zero) Y is `2^-25`.  Y
//! values smaller than that range will underflow to zero, and larger will
//! saturate to the max value.

#![allow(clippy::cast_lossless)]

const EXP_BIAS: i32 = 26;
const UV_SCALE: f32 = 410.0;

/// Largest representable Y component.
pub const Y_MAX: f32 = ((1u64 << (64 - EXP_BIAS)) - (1u64 << (64 - EXP_BIAS - 11))) as f32;

/// Smallest representable non-zero Y component.
pub const Y_MIN: f32 = 1.0 / (1u64 << (EXP_BIAS - 1)) as f32;

/// Encodes a CIE XYZ triplet into the 32-bit Luv format.
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

    // Special case: if Y is infinite, saturate to the brightest
    // white, since with infinities we have no reasonable basis
    // for determining chromaticity.
    if xyz.1.is_infinite() {
        let s = 1.0 + (15.0 * 1.0) + (3.0 * 1.0);
        let u = ((4.0 * UV_SCALE) * 1.0 / s) as u32;
        let v = ((9.0 * UV_SCALE) * 1.0 / s) as u32;
        return 0xffff0000 | (u << 8) | v;
    }

    let s = xyz.0 + (15.0 * xyz.1) + (3.0 * xyz.2);
    let u = ((4.0 * UV_SCALE) * xyz.0 / s).max(0.0).min(255.0) as u32;
    let v = ((9.0 * UV_SCALE) * xyz.1 / s).max(0.0).min(255.0) as u32;

    let (l_exp, l_mant) = {
        let n = xyz.1.to_bits();
        let exp = (n >> 23) as i32 - 127 + EXP_BIAS;
        if exp <= 0 {
            return 0;
        } else if exp > 63 {
            (63, 0b11_1111_1111)
        } else {
            (exp as u32, (n & 0x7fffff) >> 13)
        }
    };

    (l_exp << 26) | (l_mant << 16) | (u << 8) | v
}

/// Decodes a 32-bit Luv formatted value into a CIE XYZ triplet.
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

        assert_eq!(tri, 0u32);
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
        assert_eq!(0, encode(fs));
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
