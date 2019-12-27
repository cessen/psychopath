use std::f64::consts::PI as PI_64;

use kioku::Arena;

use crate::{
    bbox::BBox,
    boundable::Boundable,
    color::{Color, SpectralSample},
    lerp::lerp_slice,
    math::{coordinate_system_from_vector, dot, Matrix4x4, Normal, Point, Vector},
    ray::{RayBatch, RayStack},
    sampling::{uniform_sample_cone, uniform_sample_cone_pdf, uniform_sample_sphere},
    shading::surface_closure::SurfaceClosure,
    shading::SurfaceShader,
    surface::{Surface, SurfaceIntersection, SurfaceIntersectionData},
};

use super::SurfaceLight;

// TODO: use proper error bounds for sample generation to avoid self-shadowing
// instead of these fudge factors.
const SAMPLE_POINT_FUDGE: f32 = 0.001;

// TODO: handle case where radius = 0.0.

#[derive(Copy, Clone, Debug)]
pub struct SphereLight<'a> {
    radii: &'a [f32],
    colors: &'a [Color],
    bounds_: &'a [BBox],
}

impl<'a> SphereLight<'a> {
    pub fn new<'b>(arena: &'b Arena, radii: &[f32], colors: &[Color]) -> SphereLight<'b> {
        let bbs: Vec<_> = radii
            .iter()
            .map(|r| BBox {
                min: Point::new(-*r, -*r, -*r),
                max: Point::new(*r, *r, *r),
            })
            .collect();
        SphereLight {
            radii: arena.copy_slice(&radii),
            colors: arena.copy_slice(&colors),
            bounds_: arena.copy_slice(&bbs),
        }
    }

    // TODO: this is only used from within `intersect_rays`, and could be done
    // more efficiently by inlining it there.
    fn sample_pdf(
        &self,
        space: &Matrix4x4,
        arr: Point,
        sample_dir: Vector,
        sample_u: f32,
        sample_v: f32,
        wavelength: f32,
        time: f32,
    ) -> f32 {
        // We're not using these, silence warnings
        let _ = (sample_dir, sample_u, sample_v, wavelength);

        let arr = arr * *space;
        let pos = Point::new(0.0, 0.0, 0.0);
        let radius: f64 = lerp_slice(self.radii, time) as f64;

        let d2: f64 = (pos - arr).length2() as f64; // Distance from center of sphere squared
        let d: f64 = d2.sqrt(); // Distance from center of sphere

        if d > radius {
            // Calculate the portion of the sphere visible from the point
            let sin_theta_max2: f64 = ((radius * radius) / d2).min(1.0);
            let cos_theta_max2: f64 = 1.0 - sin_theta_max2;
            let cos_theta_max: f64 = cos_theta_max2.sqrt();

            uniform_sample_cone_pdf(cos_theta_max) as f32
        } else {
            (1.0 / (4.0 * PI_64)) as f32
        }
    }
}

impl<'a> SurfaceLight for SphereLight<'a> {
    fn sample_from_point(
        &self,
        space: &Matrix4x4,
        arr: Point,
        u: f32,
        v: f32,
        wavelength: f32,
        time: f32,
    ) -> (SpectralSample, (Point, Normal, f32), f32) {
        // TODO: track fp error due to transforms
        let arr = arr * *space;
        let pos = Point::new(0.0, 0.0, 0.0);

        // Precalculate local->world space transform matrix
        let inv_space = space.inverse();

        // Calculate time interpolated values
        let radius: f64 = lerp_slice(self.radii, time) as f64;
        let col = lerp_slice(self.colors, time);
        let surface_area_inv: f64 = 1.0 / (4.0 * PI_64 * radius * radius);

        // Create a coordinate system from the vector between the
        // point and the center of the light
        let z = pos - arr;
        let d2: f64 = z.length2() as f64; // Distance from center of sphere squared
        let d = d2.sqrt(); // Distance from center of sphere
        let (z, x, y) = coordinate_system_from_vector(z);
        let (x, y, z) = (x.normalized(), y.normalized(), z.normalized());

        // Pre-calculate sample point error magnitude.
        // TODO: do this properly.  This is a total hack.
        let sample_point_err = {
            let v = Vector::new(radius as f32, radius as f32, radius as f32);
            let v2 = v * inv_space;
            v2.length() * SAMPLE_POINT_FUDGE
        };

        // If we're outside the sphere, sample the surface based on
        // the angle it subtends from the point being lit.
        if d > radius {
            // Calculate the portion of the sphere visible from the point
            let sin_theta_max2: f64 = ((radius * radius) / d2).min(1.0);
            let cos_theta_max2: f64 = 1.0 - sin_theta_max2;
            let sin_theta_max: f64 = sin_theta_max2.sqrt();
            let cos_theta_max: f64 = cos_theta_max2.sqrt();

            // Sample the cone subtended by the sphere and calculate
            // useful data from that.
            let sample = uniform_sample_cone(u, v, cos_theta_max).normalized();
            let cos_theta: f64 = sample.z() as f64;
            let cos_theta2: f64 = cos_theta * cos_theta;
            let sin_theta2: f64 = (1.0 - cos_theta2).max(0.0);
            let sin_theta: f64 = sin_theta2.sqrt();

            // Convert to a point on the sphere.
            // The technique for this is from "Akalin" on ompf2.com:
            // http://ompf2.com/viewtopic.php?f=3&t=1914#p4414
            let dd = 1.0 - (d2 * sin_theta * sin_theta / (radius * radius));
            let cos_a = if dd <= 0.0 {
                sin_theta_max
            } else {
                ((d / radius) * sin_theta2) + (cos_theta * dd.sqrt())
            };
            let sin_a = ((1.0 - (cos_a * cos_a)).max(0.0)).sqrt();
            let phi = v as f64 * 2.0 * PI_64;
            let sample = Vector::new(
                (phi.cos() * sin_a * radius) as f32,
                (phi.sin() * sin_a * radius) as f32,
                (d - (cos_a * radius)) as f32,
            );

            // Calculate the final values and return everything.
            let (sample_point, normal) = {
                let sample_vec = (x * sample.x()) + (y * sample.y()) + (z * sample.z());
                let normal = (arr + sample_vec).into_vector().normalized();
                let point = normal * radius as f32;
                (
                    point.into_point() * inv_space,
                    normal.into_normal() * inv_space,
                )
            };
            let pdf = uniform_sample_cone_pdf(cos_theta_max);
            let spectral_sample = col.to_spectral_sample(wavelength) * surface_area_inv as f32;
            return (
                spectral_sample,
                (sample_point, normal, sample_point_err),
                pdf as f32,
            );
        } else {
            // If we're inside the sphere, there's light from every direction.
            let (sample_point, normal) = {
                let sample_vec = uniform_sample_sphere(u, v);
                let normal = (arr + sample_vec).into_vector().normalized();
                let point = normal * radius as f32;
                (
                    point.into_point() * inv_space,
                    normal.into_normal() * inv_space,
                )
            };
            let pdf = 1.0 / (4.0 * PI_64);
            let spectral_sample = col.to_spectral_sample(wavelength) * surface_area_inv as f32;
            return (
                spectral_sample,
                (sample_point, normal, sample_point_err),
                pdf as f32,
            );
        }
    }

    fn is_delta(&self) -> bool {
        false
    }

    fn approximate_energy(&self) -> f32 {
        self.colors
            .iter()
            .fold(0.0, |a, &b| a + b.approximate_energy())
            / self.colors.len() as f32
    }
}

impl<'a> Surface for SphereLight<'a> {
    fn intersect_rays(
        &self,
        rays: &mut RayBatch,
        ray_stack: &mut RayStack,
        isects: &mut [SurfaceIntersection],
        shader: &dyn SurfaceShader,
        space: &[Matrix4x4],
    ) {
        let _ = shader; // Silence 'unused' warning

        ray_stack.pop_do_next_task(|ray_idx| {
            let time = rays.time(ray_idx);

            // Get the transform space
            let xform = lerp_slice(space, time);

            // Get the radius of the sphere at the ray's time
            let radius = lerp_slice(self.radii, time); // Radius of the sphere

            // Get the ray origin and direction in local space
            let orig = rays.orig_local(ray_idx).into_vector();
            let dir = rays.dir(ray_idx) * xform;

            // Code adapted to Rust from https://github.com/Tecla/Rayito
            // Ray-sphere intersection can result in either zero, one or two points
            // of intersection.  It turns into a quadratic equation, so we just find
            // the solution using the quadratic formula.  Note that there is a
            // slightly more stable form of it when computing it on a computer, and
            // we use that method to keep everything accurate.

            // Calculate quadratic coeffs
            let a = dir.length2();
            let b = 2.0 * dot(dir, orig);
            let c = orig.length2() - (radius * radius);

            let discriminant = (b * b) - (4.0 * a * c);
            if discriminant < 0.0 {
                // Discriminant less than zero?  No solution => no intersection.
                return;
            }
            let discriminant = discriminant.sqrt();

            // Compute a more stable form of our param t (t0 = q/a, t1 = c/q)
            // q = -0.5 * (b - sqrt(b * b - 4.0 * a * c)) if b < 0, or
            // q = -0.5 * (b + sqrt(b * b - 4.0 * a * c)) if b >= 0
            let q = if b < 0.0 {
                -0.5 * (b - discriminant)
            } else {
                -0.5 * (b + discriminant)
            };

            // Get our final parametric values
            let mut t0 = q / a;
            let mut t1 = if q != 0.0 { c / q } else { rays.max_t(ray_idx) };

            // Swap them so they are ordered right
            if t0 > t1 {
                use std::mem::swap;
                swap(&mut t0, &mut t1);
            }

            // Check our intersection for validity against this ray's extents
            if t0 > rays.max_t(ray_idx) || t1 <= 0.0 {
                // Didn't hit because sphere is entirely outside of ray's extents
                return;
            }

            let t = if t0 > 0.0 {
                t0
            } else if t1 <= rays.max_t(ray_idx) {
                t1
            } else {
                // Didn't hit because ray is entirely within the sphere, and
                // therefore doesn't hit its surface.
                return;
            };

            // We hit the sphere, so calculate intersection info.
            if rays.is_occlusion(ray_idx) {
                isects[ray_idx] = SurfaceIntersection::Occlude;
                rays.mark_done(ray_idx);
            } else {
                let inv_xform = xform.inverse();

                // Position is calculated from the local-space ray and t, and then
                // re-projected onto the surface of the sphere.
                let t_pos = orig + (dir * t);
                let unit_pos = t_pos.normalized();
                let pos = (unit_pos * radius * inv_xform).into_point();

                // TODO: proper error bounds.
                let pos_err = 0.001;

                let normal = unit_pos.into_normal() * inv_xform;

                let intersection_data = SurfaceIntersectionData {
                    incoming: rays.dir(ray_idx),
                    t: t,
                    pos: pos,
                    pos_err: pos_err,
                    nor: normal,
                    nor_g: normal,
                    local_space: xform,
                    sample_pdf: self.sample_pdf(
                        &xform,
                        rays.orig(ray_idx),
                        rays.dir(ray_idx),
                        0.0,
                        0.0,
                        rays.wavelength(ray_idx),
                        time,
                    ),
                };

                let closure = {
                    let inv_surface_area =
                        (1.0 / (4.0 * PI_64 * radius as f64 * radius as f64)) as f32;
                    let color = lerp_slice(self.colors, time) * inv_surface_area;
                    SurfaceClosure::Emit(color)
                };

                // Fill in intersection
                isects[ray_idx] = SurfaceIntersection::Hit {
                    intersection_data: intersection_data,
                    closure: closure,
                };

                // Set ray's max t
                rays.set_max_t(ray_idx, t);
            }
        });
    }
}

impl<'a> Boundable for SphereLight<'a> {
    fn bounds(&self) -> &[BBox] {
        self.bounds_
    }
}
