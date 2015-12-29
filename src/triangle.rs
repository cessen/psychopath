#![allow(dead_code)]

use std;
use ray::Ray;
use math::{Point, cross, dot};

/// Intersects ray with tri, returning (t, u, v), or None if no intersection.
pub fn intersect_ray(ray: &Ray, tri: (Point, Point, Point)) -> Option<(f32, f32, f32)> {
    let edge1 = tri.1 - tri.0;
    let edge2 = tri.2 - tri.0;
    let pvec = cross(ray.dir, edge2);
    let det = dot(edge1, pvec);

    if det <= -std::f32::EPSILON || det >= std::f32::EPSILON {
        let inv_det = 1.0 / det;
        let tvec = ray.orig - tri.0;
        let qvec = cross(tvec, edge1);

        let u = dot(tvec, pvec) * inv_det;
        if u < 0.0 || u > 1.0 {
            return None;
        }

        let v = dot(ray.dir, qvec) * inv_det;
        if v < 0.0 || (u + v) > 1.0 {
            return None;
        }

        let t = dot(edge2, qvec) * inv_det;
        return Some((t, u, v));
    } else {
        return None;
    }
}
