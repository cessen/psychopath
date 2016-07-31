use math::{Vector, Point, Matrix4x4, coordinate_system_from_vector};
use bbox::BBox;
use boundable::Boundable;
use color::{XYZ, SpectralSample, Color};
use super::LightSource;
use lerp::lerp_slice;
use sampling::{uniform_sample_cone, uniform_sample_cone_pdf, uniform_sample_sphere};
use std::f64::consts::PI as PI_64;

#[derive(Debug)]
pub struct SphereLight {
    radii: Vec<f32>,
    colors: Vec<XYZ>,
    bounds_: Vec<BBox>,
}

impl SphereLight {
    pub fn new(radii: Vec<f32>, colors: Vec<XYZ>) -> SphereLight {
        let bbs = radii.iter()
            .map(|r| {
                BBox {
                    min: Point::new(-*r, -*r, -*r),
                    max: Point::new(*r, *r, *r),
                }
            })
            .collect();
        SphereLight {
            radii: radii,
            colors: colors,
            bounds_: bbs,
        }
    }
}

impl LightSource for SphereLight {
    fn sample(&self,
              space: &Matrix4x4,
              arr: Point,
              u: f32,
              v: f32,
              wavelength: f32,
              time: f32)
              -> (SpectralSample, Vector, f32) {
        // TODO: use transform space correctly
        let pos = Point::new(0.0, 0.0, 0.0) * space.inverse();
        // Calculate time interpolated values
        let radius: f64 = lerp_slice(&self.radii, time) as f64;
        let col = lerp_slice(&self.colors, time);
        let surface_area_inv: f64 = 1.0 / (4.0 * PI_64 * radius * radius);


        // Create a coordinate system from the vector between the
        // point and the center of the light
        let z = pos - arr;
        let d2: f64 = z.length2() as f64;  // Distance from center of sphere squared
        let d = d2.sqrt(); // Distance from center of sphere
        let (z, x, y) = coordinate_system_from_vector(z);
        let (x, y, z) = (x.normalized(), y.normalized(), z.normalized());

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
            let sample = Vector::new((phi.cos() * sin_a * radius) as f32,
                                     (phi.sin() * sin_a * radius) as f32,
                                     (d - (cos_a * radius)) as f32);

            // Calculate the final values and return everything.
            let shadow_vec = (x * sample.x()) + (y * sample.y()) + (z * sample.z());
            let pdf = uniform_sample_cone_pdf(cos_theta_max);
            let spectral_sample = (col * surface_area_inv as f32).to_spectral_sample(wavelength);
            return (spectral_sample, shadow_vec, pdf as f32);
        } else {
            // If we're inside the sphere, there's light from every direction.
            let shadow_vec = uniform_sample_sphere(u, v);
            let pdf = 1.0 / (4.0 * PI_64);
            let spectral_sample = (col * surface_area_inv as f32).to_spectral_sample(wavelength);
            return (spectral_sample, shadow_vec, pdf as f32);
        }
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
        // We're not using these, silence warnings
        let _ = (sample_dir, sample_u, sample_v, wavelength);

        // TODO: use transform space correctly
        let pos = Point::new(0.0, 0.0, 0.0) * space.inverse();
        let radius: f64 = lerp_slice(&self.radii, time) as f64;

        let d2: f64 = (pos - arr).length2() as f64;  // Distance from center of sphere squared
        let d: f64 = d2.sqrt(); // Distance from center of sphere

        if d > radius {
            // Calculate the portion of the sphere visible from the point
            let sin_theta_max2: f64 = ((radius * radius) / d2).min(1.0);
            let cos_theta_max2: f64 = 1.0 - sin_theta_max2;
            let cos_theta_max: f64 = cos_theta_max2.sqrt();

            return uniform_sample_cone_pdf(cos_theta_max) as f32;
        } else {
            return (1.0 / (4.0 * PI_64)) as f32;
        }
    }

    fn outgoing(&self,
                space: &Matrix4x4,
                dir: Vector,
                u: f32,
                v: f32,
                wavelength: f32,
                time: f32)
                -> SpectralSample {
        // We're not using these, silence warnings
        let _ = (space, dir, u, v);

        // TODO: use transform space correctly
        let radius = lerp_slice(&self.radii, time) as f64;
        let col = lerp_slice(&self.colors, time);
        let surface_area = 4.0 * PI_64 * radius * radius;
        (col / surface_area as f32).to_spectral_sample(wavelength)
    }

    fn is_delta(&self) -> bool {
        false
    }
}

impl Boundable for SphereLight {
    fn bounds<'a>(&'a self) -> &'a [BBox] {
        &self.bounds_
    }
}
