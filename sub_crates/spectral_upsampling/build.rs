// Get Jakob tables into a native rust format.

use std::{
    env,
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

/// How many polynomial coefficients?
const RGB2SPEC_N_COEFFS: usize = 3;

/// Table resolution.
const TABLE_RES: usize = 64;

// For the small table, what is the middle value used?
const MID_VALUE: f32 = 0.5;

fn main() {
    // Write tables to Rust file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("jakob_table_inc.rs");
    let mut f = File::create(&dest_path).unwrap();

    // Rec.709
    let rec709_table = rgb2spec_load_small("jakob_tables/srgb.coeff");
    f.write_all(format!("\nconst REC709_TABLE_RES: usize = {};", TABLE_RES).as_bytes())
        .unwrap();
    f.write_all(format!("\nconst REC709_TABLE_MID_VALUE: f32 = {};", MID_VALUE).as_bytes())
        .unwrap();
    f.write_all("\npub static REC709_TABLE: &[[(f32, f32, f32); 2]; 64 * 64 * 3] = &[".as_bytes())
        .unwrap();
    for item in &rec709_table {
        f.write_all(
            format!(
                "\n    [({}, {}, {}), ({}, {}, {})],",
                item[0].0, item[0].1, item[0].2, item[1].0, item[1].1, item[1].2
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write_all("\n];".as_bytes()).unwrap();

    // Rec.2020
    let rec2020_table = rgb2spec_load_small("jakob_tables/rec2020.coeff");
    f.write_all(format!("\nconst REC2020_TABLE_RES: usize = {};", TABLE_RES).as_bytes())
        .unwrap();
    f.write_all(format!("\nconst REC2020_TABLE_MID_VALUE: f32 = {};", MID_VALUE).as_bytes())
        .unwrap();
    f.write_all("\npub static REC2020_TABLE: &[[(f32, f32, f32); 2]; 64 * 64 * 3] = &[".as_bytes())
        .unwrap();
    for item in &rec2020_table {
        f.write_all(
            format!(
                "\n    [({}, {}, {}), ({}, {}, {})],",
                item[0].0, item[0].1, item[0].2, item[1].0, item[1].1, item[1].2
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write_all("\n];".as_bytes()).unwrap();

    // sRGB / ACES
    let aces_table = rgb2spec_load_small("jakob_tables/aces2065_1.coeff");
    f.write_all(format!("\nconst ACES_TABLE_RES: usize = {};", TABLE_RES).as_bytes())
        .unwrap();
    f.write_all(format!("\nconst ACES_TABLE_MID_VALUE: f32 = {};", MID_VALUE).as_bytes())
        .unwrap();
    f.write_all("\npub static ACES_TABLE: &[[(f32, f32, f32); 2]; 64 * 64 * 3] = &[".as_bytes())
        .unwrap();
    for item in &aces_table {
        f.write_all(
            format!(
                "\n    [({}, {}, {}), ({}, {}, {})],",
                item[0].0, item[0].1, item[0].2, item[1].0, item[1].1, item[1].2
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write_all("\n];".as_bytes()).unwrap();
}

/// Underlying representation
pub struct RGB2Spec {
    res: usize,
    scale: Vec<f32>,
    data: Vec<[f32; RGB2SPEC_N_COEFFS]>,
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

pub fn rgb2spec_load_small(filepath: &str) -> Vec<[(f32, f32, f32); 2]> {
    let big_table = rgb2spec_load(filepath);
    assert!(big_table.res == TABLE_RES);

    // Calculate z offsets and such for the mid value.
    let dz: usize = 1 * big_table.res * big_table.res;
    let z05_i = rgb2spec_find_interval(&big_table.scale, MID_VALUE);
    let z05_1: f32 = (MID_VALUE - big_table.scale[z05_i])
        / (big_table.scale[z05_i + 1] - big_table.scale[z05_i]);
    let z05_0: f32 = 1.0 - z05_1;

    // Fill in table.
    let mut table = vec![[(0.0, 0.0, 0.0); 2]; TABLE_RES * TABLE_RES * 3];
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
                (mid_coef[0], mid_coef[1], mid_coef[2]),
                (one_coef[0], one_coef[1], one_coef[2]),
            ];
        }
    }

    table
}

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
