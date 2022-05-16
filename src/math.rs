#![allow(dead_code)]

use std::f32;

pub use math3d::{cross, dot, CrossProduct, DotProduct, Normal, Point, Transform, Vector};

/// Gets the log base 2 of the given integer
pub fn log2_64(n: u64) -> u64 {
    // This works by finding the largest non-zero binary digit in the
    // number.  Its bit position is then the log2 of the integer.

    if n == 0 {
        0
    } else {
        (63 - n.leading_zeros()) as u64
    }
}

/// Creates a coordinate system from a single vector.
///
/// The input vector, v, becomes the first vector of the
/// returned tuple, with the other two vectors in the returned
/// tuple defining the orthoganal axes.
///
/// Algorithm taken from "Building an Orthonormal Basis, Revisited" by Duff et al.
pub fn coordinate_system_from_vector(v: Vector) -> (Vector, Vector, Vector) {
    let sign = v.z().signum();
    let a = -1.0 / (sign + v.z());
    let b = v.x() * v.y() * a;
    let v2 = Vector::new(1.0 + sign * v.x() * v.x() * a, sign * b, -sign * v.x());
    let v3 = Vector::new(b, sign + v.y() * v.y() * a, -v.y());

    (v, v2, v3)
}

/// Simple mapping of a vector that exists in a z-up space to
/// the space of another vector who's direction is considered
/// z-up for the purpose.
/// Obviously this doesn't care about the direction _around_
/// the z-up, although it will be sufficiently consistent for
/// isotropic sampling purposes.
///
/// from: The vector we're transforming.
/// toz: The vector whose space we are transforming "from" into.
///
/// Returns the transformed vector.
pub fn zup_to_vec(from: Vector, toz: Vector) -> Vector {
    let (toz, tox, toy) = coordinate_system_from_vector(toz.normalized());

    // Use simple linear algebra to convert the "from"
    // vector to a space composed of tox, toy, and toz
    // as the x, y, and z axes.
    (tox * from.x()) + (toy * from.y()) + (toz * from.z())
}

/// A highly accurate approximation of the probit function.
///
/// From Peter John Acklam, sourced from here:
/// https://web.archive.org/web/20151030215612/http://home.online.no/~pjacklam/notes/invnorm/
///
/// Regarding the approximation error, he says, "The absolute value of the
/// relative error is less than 1.15 × 10−9 in the entire region."
///
/// Given that this implementation outputs 32-bit floating point values,
/// and 32-bit floating point has significantly less precision than that,
/// this approximation can essentially be considered exact.
pub fn probit(n: f32, width: f32) -> f32 {
    let n = n as f64;

    // Coefficients of the rational approximations.
    const A1: f64 = -3.969683028665376e+01;
    const A2: f64 = 2.209460984245205e+02;
    const A3: f64 = -2.759285104469687e+02;
    const A4: f64 = 1.383577518672690e+02;
    const A5: f64 = -3.066479806614716e+01;
    const A6: f64 = 2.506628277459239e+00;

    const B1: f64 = -5.447609879822406e+01;
    const B2: f64 = 1.615858368580409e+02;
    const B3: f64 = -1.556989798598866e+02;
    const B4: f64 = 6.680131188771972e+01;
    const B5: f64 = -1.328068155288572e+01;

    const C1: f64 = -7.784894002430293e-03;
    const C2: f64 = -3.223964580411365e-01;
    const C3: f64 = -2.400758277161838e+00;
    const C4: f64 = -2.549732539343734e+00;
    const C5: f64 = 4.374664141464968e+00;
    const C6: f64 = 2.938163982698783e+00;

    const D1: f64 = 7.784695709041462e-03;
    const D2: f64 = 3.224671290700398e-01;
    const D3: f64 = 2.445134137142996e+00;
    const D4: f64 = 3.754408661907416e+00;

    // Transition points between the rational functions.
    const N_LOW: f64 = 0.02425;
    const N_HIGH: f64 = 1.0 - N_LOW;

    let x = match n {
        // Lower region.
        n if 0.0 < n && n < N_LOW => {
            let q = (-2.0 * n.ln()).sqrt();
            (((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
                / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
        }

        // Central region.
        n if n <= N_HIGH => {
            let q = n - 0.5;
            let r = q * q;
            (((((A1 * r + A2) * r + A3) * r + A4) * r + A5) * r + A6) * q
                / (((((B1 * r + B2) * r + B3) * r + B4) * r + B5) * r + 1.0)
        }

        // Upper region.
        n if n < 1.0 => {
            let q = (-2.0 * (1.0 - n).ln()).sqrt();
            -(((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
                / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
        }

        // Exactly 1 or 0.  Should be extremely rare.
        n if n == 0.0 => -std::f64::INFINITY,
        n if n == 1.0 => std::f64::INFINITY,

        // Outside of valid input range.
        _ => std::f64::NAN,
    };

    x as f32 * width
}

pub fn fast_ln(x: f32) -> f32 {
    fastapprox::fast::ln(x)
}

pub fn fast_pow2(p: f32) -> f32 {
    fastapprox::fast::pow2(p)
}

pub fn fast_log2(x: f32) -> f32 {
    fastapprox::fast::log2(x)
}

pub fn fast_exp(p: f32) -> f32 {
    fastapprox::fast::exp(p)
}

pub fn fast_pow(x: f32, p: f32) -> f32 {
    fastapprox::fast::pow(x, p)
}

pub fn faster_pow2(p: f32) -> f32 {
    fastapprox::faster::pow2(p)
}

pub fn faster_exp(p: f32) -> f32 {
    fastapprox::faster::exp(p)
}

//----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log2_64_test() {
        assert_eq!(0, log2_64(0));

        for i in 0..64 {
            assert_eq!(i, log2_64(1 << i));
        }

        for i in 8..64 {
            assert_eq!(i, log2_64((1 << i) + 227));
        }

        for i in 16..64 {
            assert_eq!(i, log2_64((1 << i) + 56369));
        }

        for i in 32..64 {
            assert_eq!(i, log2_64((1 << i) + 2514124923));
        }
    }
}
