//! Encoding/decoding for a 32-bit representation of unit 3d vectors.
//!
//! Follows the Oct32 encoding specified in the paper "A Survey
//! of Efficient Representations for Independent Unit Vectors" by
//! Cigolle et al.

const STEP_SIZE: f32 = 1.0 / STEPS;
const STEPS: f32 = ((1 << (16 - 1)) - 1) as f32;

/// Encodes a vector of three floats to the oct32 format.
///
/// The input vector does not need to be normalized--only the direction
/// matters to the encoding process, not the length.
#[inline]
pub fn encode(vec: (f32, f32, f32)) -> u32 {
    let (u, v) = vec3_to_oct(vec);
    ((to_snorm_16(u) as u32) << 16) | to_snorm_16(v) as u32
}

/// Encodes a vector of three floats to the oct32 format.
///
/// This is the same as `encode()` except that it is slower and encodes
/// with slightly better precision.
pub fn encode_precise(vec: (f32, f32, f32)) -> u32 {
    #[inline(always)]
    fn dot_norm(a: (f32, f32, f32), b: (f32, f32, f32)) -> f64 {
        let l = ((a.0 as f64 * a.0 as f64) + (a.1 as f64 * a.1 as f64) + (a.2 as f64 * a.2 as f64))
            .sqrt();
        ((a.0 as f64 * b.0 as f64) + (a.1 as f64 * b.1 as f64) + (a.2 as f64 * b.2 as f64)) / l
    }

    // Calculate the initial floored version.
    let s = {
        let mut s = vec3_to_oct(vec); // Remap to the square.
        s.0 = (s.0.max(-1.0).min(1.0) * STEPS).floor() * STEP_SIZE;
        s.1 = (s.1.max(-1.0).min(1.0) * STEPS).floor() * STEP_SIZE;
        s
    };

    // Test all combinations of floor and ceil and keep the best.
    // Note that at +/- 1, this will exit the square, but that
    // will be a worse encoding and never win.
    let mut best_rep = s;
    let mut max_dot = 0.0;
    for &(i, j) in &[
        (0.0, 0.0),
        (0.0, STEP_SIZE),
        (STEP_SIZE, 0.0),
        (STEP_SIZE, STEP_SIZE),
    ] {
        let candidate = (s.0 + i, s.1 + j);
        let oct = oct_to_vec3(candidate);
        let dot = dot_norm(oct, vec);
        if dot > max_dot {
            best_rep = candidate;
            max_dot = dot;
        }
    }

    ((to_snorm_16(best_rep.0) as u32) << 16) | to_snorm_16(best_rep.1) as u32
}

/// Decodes from an oct32 to a vector of three floats.
///
/// The returned vector will not generally be normalized.  Code that
/// needs a normalized vector should normalize the returned vector.
#[inline]
pub fn decode(n: u32) -> (f32, f32, f32) {
    oct_to_vec3((from_snorm_16((n >> 16) as u16), from_snorm_16(n as u16)))
}

#[inline(always)]
fn vec3_to_oct(vec: (f32, f32, f32)) -> (f32, f32) {
    let l1_norm = vec.0.abs() + vec.1.abs() + vec.2.abs();
    let u = vec.0 / l1_norm;
    let v = vec.1 / l1_norm;

    if vec.2 > 0.0 {
        (u, v)
    } else {
        ((1.0 - v.abs()) * sign(vec.0), (1.0 - u.abs()) * sign(vec.1))
    }
}

#[inline(always)]
fn oct_to_vec3(oct: (f32, f32)) -> (f32, f32, f32) {
    let vec2 = 1.0 - (oct.0.abs() + oct.1.abs());

    if vec2 < 0.0 {
        (
            (1.0 - oct.1.abs()) * sign(oct.0),
            (1.0 - oct.0.abs()) * sign(oct.1),
            vec2,
        )
    } else {
        (oct.0, oct.1, vec2)
    }
}

#[inline(always)]
fn to_snorm_16(n: f32) -> u16 {
    (n * STEPS).round() as i16 as u16
}

#[inline(always)]
fn from_snorm_16(n: u16) -> f32 {
    f32::from(n as i16) * STEP_SIZE
}

#[inline(always)]
fn sign(n: f32) -> f32 {
    if n < 0.0 {
        -1.0
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_directions() {
        let px = (1.0, 0.0, 0.0);
        let px_oct = encode(px);
        let px_octp = encode_precise(px);

        let nx = (-1.0, 0.0, 0.0);
        let nx_oct = encode(nx);
        let nx_octp = encode_precise(nx);

        let py = (0.0, 1.0, 0.0);
        let py_oct = encode(py);
        let py_octp = encode_precise(py);

        let ny = (0.0, -1.0, 0.0);
        let ny_oct = encode(ny);
        let ny_octp = encode_precise(ny);

        let pz = (0.0, 0.0, 1.0);
        let pz_oct = encode(pz);
        let pz_octp = encode_precise(pz);

        let nz = (0.0, 0.0, -1.0);
        let nz_oct = encode(nz);
        let nz_octp = encode_precise(nz);

        assert_eq!(px, decode(px_oct));
        assert_eq!(nx, decode(nx_oct));
        assert_eq!(py, decode(py_oct));
        assert_eq!(ny, decode(ny_oct));
        assert_eq!(pz, decode(pz_oct));
        assert_eq!(nz, decode(nz_oct));

        assert_eq!(px, decode(px_octp));
        assert_eq!(nx, decode(nx_octp));
        assert_eq!(py, decode(py_octp));
        assert_eq!(ny, decode(ny_octp));
        assert_eq!(pz, decode(pz_octp));
        assert_eq!(nz, decode(nz_octp));
    }
}
