#![allow(dead_code)]

use std::f32::consts::FRAC_PI_4 as QPI_32;
use std::f32::consts::PI as PI_32;
use std::f64::consts::PI as PI_64;

use crate::math::{cross, dot, Point, Vector};

/// Maps the unit square to the unit circle.
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
    Vector::new(u, v, z)
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

/// Samples a solid angle defined by a cone originating from (0,0,0)
/// and pointing down the positive z-axis.
///
/// `u`, `v`: sampling variables, should each be in the interval [0,1]
/// `cos_theta_max`: cosine of the max angle from the z-axis, defining
///                  the outer extent of the cone.
pub fn uniform_sample_cone(u: f32, v: f32, cos_theta_max: f64) -> Vector {
    let cos_theta = (1.0 - u as f64) + (u as f64 * cos_theta_max);
    let sin_theta = (1.0 - (cos_theta * cos_theta)).sqrt();
    let phi = v as f64 * 2.0 * PI_64;
    Vector::new(
        (phi.cos() * sin_theta) as f32,
        (phi.sin() * sin_theta) as f32,
        cos_theta as f32,
    )
}

pub fn uniform_sample_cone_pdf(cos_theta_max: f64) -> f64 {
    // 1.0 / solid angle
    1.0 / (2.0 * PI_64 * (1.0 - cos_theta_max))
}

/// Generates a uniform sample on a triangle given two uniform random
/// variables i and j in [0, 1].
pub fn uniform_sample_triangle(va: Vector, vb: Vector, vc: Vector, i: f32, j: f32) -> Vector {
    let isqrt = i.sqrt();
    let a = 1.0 - isqrt;
    let b = isqrt * (1.0 - j);
    let c = j * isqrt;

    (va * a) + (vb * b) + (vc * c)
}

/// Calculates the surface area of a triangle.
pub fn triangle_surface_area(p0: Point, p1: Point, p2: Point) -> f32 {
    0.5 * cross(p1 - p0, p2 - p0).length()
}

/// Calculates the projected solid angle of a spherical triangle.
///
/// A, B, and C are the points of the triangle on a unit sphere.
pub fn spherical_triangle_solid_angle(va: Vector, vb: Vector, vc: Vector) -> f32 {
    // Calculate sines and cosines of the spherical triangle's edge lengths
    let cos_a: f64 = dot(vb, vc).max(-1.0).min(1.0) as f64;
    let cos_b: f64 = dot(vc, va).max(-1.0).min(1.0) as f64;
    let cos_c: f64 = dot(va, vb).max(-1.0).min(1.0) as f64;
    let sin_a: f64 = (1.0 - (cos_a * cos_a)).sqrt();
    let sin_b: f64 = (1.0 - (cos_b * cos_b)).sqrt();
    let sin_c: f64 = (1.0 - (cos_c * cos_c)).sqrt();

    // If two of the vertices are coincident, area is zero.
    // Return early to avoid a divide by zero below.
    if cos_a == 1.0 || cos_b == 1.0 || cos_c == 1.0 {
        return 0.0;
    }

    // Calculate the cosine of the angles at the vertices
    let cos_va = ((cos_a - (cos_b * cos_c)) / (sin_b * sin_c))
        .max(-1.0)
        .min(1.0);
    let cos_vb = ((cos_b - (cos_c * cos_a)) / (sin_c * sin_a))
        .max(-1.0)
        .min(1.0);
    let cos_vc = ((cos_c - (cos_a * cos_b)) / (sin_a * sin_b))
        .max(-1.0)
        .min(1.0);

    // Calculate the angles themselves, in radians
    let ang_va = cos_va.acos();
    let ang_vb = cos_vb.acos();
    let ang_vc = cos_vc.acos();

    // Calculate and return the solid angle of the triangle
    (ang_va + ang_vb + ang_vc - PI_64) as f32
}

/// Generates a uniform sample on a spherical triangle given two uniform
/// random variables i and j in [0, 1].
pub fn uniform_sample_spherical_triangle(
    va: Vector,
    vb: Vector,
    vc: Vector,
    i: f32,
    j: f32,
) -> Vector {
    // Calculate sines and cosines of the spherical triangle's edge lengths
    let cos_a: f64 = dot(vb, vc).max(-1.0).min(1.0) as f64;
    let cos_b: f64 = dot(vc, va).max(-1.0).min(1.0) as f64;
    let cos_c: f64 = dot(va, vb).max(-1.0).min(1.0) as f64;
    let sin_a: f64 = (1.0 - (cos_a * cos_a)).sqrt();
    let sin_b: f64 = (1.0 - (cos_b * cos_b)).sqrt();
    let sin_c: f64 = (1.0 - (cos_c * cos_c)).sqrt();

    // If two of the vertices are coincident, area is zero.
    // Return early to avoid a divide by zero below.
    if cos_a == 1.0 || cos_b == 1.0 || cos_c == 1.0 {
        // TODO: do something more intelligent here, in the case that it's
        // an infinitely thin line.
        return va;
    }

    // Calculate the cosine of the angles at the vertices
    let cos_va = ((cos_a - (cos_b * cos_c)) / (sin_b * sin_c))
        .max(-1.0)
        .min(1.0);
    let cos_vb = ((cos_b - (cos_c * cos_a)) / (sin_c * sin_a))
        .max(-1.0)
        .min(1.0);
    let cos_vc = ((cos_c - (cos_a * cos_b)) / (sin_a * sin_b))
        .max(-1.0)
        .min(1.0);

    // Calculate sine for A
    let sin_va = (1.0 - (cos_va * cos_va)).sqrt();

    // Calculate the angles themselves, in radians
    let ang_va = cos_va.acos();
    let ang_vb = cos_vb.acos();
    let ang_vc = cos_vc.acos();

    // Calculate the area of the spherical triangle
    let area = ang_va + ang_vb + ang_vc - PI_64;

    // The rest of this is from the paper "Stratified Sampling of Spherical
    // Triangles" by James Arvo.
    let area_2 = area * i as f64;

    let s = (area_2 - ang_va).sin();
    let t = (area_2 - ang_va).cos();
    let u = t - cos_va;
    let v = s + (sin_va * cos_c);

    let q_top = (((v * t) - (u * s)) * cos_va) - v;
    let q_bottom = ((v * s) + (u * t)) * sin_va;
    let q = q_top / q_bottom;

    let vc_2 =
        (va * q as f32) + ((vc - (va * dot(vc, va))).normalized() * (1.0 - (q * q)).sqrt() as f32);

    let z = 1.0 - (j * (1.0 - dot(vc_2, vb)));

    (vb * z) + ((vc_2 - (vb * dot(vc_2, vb))).normalized() * (1.0 - (z * z)).sqrt())
}
