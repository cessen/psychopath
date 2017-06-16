use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;


#[derive(Copy, Clone)]
struct Chromaticities {
    r: (f64, f64),
    g: (f64, f64),
    b: (f64, f64),
    w: (f64, f64),
}


fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Rec709
    {
        let chroma = Chromaticities {
            r: (0.640, 0.330),
            g: (0.300, 0.600),
            b: (0.150, 0.060),
            w: (0.3127, 0.3290),
        };

        let to_xyz = rgb_to_xyz(chroma, 1.0);
        let dest_path = Path::new(&out_dir).join("rec709_inc.rs");
        let mut f = File::create(&dest_path).unwrap();
        write_conversion_functions("rec709", to_xyz, &mut f);
    }

    // Rec2020
    {
        let chroma = Chromaticities {
            r: (0.708, 0.292),
            g: (0.170, 0.797),
            b: (0.131, 0.046),
            w: (0.3127, 0.3290),
        };

        let to_xyz = rgb_to_xyz(chroma, 1.0);
        let dest_path = Path::new(&out_dir).join("rec2020_inc.rs");
        let mut f = File::create(&dest_path).unwrap();
        write_conversion_functions("rec2020", to_xyz, &mut f);
    }

    // ACES AP0
    {
        let chroma = Chromaticities {
            r: (0.73470, 0.26530),
            g: (0.00000, 1.00000),
            b: (0.00010, -0.07700),
            w: (0.32168, 0.33767),
        };

        let to_xyz = rgb_to_xyz(chroma, 1.0);
        let dest_path = Path::new(&out_dir).join("aces_ap0_inc.rs");
        let mut f = File::create(&dest_path).unwrap();
        write_conversion_functions("aces_ap0", to_xyz, &mut f);
    }

    // ACES AP1
    {
        let chroma = Chromaticities {
            r: (0.713, 0.293),
            g: (0.165, 0.830),
            b: (0.128, 0.044),
            w: (0.32168, 0.33767),
        };

        let to_xyz = rgb_to_xyz(chroma, 1.0);
        let dest_path = Path::new(&out_dir).join("aces_ap1_inc.rs");
        let mut f = File::create(&dest_path).unwrap();
        write_conversion_functions("aces_ap1", to_xyz, &mut f);
    }
}


/// Generates conversion functions for the given rgb to xyz transform matrix.
fn write_conversion_functions(space_name: &str, to_xyz: [[f64; 3]; 3], f: &mut File) {

    f.write_all(
        format!(
            r#"
#[inline]
pub fn {}_to_xyz(rgb: (f32, f32, f32)) -> (f32, f32, f32) {{
    (
        (rgb.0 * {:.10}) + (rgb.1 * {:.10}) + (rgb.2 * {:.10}),
        (rgb.0 * {:.10}) + (rgb.1 * {:.10}) + (rgb.2 * {:.10}),
        (rgb.0 * {:.10}) + (rgb.1 * {:.10}) + (rgb.2 * {:.10}),
    )
}}
        "#,
            space_name,
            to_xyz[0][0],
            to_xyz[0][1],
            to_xyz[0][2],
            to_xyz[1][0],
            to_xyz[1][1],
            to_xyz[1][2],
            to_xyz[2][0],
            to_xyz[2][1],
            to_xyz[2][2]
        ).as_bytes(),
    ).unwrap();

    let inv = inverse(to_xyz);
    f.write_all(
        format!(
            r#"
#[inline]
pub fn xyz_to_{}(xyz: (f32, f32, f32)) -> (f32, f32, f32) {{
    (
        (xyz.0 * {:.10}) + (xyz.1 * {:.10}) + (xyz.2 * {:.10}),
        (xyz.0 * {:.10}) + (xyz.1 * {:.10}) + (xyz.2 * {:.10}),
        (xyz.0 * {:.10}) + (xyz.1 * {:.10}) + (xyz.2 * {:.10}),
    )
}}
        "#,
            space_name,
            inv[0][0],
            inv[0][1],
            inv[0][2],
            inv[1][0],
            inv[1][1],
            inv[1][2],
            inv[2][0],
            inv[2][1],
            inv[2][2]
        ).as_bytes(),
    ).unwrap();

    let e_to_xyz = adapt_to_e(to_xyz, 1.0);
    f.write_all(
        format!(
            r#"
#[inline]
pub fn {}_e_to_xyz(rgb: (f32, f32, f32)) -> (f32, f32, f32) {{
    (
        (rgb.0 * {:.10}) + (rgb.1 * {:.10}) + (rgb.2 * {:.10}),
        (rgb.0 * {:.10}) + (rgb.1 * {:.10}) + (rgb.2 * {:.10}),
        (rgb.0 * {:.10}) + (rgb.1 * {:.10}) + (rgb.2 * {:.10}),
    )
}}
        "#,
            space_name,
            e_to_xyz[0][0],
            e_to_xyz[0][1],
            e_to_xyz[0][2],
            e_to_xyz[1][0],
            e_to_xyz[1][1],
            e_to_xyz[1][2],
            e_to_xyz[2][0],
            e_to_xyz[2][1],
            e_to_xyz[2][2]
        ).as_bytes(),
    ).unwrap();

    let inv_e = inverse(e_to_xyz);
    f.write_all(
        format!(
            r#"
#[inline]
pub fn xyz_to_{}_e(xyz: (f32, f32, f32)) -> (f32, f32, f32) {{
    (
        (xyz.0 * {:.10}) + (xyz.1 * {:.10}) + (xyz.2 * {:.10}),
        (xyz.0 * {:.10}) + (xyz.1 * {:.10}) + (xyz.2 * {:.10}),
        (xyz.0 * {:.10}) + (xyz.1 * {:.10}) + (xyz.2 * {:.10}),
    )
}}
        "#,
            space_name,
            inv_e[0][0],
            inv_e[0][1],
            inv_e[0][2],
            inv_e[1][0],
            inv_e[1][1],
            inv_e[1][2],
            inv_e[2][0],
            inv_e[2][1],
            inv_e[2][2]
        ).as_bytes(),
    ).unwrap();
}


/// Port of the RGBtoXYZ function from the ACES CTL reference implementation.
/// See lib/IlmCtlMath/CtlColorSpace.cpp in the CTL reference implementation.
///
/// This takes the chromaticities of an RGB colorspace and generates a
/// transform matrix from that space to XYZ.
///
/// * `chroma` is the chromaticities.
/// * `y` is the XYZ "Y" value that should map to RGB (1,1,1)
fn rgb_to_xyz(chroma: Chromaticities, y: f64) -> [[f64; 3]; 3] {
    // X and Z values of RGB value (1, 1, 1), or "white"
    let x = chroma.w.0 * y / chroma.w.1;
    let z = (1.0 - chroma.w.0 - chroma.w.1) * y / chroma.w.1;

    // Scale factors for matrix rows
    let d = chroma.r.0 * (chroma.b.1 - chroma.g.1) + chroma.b.0 * (chroma.g.1 - chroma.r.1) +
        chroma.g.0 * (chroma.r.1 - chroma.b.1);

    let sr = (x * (chroma.b.1 - chroma.g.1) -
                  chroma.g.0 * (y * (chroma.b.1 - 1.0) + chroma.b.1 * (x + z)) +
                  chroma.b.0 * (y * (chroma.g.1 - 1.0) + chroma.g.1 * (x + z))) / d;

    let sg = (x * (chroma.r.1 - chroma.b.1) +
                  chroma.r.0 * (y * (chroma.b.1 - 1.0) + chroma.b.1 * (x + z)) -
                  chroma.b.0 * (y * (chroma.r.1 - 1.0) + chroma.r.1 * (x + z))) / d;

    let sb = (x * (chroma.g.1 - chroma.r.1) -
                  chroma.r.0 * (y * (chroma.g.1 - 1.0) + chroma.g.1 * (x + z)) +
                  chroma.g.0 * (y * (chroma.r.1 - 1.0) + chroma.r.1 * (x + z))) / d;

    // Assemble the matrix
    let mut mat = [[0.0; 3]; 3];

    mat[0][0] = sr * chroma.r.0;
    mat[0][1] = sg * chroma.g.0;
    mat[0][2] = sb * chroma.b.0;

    mat[1][0] = sr * chroma.r.1;
    mat[1][1] = sg * chroma.g.1;
    mat[1][2] = sb * chroma.b.1;

    mat[2][0] = sr * (1.0 - chroma.r.0 - chroma.r.1);
    mat[2][1] = sg * (1.0 - chroma.g.0 - chroma.g.1);
    mat[2][2] = sb * (1.0 - chroma.b.0 - chroma.b.1);

    mat
}


/// Chromatically adapts a matrix from `rgb_to_xyz` to a whitepoint of E.
///
/// In other words, makes it so that RGB (1,1,1) maps to XYZ (1,1,1).
fn adapt_to_e(mat: [[f64; 3]; 3], y: f64) -> [[f64; 3]; 3] {
    let r_fac = y / (mat[0][0] + mat[0][1] + mat[0][2]);
    let g_fac = y / (mat[1][0] + mat[1][1] + mat[1][2]);
    let b_fac = y / (mat[2][0] + mat[2][1] + mat[2][2]);

    let mut mat2 = [[0.0; 3]; 3];

    mat2[0][0] = mat[0][0] * r_fac;
    mat2[0][1] = mat[0][1] * r_fac;
    mat2[0][2] = mat[0][2] * r_fac;

    mat2[1][0] = mat[1][0] * g_fac;
    mat2[1][1] = mat[1][1] * g_fac;
    mat2[1][2] = mat[1][2] * g_fac;

    mat2[2][0] = mat[2][0] * b_fac;
    mat2[2][1] = mat[2][1] * b_fac;
    mat2[2][2] = mat[2][2] * b_fac;

    mat2
}


/// Calculates the inverse of the given 3x3 matrix.
///
/// Ported to Rust from `gjInverse()` in IlmBase's Imath/ImathMatrix.h
fn inverse(m: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut s = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut t = m;

    // Forward elimination
    for i in 0..2 {
        let mut pivot = i;
        let mut pivotsize = t[i][i];

        if pivotsize < 0.0 {
            pivotsize = -pivotsize;
        }

        for j in (i + 1)..3 {
            let mut tmp = t[j][i];

            if tmp < 0.0 {
                tmp = -tmp;
            }

            if tmp > pivotsize {
                pivot = j;
                pivotsize = tmp;
            }
        }

        if pivotsize == 0.0 {
            panic!("Cannot invert singular matrix.");
        }

        if pivot != i {
            for j in 0..3 {
                let mut tmp = t[i][j];
                t[i][j] = t[pivot][j];
                t[pivot][j] = tmp;

                tmp = s[i][j];
                s[i][j] = s[pivot][j];
                s[pivot][j] = tmp;
            }
        }

        for j in (i + 1)..3 {
            let f = t[j][i] / t[i][i];

            for k in 0..3 {
                t[j][k] -= f * t[i][k];
                s[j][k] -= f * s[i][k];
            }
        }
    }

    // Backward substitution
    for i in (0..3).rev() {
        let f = t[i][i];

        if t[i][i] == 0.0 {
            panic!("Cannot invert singular matrix.");
        }

        for j in 0..3 {
            t[i][j] /= f;
            s[i][j] /= f;
        }

        for j in 0..i {
            let f = t[j][i];

            for k in 0..3 {
                t[j][k] -= f * t[i][k];
                s[j][k] -= f * s[i][k];
            }
        }
    }

    s
}
