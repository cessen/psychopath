#![allow(dead_code)]

use fp_utils::fp_gamma;
use math::Point;
use ray::Ray;

/// Intersects `ray` with `tri`, returning `Some((t, b0, b1, b2))`, or `None`
/// if no intersection.
///
/// Returned values:
///
/// * `t` is the ray t at the hit point.
/// * `b0`, `b1`, and `b2` are the barycentric coordinates of the triangle at
///   the hit point.
///
/// Uses the ray-triangle test from the paper "Watertight Ray/Triangle
/// Intersection" by Woop et al.
pub fn intersect_ray(ray: &Ray, tri: (Point, Point, Point)) -> Option<(f32, f32, f32, f32)> {
    // Calculate the permuted dimension indices for the new ray space.
    let (xi, yi, zi) = {
        let xabs = ray.dir.x().abs();
        let yabs = ray.dir.y().abs();
        let zabs = ray.dir.z().abs();

        if xabs > yabs && xabs > zabs {
            (1, 2, 0)
        } else if yabs > zabs {
            (2, 0, 1)
        } else {
            (0, 1, 2)
        }
    };

    let dir_x = ray.dir.get_n(xi);
    let dir_y = ray.dir.get_n(yi);
    let dir_z = ray.dir.get_n(zi);

    // Calculate shear constants.
    let sx = dir_x / dir_z;
    let sy = dir_y / dir_z;
    let sz = 1.0 / dir_z;

    // Calculate vertices in ray space.
    let p0 = tri.0 - ray.orig;
    let p1 = tri.1 - ray.orig;
    let p2 = tri.2 - ray.orig;

    let p0x = p0.get_n(xi) - (sx * p0.get_n(zi));
    let p0y = p0.get_n(yi) - (sy * p0.get_n(zi));
    let p1x = p1.get_n(xi) - (sx * p1.get_n(zi));
    let p1y = p1.get_n(yi) - (sy * p1.get_n(zi));
    let p2x = p2.get_n(xi) - (sx * p2.get_n(zi));
    let p2y = p2.get_n(yi) - (sy * p2.get_n(zi));

    // Calculate scaled barycentric coordinates.
    let mut e0 = (p1x * p2y) - (p1y * p2x);
    let mut e1 = (p2x * p0y) - (p2y * p0x);
    let mut e2 = (p0x * p1y) - (p0y * p1x);

    // Fallback to test against edges using double precision.
    if e0 == 0.0 || e1 == 0.0 || e2 == 0.0 {
        e0 = ((p1x as f64 * p2y as f64) - (p1y as f64 * p2x as f64)) as f32;
        e1 = ((p2x as f64 * p0y as f64) - (p2y as f64 * p0x as f64)) as f32;
        e2 = ((p0x as f64 * p1y as f64) - (p0y as f64 * p1x as f64)) as f32;
    }

    // Check if the ray hit the triangle.
    if (e0 < 0.0 || e1 < 0.0 || e2 < 0.0) && (e0 > 0.0 || e1 > 0.0 || e2 > 0.0) {
        return None;
    }

    // Determinant
    let det = e0 + e1 + e2;
    if det == 0.0 {
        return None;
    }

    // Calculate t of hitpoint.
    let p0z = sz * p0.get_n(zi);
    let p1z = sz * p1.get_n(zi);
    let p2z = sz * p2.get_n(zi);
    let t_scaled = (e0 * p0z) + (e1 * p1z) + (e2 * p2z);

    // Check if the hitpoint t is within ray min/max t.
    if (det > 0.0 && (t_scaled <= 0.0 || t_scaled > (ray.max_t * det)))
        || (det < 0.0 && (t_scaled >= 0.0 || t_scaled < (ray.max_t * det)))
    {
        return None;
    }

    // Calculate t and the hitpoint barycentric coordinates.
    let inv_det = 1.0 / det;
    let b0 = e0 * inv_det;
    let b1 = e1 * inv_det;
    let b2 = e2 * inv_det;
    let t = t_scaled * inv_det;

    // Check error bounds on t for very close hit points.
    // The technique used here is from "Physically Based Rendering: From Theory
    // to Implementation" third edition by Pharr et al.
    {
        // Calculate delta z
        let max_zt = max_abs_3(p0z, p1z, p2z);
        let dz = fp_gamma(3) * max_zt;

        // Calculate delta x and y
        let max_xt = max_abs_3(p0x, p1x, p2x);
        let max_yt = max_abs_3(p0y, p1y, p2y);
        let dx = fp_gamma(5) * (max_xt + max_zt);
        let dy = fp_gamma(5) * (max_yt + max_zt);

        // Calculate delta e
        let de = 2.0 * ((fp_gamma(2) * max_xt * max_yt) + (dy * max_xt + dx * max_yt));

        // Calculate delta t
        let max_e = max_abs_3(e0, e1, e2);
        let dt =
            3.0 * ((fp_gamma(3) * max_e * max_zt) + (de * max_zt + dz * max_e)) * inv_det.abs();

        // Finally, do the check
        if t <= dt {
            return None;
        }
    }

    // Return t and barycentric coordinates
    Some((t, b0, b1, b2))
}

/// Calculates a point on a triangle's surface at the given barycentric
/// coordinates.
///
/// Returns the point and the error magnitude of the point.
pub fn surface_point(tri: (Point, Point, Point), bary: (f32, f32, f32)) -> (Point, f32) {
    let pos = ((tri.0.into_vector() * bary.0)
        + (tri.1.into_vector() * bary.1)
        + (tri.2.into_vector() * bary.2))
        .into_point();

    let pos_err = (((tri.0.into_vector().abs() * bary.0)
        + (tri.1.into_vector().abs() * bary.1)
        + (tri.2.into_vector().abs() * bary.2))
        * fp_gamma(7))
    .co
    .h_max();

    (pos, pos_err)
}

fn max_abs_3(a: f32, b: f32, c: f32) -> f32 {
    let a = a.abs();
    let b = b.abs();
    let c = c.abs();

    if a > b && a > c {
        a
    } else if b > c {
        b
    } else {
        c
    }
}
