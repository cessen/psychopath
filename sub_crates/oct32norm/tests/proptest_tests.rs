#[macro_use]
extern crate proptest;
extern crate oct32norm;

use oct32norm::{decode, encode};
use proptest::test_runner::Config;

/// Calculates the cosine of the angle between the two vectors,
/// and checks to see if it's greater than the passed cos.
fn cos_gt(a: (f32, f32, f32), b: (f32, f32, f32), cos: f64) -> bool {
    fn normalize(v: (f32, f32, f32)) -> (f64, f64, f64) {
        let norm =
            ((v.0 as f64 * v.0 as f64) + (v.1 as f64 * v.1 as f64) + (v.2 as f64 * v.2 as f64))
                .sqrt();
        (v.0 as f64 / norm, v.1 as f64 / norm, v.2 as f64 / norm)
    }
    let a = normalize(a);
    let b = normalize(b);
    let cos2 = (a.0 * b.0) + (a.1 * b.1) + (a.2 * b.2);
    let r = cos2 > cos as f64;

    if !r {
        println!("cos: {}, left: {:?}, right: {:?}", cos2, a, b);
    }

    r
}

/// Checks if the difference between the two vectors on all axes is
/// less than delta.  Both vectors are L1-normalized first.
fn l1_delta_lt(a: (f32, f32, f32), b: (f32, f32, f32), delta: f32) -> bool {
    fn l1_normalize(v: (f32, f32, f32)) -> (f32, f32, f32) {
        let l1_norm = v.0.abs() + v.1.abs() + v.2.abs();
        (v.0 / l1_norm, v.1 / l1_norm, v.2 / l1_norm)
    }

    let a = l1_normalize(a);
    let b = l1_normalize(b);

    let rx = (a.0 - b.0).abs() < delta;
    let ry = (a.1 - b.1).abs() < delta;
    let rz = (a.2 - b.2).abs() < delta;

    let r = rx && ry && rz;

    if !r {
        println!("left: {:?}, right: {:?}", a, b);
    }

    r
}

proptest! {
    #![proptest_config(Config::with_cases(4096))]

    #[test]
    fn pt_roundtrip_angle_precision(v in (-1.0f32..1.0, -1.0f32..1.0, -1.0f32..1.0)) {
        let oct = encode(v);
        let v2 = decode(oct);

        // Check if the angle between the original and the roundtrip
        // is less than 0.004 degrees
        assert!(cos_gt(v, v2, 0.9999999976));
    }

    #[test]
    fn pt_roundtrip_component_precision(v in (-1.0f32..1.0, -1.0f32..1.0, -1.0f32..1.0)) {
        let oct = encode(v);
        let v2 = decode(oct);

        assert!(l1_delta_lt(v, v2, 0.00005));
    }
}
