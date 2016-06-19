use math::Vector;

use std::f64::consts::PI as PI_64;
use std::f32::consts::PI as PI_32;
use std::f32::consts::FRAC_PI_4 as QPI_32;

/// Maps the unit square to the unit circle.
/// Modifies x and y in place.
/// NOTE: x and y should be distributed within [-1, 1],
/// not [0, 1].
pub fn square_to_circle(x: f32, y: f32) -> (f32, f32) {
    debug_assert!(x >= -1.0 && x <= 1.0 && y >= -1.0 && y <= 1.0);

    if x == 0.0 && y == 0.0 {
        return (0.0, 0.0);
    }

    let (radius, angle) = if x > y.abs() {
        // Quadrant 1
        (x, QPI_32 * (y / x))
    } else if y > x.abs() {
        // Quadrant 2
        (y, QPI_32 * (2.0 - (x / y)))
    } else if x < -(y.abs()) {
        // Quadrant 3
        (-x, QPI_32 * (4.0 + (y / x)))
    } else {
        // Quadrant 4
        (-y, QPI_32 * (6.0 - (x / y)))
    };

    (radius * angle.cos(), radius * angle.sin())
}

pub fn cosine_sample_hemisphere(u: f32, v: f32) -> Vector {
    let (u, v) = square_to_circle((u * 2.0) - 1.0, (v * 2.0) - 1.0);
    let z = (1.0 - ((u * u) + (v * v))).max(0.0).sqrt();
    return Vector::new(u, v, z);
}

pub fn uniform_sample_hemisphere(u: f32, v: f32) -> Vector {
    let z = u;
    let r = (1.0 - (z * z)).max(0.0).sqrt();
    let phi = 2.0 * PI_32 * v;
    let x = r * phi.cos();
    let y = r * phi.sin();
    Vector::new(x, y, z)
}

pub fn uniform_sample_sphere(u: f32, v: f32) -> Vector {
    let z = 1.0 - (2.0 * u);
    let r = (1.0 - (z * z)).max(0.0).sqrt();
    let phi = 2.0 * PI_32 * v;
    let x = r * phi.cos();
    let y = r * phi.sin();
    Vector::new(x, y, z)
}

pub fn uniform_sample_cone(u: f32, v: f32, cos_theta_max: f64) -> Vector {
    let cos_theta = (1.0 - u as f64) + (u as f64 * cos_theta_max);
    let sin_theta = (1.0 - (cos_theta * cos_theta)).sqrt();
    let phi = v as f64 * 2.0 * PI_64;
    Vector::new((phi.cos() * sin_theta) as f32,
                (phi.sin() * sin_theta) as f32,
                cos_theta as f32)
}

pub fn uniform_sample_cone_pdf(cos_theta_max: f64) -> f64 {
    // 1.0 / solid angle
    1.0 / (2.0 * PI_64 * (1.0 - cos_theta_max))
}
