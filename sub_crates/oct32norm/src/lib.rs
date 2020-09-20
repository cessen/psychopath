//! Encoding/decoding for a 32-bit representation of unit 3d vectors.
//!
//! Follows the Oct32 encoding specified in the paper "A Survey
//! of Efficient Representations for Independent Unit Vectors" by
//! Cigolle et al.

/// Encodes a vector of three floats to the oct32 format.
///
/// The input vector does not need to be normalized--only the direction
/// matters to the encoding process, not the length.
#[inline]
pub fn encode(vec: (f32, f32, f32)) -> u32 {
    let l1_norm = vec.0.abs() + vec.1.abs() + vec.2.abs();
    let v0_norm = vec.0 / l1_norm;
    let v1_norm = vec.1 / l1_norm;

    let (u, v) = if vec.2 < 0.0 {
        (
            u32::from(to_snorm_16((1.0 - v1_norm.abs()) * sign(vec.0))),
            u32::from(to_snorm_16((1.0 - v0_norm.abs()) * sign(vec.1))),
        )
    } else {
        (
            u32::from(to_snorm_16(v0_norm)),
            u32::from(to_snorm_16(v1_norm)),
        )
    };

    (u << 16) | v
}

/// Decodes from an oct32 to a vector of three floats.
///
/// The returned vector will not generally be normalized.  Code that
/// needs a normalized vector should normalize the returned vector.
#[inline]
pub fn decode(n: u32) -> (f32, f32, f32) {
    let mut vec0 = from_snorm_16((n >> 16) as u16);
    let mut vec1 = from_snorm_16(n as u16);
    let vec2 = 1.0 - (vec0.abs() + vec1.abs());

    if vec2 < 0.0 {
        let old_x = vec0;
        vec0 = (1.0 - vec1.abs()) * sign(old_x);
        vec1 = (1.0 - old_x.abs()) * sign(vec1);
    }

    (vec0, vec1, vec2)
}

#[inline(always)]
fn to_snorm_16(n: f32) -> u16 {
    (n * ((1u32 << (16 - 1)) - 1) as f32).round() as i16 as u16
}

#[inline(always)]
fn from_snorm_16(n: u16) -> f32 {
    f32::from(n as i16) * (1.0f32 / ((1u32 << (16 - 1)) - 1) as f32)
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

        let nx = (-1.0, 0.0, 0.0);
        let nx_oct = encode(nx);

        let py = (0.0, 1.0, 0.0);
        let py_oct = encode(py);

        let ny = (0.0, -1.0, 0.0);
        let ny_oct = encode(ny);

        let pz = (0.0, 0.0, 1.0);
        let pz_oct = encode(pz);

        let nz = (0.0, 0.0, -1.0);
        let nz_oct = encode(nz);

        assert_eq!(px, decode(px_oct));
        assert_eq!(nx, decode(nx_oct));
        assert_eq!(py, decode(py_oct));
        assert_eq!(ny, decode(ny_oct));
        assert_eq!(pz, decode(pz_oct));
        assert_eq!(nz, decode(nz_oct));
    }
}
