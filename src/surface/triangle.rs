#![allow(dead_code)]

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
    let t = (e0 * p0z) + (e1 * p1z) + (e2 * p2z);

    // Check if the hitpoint t is within ray min/max t.
    if det > 0.0 && (t <= 0.0 || t > (ray.max_t * det)) {
        return None;
    } else if det < 0.0 && (t >= 0.0 || t < (ray.max_t * det)) {
        return None;
    }

    // Return t and the hitpoint barycentric coordinates.
    let inv_det = 1.0 / det;
    Some((t * inv_det, e0 * inv_det, e1 * inv_det, e2 * inv_det))
}
