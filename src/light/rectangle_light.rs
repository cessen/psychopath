use math::{Vector, Point, Matrix4x4, cross};
use bbox::BBox;
use boundable::Boundable;
use color::{XYZ, SpectralSample, Color};
use super::LightSource;
use lerp::lerp_slice;
use sampling::{spherical_triangle_solid_angle, uniform_sample_spherical_triangle};
use std::f64::consts::PI as PI_64;
use std::f32::consts::PI as PI_32;

#[derive(Debug)]
pub struct RectangleLight {
    dimensions: Vec<(f32, f32)>,
    colors: Vec<XYZ>,
    bounds_: Vec<BBox>,
}

impl RectangleLight {
    pub fn new(dimensions: Vec<(f32, f32)>, colors: Vec<XYZ>) -> RectangleLight {
        let bbs = dimensions.iter()
            .map(|d| {
                BBox {
                    min: Point::new(d.0 * -0.5, d.1 * -0.5, 0.0),
                    max: Point::new(d.0 * 0.5, d.1 * 0.5, 0.0),
                }
            })
            .collect();
        RectangleLight {
            dimensions: dimensions,
            colors: colors,
            bounds_: bbs,
        }
    }
}

impl LightSource for RectangleLight {
    fn sample(&self,
              space: &Matrix4x4,
              arr: Point,
              u: f32,
              v: f32,
              wavelength: f32,
              time: f32)
              -> (SpectralSample, Vector, f32) {
        // Calculate time interpolated values
        let dim = lerp_slice(&self.dimensions, time);
        let col = lerp_slice(&self.colors, time);

        // TODO: Is this right?  Do we need to get the surface area post-transform?
        let surface_area_inv: f64 = 1.0 / (dim.0 as f64 * dim.1 as f64);

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

        // Normalize the solid angles for selection purposes
        let prob_1 = area_1 / (area_1 + area_2);
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
        let shadow_vec_local = shadow_vec_local * (-arr_local[2] / shadow_vec_local[2]);
        let mut sample_point_local = arr_local + shadow_vec_local;
        sample_point_local[0] = sample_point_local[0].max(dim.0 * -0.5).min(dim.0 * 0.5);
        sample_point_local[1] = sample_point_local[1].max(dim.1 * -0.5).min(dim.1 * 0.5);
        sample_point_local[2] = 0.0;
        let sample_point = sample_point_local * space_inv;
        let shadow_vec = sample_point - arr;

        // Calculate pdf and light energy
        let pdf = 1.0 / (area_1 + area_2); // PDF of the ray direction being sampled
        let spectral_sample = (col * surface_area_inv as f32 * 0.5).to_spectral_sample(wavelength);

        return (spectral_sample, shadow_vec, pdf as f32);
    }

    fn sample_pdf(&self,
                  space: &Matrix4x4,
                  arr: Point,
                  sample_dir: Vector,
                  sample_u: f32,
                  sample_v: f32,
                  wavelength: f32,
                  time: f32)
                  -> f32 {
        let dim = lerp_slice(&self.dimensions, time);

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

        1.0 / (area_1 + area_2)
    }

    fn outgoing(&self,
                space: &Matrix4x4,
                dir: Vector,
                u: f32,
                v: f32,
                wavelength: f32,
                time: f32)
                -> SpectralSample {
        let dim = lerp_slice(&self.dimensions, time);
        let col = lerp_slice(&self.colors, time);

        // TODO: Is this right?  Do we need to get the surface area post-transform?
        let surface_area_inv: f64 = 1.0 / (dim.0 as f64 * dim.1 as f64);

        (col * surface_area_inv as f32 * 0.5).to_spectral_sample(wavelength)
    }

    fn is_delta(&self) -> bool {
        false
    }
}

impl Boundable for RectangleLight {
    fn bounds<'a>(&'a self) -> &'a [BBox] {
        &self.bounds_
    }
}
