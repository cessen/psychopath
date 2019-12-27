use kioku::Arena;

use crate::{
    bbox::BBox,
    boundable::Boundable,
    color::{Color, SpectralSample},
    lerp::lerp_slice,
    math::{cross, dot, Matrix4x4, Normal, Point, Vector},
    ray::{RayBatch, RayStack},
    sampling::{
        spherical_triangle_solid_angle, triangle_surface_area, uniform_sample_spherical_triangle,
        uniform_sample_triangle,
    },
    shading::surface_closure::SurfaceClosure,
    shading::SurfaceShader,
    surface::{triangle, Surface, SurfaceIntersection, SurfaceIntersectionData},
};

use super::SurfaceLight;

const SIMPLE_SAMPLING_THRESHOLD: f32 = 0.01;

#[derive(Copy, Clone, Debug)]
pub struct RectangleLight<'a> {
    dimensions: &'a [(f32, f32)],
    colors: &'a [Color],
    bounds_: &'a [BBox],
}

impl<'a> RectangleLight<'a> {
    pub fn new<'b>(
        arena: &'b Arena,
        dimensions: &[(f32, f32)],
        colors: &[Color],
    ) -> RectangleLight<'b> {
        let bbs: Vec<_> = dimensions
            .iter()
            .map(|d| BBox {
                min: Point::new(d.0 * -0.5, d.1 * -0.5, 0.0),
                max: Point::new(d.0 * 0.5, d.1 * 0.5, 0.0),
            })
            .collect();
        RectangleLight {
            dimensions: arena.copy_slice(&dimensions),
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
        hit_point: Point,
        wavelength: f32,
        time: f32,
    ) -> f32 {
        // We're not using these, silence warnings
        let _ = wavelength;

        let dim = lerp_slice(self.dimensions, time);

        // Get the four corners of the rectangle, transformed into world space
        let space_inv = space.inverse();
        let p1 = Point::new(dim.0 * 0.5, dim.1 * 0.5, 0.0) * space_inv;
        let p2 = Point::new(dim.0 * -0.5, dim.1 * 0.5, 0.0) * space_inv;
        let p3 = Point::new(dim.0 * -0.5, dim.1 * -0.5, 0.0) * space_inv;
        let p4 = Point::new(dim.0 * 0.5, dim.1 * -0.5, 0.0) * space_inv;

        // Get the four corners of the rectangle, projected on to the unit
        // sphere centered around arr.
        let sp1 = (p1 - arr).normalized();
        let sp2 = (p2 - arr).normalized();
        let sp3 = (p3 - arr).normalized();
        let sp4 = (p4 - arr).normalized();

        // Get the solid angles of the rectangle split into two triangles
        let area_1 = spherical_triangle_solid_angle(sp2, sp1, sp3);
        let area_2 = spherical_triangle_solid_angle(sp4, sp1, sp3);

        // World-space surface normal
        let normal = Normal::new(0.0, 0.0, 1.0) * space_inv;

        // PDF
        if (area_1 + area_2) < SIMPLE_SAMPLING_THRESHOLD {
            let area = triangle_surface_area(p2, p1, p3) + triangle_surface_area(p4, p1, p3);
            (hit_point - arr).length2()
                / dot(sample_dir.normalized(), normal.into_vector().normalized()).abs()
                / area
        } else {
            1.0 / (area_1 + area_2)
        }
    }

    // fn outgoing(
    //     &self,
    //     space: &Matrix4x4,
    //     dir: Vector,
    //     u: f32,
    //     v: f32,
    //     wavelength: f32,
    //     time: f32,
    // ) -> SpectralSample {
    //     // We're not using these, silence warnings
    //     let _ = (space, dir, u, v);

    //     let dim = lerp_slice(self.dimensions, time);
    //     let col = lerp_slice(self.colors, time);

    //     // TODO: Is this right?  Do we need to get the surface area post-transform?
    //     let surface_area_inv: f64 = 1.0 / (dim.0 as f64 * dim.1 as f64);

    //     (col * surface_area_inv as f32 * 0.5).to_spectral_sample(wavelength)
    // }
}

impl<'a> SurfaceLight for RectangleLight<'a> {
    fn sample_from_point(
        &self,
        space: &Matrix4x4,
        arr: Point,
        u: f32,
        v: f32,
        wavelength: f32,
        time: f32,
    ) -> (SpectralSample, (Point, Normal, f32), f32) {
        // Calculate time interpolated values
        let dim = lerp_slice(self.dimensions, time);
        let col = lerp_slice(self.colors, time);

        let surface_area: f64 = dim.0 as f64 * dim.1 as f64;
        let surface_area_inv: f64 = 1.0 / surface_area;

        // Get the four corners of the rectangle, transformed into world space
        let space_inv = space.inverse();
        let p1 = Point::new(dim.0 * 0.5, dim.1 * 0.5, 0.0) * space_inv;
        let p2 = Point::new(dim.0 * -0.5, dim.1 * 0.5, 0.0) * space_inv;
        let p3 = Point::new(dim.0 * -0.5, dim.1 * -0.5, 0.0) * space_inv;
        let p4 = Point::new(dim.0 * 0.5, dim.1 * -0.5, 0.0) * space_inv;

        // Get the four corners of the rectangle relative to arr.
        let lp1 = p1 - arr;
        let lp2 = p2 - arr;
        let lp3 = p3 - arr;
        let lp4 = p4 - arr;

        // Four corners projected on to the unit sphere.
        let sp1 = lp1.normalized();
        let sp2 = lp2.normalized();
        let sp3 = lp3.normalized();
        let sp4 = lp4.normalized();

        // Get the solid angles of the rectangle split into two triangles
        let area_1 = spherical_triangle_solid_angle(sp2, sp1, sp3);
        let area_2 = spherical_triangle_solid_angle(sp4, sp1, sp3);

        // Calculate world-space surface normal
        let normal = Normal::new(0.0, 0.0, 1.0) * space_inv;

        if (area_1 + area_2) < SIMPLE_SAMPLING_THRESHOLD {
            // Simple sampling for more distant lights
            let surface_area_1 = triangle_surface_area(p2, p1, p3);
            let surface_area_2 = triangle_surface_area(p4, p1, p3);
            let sample_point = {
                // Select which triangle to sample
                let threshhold = surface_area_1 / (surface_area_1 + surface_area_2);
                if u < threshhold {
                    uniform_sample_triangle(
                        p2.into_vector(),
                        p1.into_vector(),
                        p3.into_vector(),
                        v,
                        u / threshhold,
                    )
                } else {
                    uniform_sample_triangle(
                        p4.into_vector(),
                        p1.into_vector(),
                        p3.into_vector(),
                        v,
                        (u - threshhold) / (1.0 - threshhold),
                    )
                }
            }
            .into_point();
            let shadow_vec = sample_point - arr;
            let spectral_sample =
                (col).to_spectral_sample(wavelength) * surface_area_inv as f32 * 0.5;
            let pdf = (sample_point - arr).length2()
                / dot(shadow_vec.normalized(), normal.into_vector().normalized()).abs()
                / (surface_area_1 + surface_area_2);
            let point_err = 0.0001; // TODO: this is a hack, do properly.
            (spectral_sample, (sample_point, normal, point_err), pdf)
        } else {
            // Sophisticated sampling for close lights.

            // Normalize the solid angles for selection purposes
            let prob_1 = if area_1.is_infinite() {
                1.0
            } else if area_2.is_infinite() {
                0.0
            } else {
                area_1 / (area_1 + area_2)
            };
            let prob_2 = 1.0 - prob_1;

            // Select one of the triangles and sample it
            let shadow_vec = if u < prob_1 {
                uniform_sample_spherical_triangle(sp2, sp1, sp3, v, u / prob_1)
            } else {
                uniform_sample_spherical_triangle(sp4, sp1, sp3, v, 1.0 - ((u - prob_1) / prob_2))
            };

            // Project shadow_vec back onto the light's surface
            let arr_local = arr * *space;
            let shadow_vec_local = shadow_vec * *space;
            let shadow_vec_local = shadow_vec_local * (-arr_local.z() / shadow_vec_local.z());
            let mut sample_point_local = arr_local + shadow_vec_local;
            {
                let x = sample_point_local.x().max(dim.0 * -0.5).min(dim.0 * 0.5);
                let y = sample_point_local.y().max(dim.1 * -0.5).min(dim.1 * 0.5);
                sample_point_local.set_x(x);
                sample_point_local.set_y(y);
                sample_point_local.set_z(0.0);
            }
            let sample_point = sample_point_local * space_inv;
            let point_err = 0.0001; // TODO: this is a hack, do properly.

            // Calculate pdf and light energy
            let pdf = 1.0 / (area_1 + area_2); // PDF of the ray direction being sampled
            let spectral_sample =
                col.to_spectral_sample(wavelength) * surface_area_inv as f32 * 0.5;

            (
                spectral_sample,
                (sample_point, normal, point_err),
                pdf as f32,
            )
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

impl<'a> Surface for RectangleLight<'a> {
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
            let orig = rays.orig(ray_idx);
            let dir = rays.dir(ray_idx);
            let max_t = rays.max_t(ray_idx);

            // Calculate time interpolated values
            let dim = lerp_slice(self.dimensions, time);
            let xform = lerp_slice(space, time);

            let space_inv = xform.inverse();

            // Get the four corners of the rectangle, transformed into world space
            let p1 = Point::new(dim.0 * 0.5, dim.1 * 0.5, 0.0) * space_inv;
            let p2 = Point::new(dim.0 * -0.5, dim.1 * 0.5, 0.0) * space_inv;
            let p3 = Point::new(dim.0 * -0.5, dim.1 * -0.5, 0.0) * space_inv;
            let p4 = Point::new(dim.0 * 0.5, dim.1 * -0.5, 0.0) * space_inv;

            // Test against two triangles that make up the light
            let ray_pre = triangle::RayTriPrecompute::new(dir);
            for tri in &[(p1, p2, p3), (p3, p4, p1)] {
                if let Some((t, b0, b1, b2)) = triangle::intersect_ray(orig, ray_pre, max_t, *tri) {
                    if t < max_t {
                        if rays.is_occlusion(ray_idx) {
                            isects[ray_idx] = SurfaceIntersection::Occlude;
                            rays.mark_done(ray_idx);
                        } else {
                            let (pos, pos_err) = triangle::surface_point(*tri, (b0, b1, b2));
                            let normal = cross(tri.0 - tri.1, tri.0 - tri.2).into_normal();

                            let intersection_data = SurfaceIntersectionData {
                                incoming: dir,
                                t: t,
                                pos: pos,
                                pos_err: pos_err,
                                nor: normal,
                                nor_g: normal,
                                local_space: xform,
                                sample_pdf: self.sample_pdf(
                                    &xform,
                                    orig,
                                    dir,
                                    pos,
                                    rays.wavelength(ray_idx),
                                    time,
                                ),
                            };

                            let closure = {
                                let inv_surface_area = (1.0 / (dim.0 as f64 * dim.1 as f64)) as f32;
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

                        break;
                    }
                }
            }
        });
    }
}

impl<'a> Boundable for RectangleLight<'a> {
    fn bounds(&self) -> &[BBox] {
        self.bounds_
    }
}
