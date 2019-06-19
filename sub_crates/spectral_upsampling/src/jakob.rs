use std::{fs::File, io, io::Read};

use float4::Float4;

use lazy_static::lazy_static;

/// How many polynomial coefficients?
const RGB2SPEC_N_COEFFS: usize = 3;

/// Table resolution.
const TABLE_RES: usize = 64;

// For the small table, what is the middle value used?
const MID_VALUE: f32 = 0.5;

lazy_static! {
    static ref ACES_TABLE: RGB2Spec = rgb2spec_load("");
    static ref ACES_TABLE_SMALL: Vec<[Float4; 2]> = rgb2spec_load_small("");
}

pub fn spectrum_acesrgb_to_p(lambda: f32, rgb: (f32, f32, f32)) -> f32 {
    let max = {
        let mut max = rgb.0;
        if max < rgb.1 {
            max = rgb.1
        };
        if max < rgb.2 {
            max = rgb.2
        };
        max
    };

    if max == 0.0 {
        0.0
    } else if max <= 1.0 {
        let co = rgb2spec_fetch(&ACES_TABLE, [rgb.0, rgb.1, rgb.2]);
        rgb2spec_eval(co, lambda)
    } else {
        let rgb = (rgb.0 / max, rgb.1 / max, rgb.2 / max);
        let co = rgb2spec_fetch(&ACES_TABLE, [rgb.0, rgb.1, rgb.2]);
        rgb2spec_eval(co, lambda) * max
    }
}

#[inline]
pub fn spectrum_acesrgb_to_p4(lambdas: Float4, rgb: (f32, f32, f32)) -> Float4 {
    let max = {
        let mut max = rgb.0;
        if max < rgb.1 {
            max = rgb.1
        };
        if max < rgb.2 {
            max = rgb.2
        };
        max
    };

    if max == 0.0 {
        Float4::splat(0.0)
    } else if max <= 1.0 {
        let co = rgb2spec_fetch(&ACES_TABLE, [rgb.0, rgb.1, rgb.2]);
        rgb2spec_eval_4(co, lambdas)
    } else {
        let rgb_norm = (rgb.0 / max, rgb.1 / max, rgb.2 / max);
        let co = rgb2spec_fetch(&ACES_TABLE, [rgb_norm.0, rgb_norm.1, rgb_norm.2]);
        rgb2spec_eval_4(co, lambdas) * Float4::splat(max)
    }
}

#[inline]
pub fn small_spectrum_acesrgb_to_p4(lambdas: Float4, rgb: (f32, f32, f32)) -> Float4 {
    // Determine largest RGB component, and calculate the other two
    // components scaled for lookups.
    let (i, max_val, x, y) = {
        let mut i = 0;
        let mut max_val = rgb.0;
        let mut x = rgb.1;
        let mut y = rgb.2;

        if rgb.1 > max_val {
            i = 1;
            max_val = rgb.1;
            x = rgb.2;
            y = rgb.0;
        }

        if rgb.2 > max_val {
            i = 2;
            max_val = rgb.2;
            x = rgb.0;
            y = rgb.1;
        }

        let scale = 63.0 / max_val;
        x *= scale;
        y *= scale;

        (i, max_val, x, y)
    };

    // Make sure we're not looking up black, to avoid NaN's from divide by zero.
    if max_val == 0.0 {
        return Float4::splat(0.0);
    }

    // Calculate lookup coordinates.
    let xi = (x as usize).min(TABLE_RES - 2);
    let yi = (y as usize).min(TABLE_RES - 2);
    let offset = (TABLE_RES * TABLE_RES * i) + (yi * TABLE_RES) + xi;
    let dx = 1;
    let dy = TABLE_RES;

    // Look up values from table.
    let a0 = ACES_TABLE_SMALL[offset];
    let a1 = ACES_TABLE_SMALL[offset + dx];
    let a2 = ACES_TABLE_SMALL[offset + dy];
    let a3 = ACES_TABLE_SMALL[offset + dy + dx];

    // Do interpolation.
    let x1: f32 = x - xi as f32;
    let x0: f32 = 1.0 - x1 as f32;
    let y1: f32 = y - yi as f32;
    let y0: f32 = 1.0 - y1 as f32;
    let b0 = [(a0[0] * x0) + (a1[0] * x1), (a0[1] * x0) + (a1[1] * x1)];
    let b1 = [(a2[0] * x0) + (a3[0] * x1), (a2[1] * x0) + (a3[1] * x1)];
    let c = [(b0[0] * y0) + (b1[0] * y1), (b0[1] * y0) + (b1[1] * y1)];

    // Evaluate the spectral function and return the result.
    if max_val <= MID_VALUE {
        rgb2spec_eval_4([c[0].get_0(), c[0].get_1(), c[0].get_2()], lambdas)
            * (1.0 / MID_VALUE)
            * max_val
    } else if max_val < 1.0 {
        let n = (max_val - MID_VALUE) / (1.0 - MID_VALUE);
        let s0 = rgb2spec_eval_4([c[0].get_0(), c[0].get_1(), c[0].get_2()], lambdas);
        let s1 = rgb2spec_eval_4([c[1].get_0(), c[1].get_1(), c[1].get_2()], lambdas);
        (s0 * (1.0 - n)) + (s1 * n)
    } else {
        rgb2spec_eval_4([c[1].get_0(), c[1].get_1(), c[1].get_2()], lambdas) * max_val
    }
}

pub fn rgb2spec_load(filepath: &str) -> RGB2Spec {
    let file_contents = {
        let mut file_contents = Vec::new();
        let mut f = io::BufReader::new(File::open(filepath).unwrap());
        f.read_to_end(&mut file_contents).unwrap();
        file_contents
    };

    // Check the header
    let header = &file_contents[0..4];
    if header != "SPEC".as_bytes() {
        panic!("Not a spectral table.");
    }

    // Get resolution of the table
    let res = u32::from_le_bytes([
        file_contents[4],
        file_contents[5],
        file_contents[6],
        file_contents[7],
    ]) as usize;

    // Calculate sizes
    let size_scale = res;
    let size_data = res * res * res * RGB2SPEC_N_COEFFS;

    // Load the table scale data
    let mut scale = Vec::with_capacity(size_scale);
    for i in 0..size_scale {
        let ii = i * 4 + 8;
        let n = f32::from_bits(u32::from_le_bytes([
            file_contents[ii],
            file_contents[ii + 1],
            file_contents[ii + 2],
            file_contents[ii + 3],
        ]));
        scale.push(n);
    }

    // Load the table coefficient data
    let mut data = Vec::with_capacity(size_data);
    for i in 0..size_data {
        let ii = i * 4 * RGB2SPEC_N_COEFFS + 8 + (size_scale * 4);
        let n1 = f32::from_bits(u32::from_le_bytes([
            file_contents[ii],
            file_contents[ii + 1],
            file_contents[ii + 2],
            file_contents[ii + 3],
        ]));
        let n2 = f32::from_bits(u32::from_le_bytes([
            file_contents[ii + 4],
            file_contents[ii + 5],
            file_contents[ii + 6],
            file_contents[ii + 7],
        ]));
        let n3 = f32::from_bits(u32::from_le_bytes([
            file_contents[ii + 8],
            file_contents[ii + 9],
            file_contents[ii + 10],
            file_contents[ii + 11],
        ]));
        data.push([n1, n2, n3]);
    }

    RGB2Spec {
        res: res,
        scale: scale,
        data: data,
    }
}

pub fn rgb2spec_load_small(filepath: &str) -> Vec<[Float4; 2]> {
    let big_table = rgb2spec_load(filepath);
    assert!(big_table.res == TABLE_RES);

    // Calculate z offsets and such for the mid value.
    let dz: usize = 1 * big_table.res * big_table.res;
    let z05_i = rgb2spec_find_interval(&big_table.scale, MID_VALUE);
    let z05_1: f32 = (MID_VALUE - big_table.scale[z05_i])
        / (big_table.scale[z05_i + 1] - big_table.scale[z05_i]);
    let z05_0: f32 = 1.0 - z05_1;

    // Fill in table.
    let mut table = vec![[Float4::splat(0.0); 2]; TABLE_RES * TABLE_RES * 3];
    for i in 0..3 {
        let offset = i * big_table.res * big_table.res * big_table.res;
        for j in 0..(big_table.res * big_table.res) {
            let one_coef = big_table.data[offset + ((TABLE_RES - 1) * dz) + j];

            let mid_coef_0 = big_table.data[offset + (z05_i * dz) + j];
            let mid_coef_1 = big_table.data[offset + ((z05_i + 1) * dz) + j];
            let mid_coef = [
                (mid_coef_0[0] * z05_0) + (mid_coef_1[0] * z05_1),
                (mid_coef_0[1] * z05_0) + (mid_coef_1[1] * z05_1),
                (mid_coef_0[2] * z05_0) + (mid_coef_1[2] * z05_1),
            ];

            table[(i * big_table.res * big_table.res) + j] = [
                Float4::new(mid_coef[0], mid_coef[1], mid_coef[2], 0.0),
                Float4::new(one_coef[0], one_coef[1], one_coef[2], 0.0),
            ];
        }
    }

    table
}

/// Underlying representation
pub struct RGB2Spec {
    res: usize,
    scale: Vec<f32>,
    data: Vec<[f32; RGB2SPEC_N_COEFFS]>,
}

//============================================================
// Coefficient -> eval functions

#[inline(always)]
fn rgb2spec_fma(a: f32, b: f32, c: f32) -> f32 {
    a * b + c
}

#[inline(always)]
fn rgb2spec_fma_4(a: Float4, b: Float4, c: Float4) -> Float4 {
    a.fmadd(b, c)
}

fn rgb2spec_eval(coeff: [f32; RGB2SPEC_N_COEFFS], lambda: f32) -> f32 {
    let x = rgb2spec_fma(rgb2spec_fma(coeff[0], lambda, coeff[1]), lambda, coeff[2]);

    let y = 1.0 / (rgb2spec_fma(x, x, 1.0)).sqrt();

    rgb2spec_fma(0.5 * x, y, 0.5)
}

fn rgb2spec_eval_4(coeff: [f32; RGB2SPEC_N_COEFFS], lambda: Float4) -> Float4 {
    let co0 = Float4::splat(coeff[0]);
    let co1 = Float4::splat(coeff[1]);
    let co2 = Float4::splat(coeff[2]);

    let x = rgb2spec_fma_4(rgb2spec_fma_4(co0, lambda, co1), lambda, co2);

    let y = Float4::splat(1.0) / (rgb2spec_fma_4(x, x, Float4::splat(1.0))).sqrt();

    rgb2spec_fma_4(Float4::splat(0.5) * x, y, Float4::splat(0.5))
}

//=================================================================
// Other misc helper functions

fn rgb2spec_find_interval(values: &[f32], x: f32) -> usize {
    let last_interval = values.len() - 2;
    let mut left = 0;
    let mut size = last_interval;

    while size > 0 {
        let half = size >> 1;
        let middle = left + half + 1;

        if values[middle] < x {
            left = middle;
            size -= half + 1;
        } else {
            size = half;
        }
    }

    if left < last_interval {
        left
    } else {
        last_interval
    }
}

/// Convert an RGB value into a RGB2Spec coefficient representation
fn rgb2spec_fetch(model: &RGB2Spec, rgb: [f32; 3]) -> [f32; RGB2SPEC_N_COEFFS] {
    assert!(
        rgb[0] >= 0.0
            && rgb[1] >= 0.0
            && rgb[2] >= 0.0
            && rgb[0] <= 1.0
            && rgb[1] <= 1.0
            && rgb[2] <= 1.0
    );

    let res = model.res;

    // Determine largest RGB component.
    let i = {
        let mut i = 0;
        if rgb[i] < rgb[1] {
            i = 1;
        }
        if rgb[i] < rgb[2] {
            i = 2;
        }
        i
    };

    let z = rgb[i];
    let scale = (res - 1) as f32 / z;
    let x = rgb[(i + 1) % 3] * scale;
    let y = rgb[(i + 2) % 3] * scale;

    // Bilinearly interpolated lookup.
    let xi: usize = if (x as usize) < (res - 2) {
        x as usize
    } else {
        res - 2
    };
    let yi: usize = if (y as usize) < (res - 2) {
        y as usize
    } else {
        res - 2
    };
    let zi: usize = rgb2spec_find_interval(&model.scale, z);
    let offset: usize = ((i * res + zi) * res + yi) * res + xi;
    let dx: usize = 1;
    let dy: usize = 1 * res;
    let dz: usize = 1 * res * res;

    let x1: f32 = x - xi as f32;
    let x0: f32 = 1.0 - x1 as f32;
    let y1: f32 = y - yi as f32;
    let y0: f32 = 1.0 - y1 as f32;
    let z1: f32 = (z - model.scale[zi]) / (model.scale[zi + 1] - model.scale[zi]);
    let z0: f32 = 1.0 - z1 as f32;

    let a0 = model.data[offset];
    let a0 = Float4::new(a0[0], a0[1], a0[2], 0.0);
    let a1 = model.data[offset + dx];
    let a1 = Float4::new(a1[0], a1[1], a1[2], 0.0);
    let a2 = model.data[offset + dy];
    let a2 = Float4::new(a2[0], a2[1], a2[2], 0.0);
    let a3 = model.data[offset + dy + dx];
    let a3 = Float4::new(a3[0], a3[1], a3[2], 0.0);
    let a4 = model.data[offset + dz];
    let a4 = Float4::new(a4[0], a4[1], a4[2], 0.0);
    let a5 = model.data[offset + dz + dx];
    let a5 = Float4::new(a5[0], a5[1], a5[2], 0.0);
    let a6 = model.data[offset + dz + dy];
    let a6 = Float4::new(a6[0], a6[1], a6[2], 0.0);
    let a7 = model.data[offset + dz + dy + dx];
    let a7 = Float4::new(a7[0], a7[1], a7[2], 0.0);

    let b0 = (a0 * x0) + (a1 * x1);
    let b1 = (a2 * x0) + (a3 * x1);
    let b2 = (a4 * x0) + (a5 * x1);
    let b3 = (a6 * x0) + (a7 * x1);

    let c0 = (b0 * y0) + (b1 * y1);
    let c1 = (b2 * y0) + (b3 * y1);

    let d = (c0 * z0) + (c1 * z1);

    [d.get_0(), d.get_1(), d.get_2()]
}
