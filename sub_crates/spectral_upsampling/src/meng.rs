// Since this is basicallt translated from C, silence a bunch of
// clippy warnings that stem from the C code.
#![allow(clippy::needless_return)]
#![allow(clippy::useless_let_if_seq)]
#![allow(clippy::cyclomatic_complexity)]

use std::f32;

use glam::Vec4;

mod meng_spectra_tables;

pub use self::meng_spectra_tables::{
    EQUAL_ENERGY_REFLECTANCE, SPECTRUM_SAMPLE_MAX, SPECTRUM_SAMPLE_MIN,
};

use self::meng_spectra_tables::{
    SPECTRUM_DATA_POINTS,
    // CMF_X,
    // CMF_Y,
    // CMF_Z,
    // SPECTRUM_MAT_UV_TO_XY,
    SPECTRUM_GRID,
    SPECTRUM_GRID_HEIGHT,
    SPECTRUM_GRID_WIDTH,
    // SPECTRUM_MAT_XY_TO_XYSTAR,
    // SPECTRUM_MAT_XYSTAR_TO_XY,
    SPECTRUM_MAT_XY_TO_UV,
    // SPECTRUM_BIN_SIZE,
    SPECTRUM_NUM_SAMPLES,
};

/// Evaluate the spectrum for xyz at the given wavelength.
#[inline]
pub fn spectrum_xyz_to_p(lambda: f32, xyz: (f32, f32, f32)) -> f32 {
    assert!(lambda >= SPECTRUM_SAMPLE_MIN);
    assert!(lambda <= SPECTRUM_SAMPLE_MAX);

    let inv_norm = xyz.0 + xyz.1 + xyz.2;
    let norm = {
        let norm = 1.0 / inv_norm;
        if norm < f32::MAX {
            norm
        } else {
            return 0.0;
        }
    };

    let xyy = (xyz.0 * norm, xyz.1 * norm, xyz.1);

    // Rotate to align with grid
    let uv = spectrum_xy_to_uv((xyy.0, xyy.1));
    if uv.0 < 0.0
        || uv.0 >= SPECTRUM_GRID_WIDTH as f32
        || uv.1 < 0.0
        || uv.1 >= SPECTRUM_GRID_HEIGHT as f32
    {
        return 0.0;
    }

    let uvi = (uv.0 as i32, uv.1 as i32);
    debug_assert!(uvi.0 < SPECTRUM_GRID_WIDTH);
    debug_assert!(uvi.1 < SPECTRUM_GRID_HEIGHT);

    let cell_idx: i32 = uvi.0 + SPECTRUM_GRID_WIDTH * uvi.1;
    debug_assert!(cell_idx >= 0);

    let cell = &SPECTRUM_GRID[cell_idx as usize];
    let inside: bool = cell.inside;
    let idx = &cell.idx;
    let num: i32 = cell.num_points;

    // If the cell has no points, nothing we can do, so return 0.0
    if num == 0 {
        return 0.0;
    }

    // Normalize lambda to spectrum table index range.
    let sb: f32 = (lambda - SPECTRUM_SAMPLE_MIN) / (SPECTRUM_SAMPLE_MAX - SPECTRUM_SAMPLE_MIN)
        * (SPECTRUM_NUM_SAMPLES as f32 - 1.0);
    debug_assert!(sb >= 0.0);
    debug_assert!(sb <= SPECTRUM_NUM_SAMPLES as f32);

    // Get the spectral values for the vertices of the grid cell.
    let mut p = [0.0f32; 6];
    let sb0: i32 = sb as i32;
    let sb1: i32 = if (sb + 1.0) < SPECTRUM_NUM_SAMPLES as f32 {
        sb as i32 + 1
    } else {
        SPECTRUM_NUM_SAMPLES - 1
    };
    assert!(sb0 < SPECTRUM_NUM_SAMPLES);
    let sbf: f32 = sb as f32 - sb0 as f32;
    for i in 0..(num as usize) {
        debug_assert!(idx[i] >= 0);
        let spectrum = &SPECTRUM_DATA_POINTS[idx[i] as usize].spectrum;
        p[i] = spectrum[sb0 as usize] * (1.0 - sbf) + spectrum[sb1 as usize] * sbf;
    }

    // Linearly interpolated the spectral power of the cell vertices.
    let mut interpolated_p: f32 = 0.0;
    if inside {
        // Fast path for normal inner quads:
        let uv2 = (uv.0 - uvi.0 as f32, uv.1 - uvi.1 as f32);

        assert!(uv2.0 >= 0.0 && uv2.0 <= 1.0);
        assert!(uv2.1 >= 0.0 && uv2.1 <= 1.0);

        // The layout of the vertices in the quad is:
        //  2  3
        //  0  1
        interpolated_p = p[0] * (1.0 - uv2.0) * (1.0 - uv2.1)
            + p[2] * (1.0 - uv2.0) * uv2.1
            + p[3] * uv2.0 * uv2.1
            + p[1] * uv2.0 * (1.0 - uv2.1);
    } else {
        // Need to go through triangulation :(
        // We get the indices in such an order that they form a triangle fan around idx[0].
        // compute barycentric coordinates of our xy* point for all triangles in the fan:
        let ex: f32 = uv.0 - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.0;
        let ey: f32 = uv.1 - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.1;
        let mut e0x: f32 =
            SPECTRUM_DATA_POINTS[idx[1] as usize].uv.0 - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.0;
        let mut e0y: f32 =
            SPECTRUM_DATA_POINTS[idx[1] as usize].uv.1 - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.1;
        let mut uu: f32 = e0x * ey - ex * e0y;

        for i in 0..(num as usize - 1) {
            let (e1x, e1y): (f32, f32) = if i as i32 == (num - 2) {
                // Close the circle
                (
                    SPECTRUM_DATA_POINTS[idx[1] as usize].uv.0
                        - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.0,
                    SPECTRUM_DATA_POINTS[idx[1] as usize].uv.1
                        - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.1,
                )
            } else {
                (
                    SPECTRUM_DATA_POINTS[idx[i + 2] as usize].uv.0
                        - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.0,
                    SPECTRUM_DATA_POINTS[idx[i + 2] as usize].uv.1
                        - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.1,
                )
            };

            let vv: f32 = ex * e1y - e1x * ey;
            let area: f32 = e0x * e1y - e1x * e0y;

            // Normalise
            let u: f32 = uu / area;
            let v: f32 = vv / area;
            let w: f32 = 1.0 - u - v;
            // Outside spectral locus (quantized version at least) or outside grid
            if u < 0.0 || v < 0.0 || w < 0.0 {
                uu = -vv;
                e0x = e1x;
                e0y = e1y;
                continue;
            }

            // This seems to be the triangle we've been looking for.
            interpolated_p =
                p[0] * w + p[i + 1] * v + p[if i as i32 == (num - 2) { 1 } else { i + 2 }] * u;
            break;
        }
    }

    // Now we have a spectrum which corresponds to the xy chromaticities of
    // the input. need to scale according to the input brightness X+Y+Z now:
    return interpolated_p * inv_norm;
}

/// Evaluate the spectrum for xyz at the given wavelengths.
///
/// Works on 4 wavelengths at once via SIMD.
#[inline]
pub fn spectrum_xyz_to_p_4(lambdas: Vec4, xyz: (f32, f32, f32)) -> Vec4 {
    assert!(lambdas.min_element() >= SPECTRUM_SAMPLE_MIN);
    assert!(lambdas.max_element() <= SPECTRUM_SAMPLE_MAX);

    let inv_norm = xyz.0 + xyz.1 + xyz.2;
    let norm = {
        let norm = 1.0 / inv_norm;
        if norm < f32::MAX {
            norm
        } else {
            return Vec4::splat(0.0);
        }
    };

    let xyy = (xyz.0 * norm, xyz.1 * norm, xyz.1);

    // Rotate to align with grid
    let uv = spectrum_xy_to_uv((xyy.0, xyy.1));
    if uv.0 < 0.0
        || uv.0 >= SPECTRUM_GRID_WIDTH as f32
        || uv.1 < 0.0
        || uv.1 >= SPECTRUM_GRID_HEIGHT as f32
    {
        return Vec4::splat(0.0);
    }

    let uvi = (uv.0 as i32, uv.1 as i32);
    debug_assert!(uvi.0 < SPECTRUM_GRID_WIDTH);
    debug_assert!(uvi.1 < SPECTRUM_GRID_HEIGHT);

    let cell_idx: i32 = uvi.0 + SPECTRUM_GRID_WIDTH * uvi.1;
    debug_assert!(cell_idx >= 0);

    let cell = &SPECTRUM_GRID[cell_idx as usize];
    let inside: bool = cell.inside;
    let idx = &cell.idx;
    let num: i32 = cell.num_points;

    // If the cell has no points, nothing we can do, so return 0.0
    if num == 0 {
        return Vec4::splat(0.0);
    }

    // Normalize lambda to spectrum table index range.
    let sb: Vec4 = (lambdas - Vec4::splat(SPECTRUM_SAMPLE_MIN))
        / (SPECTRUM_SAMPLE_MAX - SPECTRUM_SAMPLE_MIN)
        * (SPECTRUM_NUM_SAMPLES as f32 - 1.0);
    debug_assert!(sb.min_element() >= 0.0);
    debug_assert!(sb.max_element() <= SPECTRUM_NUM_SAMPLES as f32);

    // Get the spectral values for the vertices of the grid cell.
    // TODO: use integer SIMD intrinsics to make this part faster.
    let mut p = [Vec4::splat(0.0); 6];
    let sb0: [i32; 4] = [sb.x() as i32, sb.y() as i32, sb.z() as i32, sb.w() as i32];
    assert!(sb0[0].max(sb0[1]).max(sb0[2].max(sb0[3])) < SPECTRUM_NUM_SAMPLES);
    let sb1: [i32; 4] = [
        (sb.x() as i32 + 1).min(SPECTRUM_NUM_SAMPLES - 1),
        (sb.y() as i32 + 1).min(SPECTRUM_NUM_SAMPLES - 1),
        (sb.z() as i32 + 1).min(SPECTRUM_NUM_SAMPLES - 1),
        (sb.w() as i32 + 1).min(SPECTRUM_NUM_SAMPLES - 1),
    ];
    let sbf = sb - Vec4::new(sb0[0] as f32, sb0[1] as f32, sb0[2] as f32, sb0[3] as f32);
    for i in 0..(num as usize) {
        debug_assert!(idx[i] >= 0);
        let spectrum = &SPECTRUM_DATA_POINTS[idx[i] as usize].spectrum;
        let p0 = Vec4::new(
            spectrum[sb0[0] as usize],
            spectrum[sb0[1] as usize],
            spectrum[sb0[2] as usize],
            spectrum[sb0[3] as usize],
        );
        let p1 = Vec4::new(
            spectrum[sb1[0] as usize],
            spectrum[sb1[1] as usize],
            spectrum[sb1[2] as usize],
            spectrum[sb1[3] as usize],
        );
        p[i] = p0 * (Vec4::splat(1.0) - sbf) + p1 * sbf;
    }

    // Linearly interpolate the spectral power of the cell vertices.
    let mut interpolated_p = Vec4::splat(0.0);
    if inside {
        // Fast path for normal inner quads:
        let uv2 = (uv.0 - uvi.0 as f32, uv.1 - uvi.1 as f32);

        assert!(uv2.0 >= 0.0 && uv2.0 <= 1.0);
        assert!(uv2.1 >= 0.0 && uv2.1 <= 1.0);

        // The layout of the vertices in the quad is:
        //  2  3
        //  0  1
        interpolated_p = p[0] * ((1.0 - uv2.0) * (1.0 - uv2.1))
            + p[2] * ((1.0 - uv2.0) * uv2.1)
            + p[3] * (uv2.0 * uv2.1)
            + p[1] * (uv2.0 * (1.0 - uv2.1));
    } else {
        // Need to go through triangulation :(
        // We get the indices in such an order that they form a triangle fan around idx[0].
        // compute barycentric coordinates of our xy* point for all triangles in the fan:
        let ex: f32 = uv.0 - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.0;
        let ey: f32 = uv.1 - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.1;
        let mut e0x: f32 =
            SPECTRUM_DATA_POINTS[idx[1] as usize].uv.0 - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.0;
        let mut e0y: f32 =
            SPECTRUM_DATA_POINTS[idx[1] as usize].uv.1 - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.1;
        let mut uu: f32 = e0x * ey - ex * e0y;

        for i in 0..(num as usize - 1) {
            let (e1x, e1y): (f32, f32) = if i as i32 == (num - 2) {
                // Close the circle
                (
                    SPECTRUM_DATA_POINTS[idx[1] as usize].uv.0
                        - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.0,
                    SPECTRUM_DATA_POINTS[idx[1] as usize].uv.1
                        - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.1,
                )
            } else {
                (
                    SPECTRUM_DATA_POINTS[idx[i + 2] as usize].uv.0
                        - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.0,
                    SPECTRUM_DATA_POINTS[idx[i + 2] as usize].uv.1
                        - SPECTRUM_DATA_POINTS[idx[0] as usize].uv.1,
                )
            };

            let vv: f32 = ex * e1y - e1x * ey;
            let area: f32 = e0x * e1y - e1x * e0y;

            // Normalise
            let u: f32 = uu / area;
            let v: f32 = vv / area;
            let w: f32 = 1.0 - u - v;
            // Outside spectral locus (quantized version at least) or outside grid
            if u < 0.0 || v < 0.0 || w < 0.0 {
                uu = -vv;
                e0x = e1x;
                e0y = e1y;
                continue;
            }

            // This seems to be the triangle we've been looking for.
            interpolated_p =
                p[0] * w + p[i + 1] * v + p[if i as i32 == (num - 2) { 1 } else { i + 2 }] * u;
            break;
        }
    }

    // Now we have a spectrum which corresponds to the xy chromaticities of
    // the input. need to scale according to the input brightness X+Y+Z now:
    return interpolated_p * inv_norm;
}

// apply a 3x2 matrix to a 2D color.
#[inline(always)]
fn spectrum_apply_3x2(matrix: &[f32; 6], src: (f32, f32)) -> (f32, f32) {
    (
        matrix[0] * src.0 + matrix[1] * src.1 + matrix[2],
        matrix[3] * src.0 + matrix[4] * src.1 + matrix[5],
    )
}
// Concrete conversion routines.
// #[inline(always)]
// fn spectrum_xy_to_xystar(xy: (f32, f32)) -> (f32, f32) {
//     spectrum_apply_3x2(&SPECTRUM_MAT_XY_TO_XYSTAR, xy)
// }
// #[inline(always)]
// fn spectrum_xystar_to_xy(xystar: (f32, f32)) -> (f32, f32) {
//     spectrum_apply_3x2(&SPECTRUM_MAT_XYSTAR_TO_XY, xystar)
// }
#[inline(always)]
fn spectrum_xy_to_uv(xy: (f32, f32)) -> (f32, f32) {
    spectrum_apply_3x2(&SPECTRUM_MAT_XY_TO_UV, xy)
}
// #[inline(always)]
// fn spectrum_uv_to_xy(uv: (f32, f32)) -> (f32, f32) {
//     spectrum_apply_3x2(&SPECTRUM_MAT_UV_TO_XY, uv)
// }

// #[inline]
// pub fn xyz_from_spectrum(spectrum: &[f32]) -> (f32, f32, f32) {
//     let mut xyz = (0.0, 0.0, 0.0);
//     for i in 0..(SPECTRUM_NUM_SAMPLES as usize) {
//         xyz.0 += spectrum[i] * CMF_X[i];
//         xyz.1 += spectrum[i] * CMF_Y[i];
//         xyz.2 += spectrum[i] * CMF_Z[i];
//     }
//     xyz.0 *= SPECTRUM_BIN_SIZE;
//     xyz.1 *= SPECTRUM_BIN_SIZE;
//     xyz.2 *= SPECTRUM_BIN_SIZE;
//     return xyz;
// }
