use std::f64::consts::PI as PI_64;

use color::{XYZ, SpectralSample, Color};
use lerp::lerp_slice;
use math::{Vector, coordinate_system_from_vector};
use sampling::{uniform_sample_cone, uniform_sample_cone_pdf};

use super::WorldLightSource;

// TODO: handle case where radius = 0.0.

#[derive(Debug)]
pub struct DistantDiskLight {
    radii: Vec<f32>,
    directions: Vec<Vector>,
    colors: Vec<XYZ>,
}

impl DistantDiskLight {
    pub fn new(radii: Vec<f32>, directions: Vec<Vector>, colors: Vec<XYZ>) -> DistantDiskLight {
        DistantDiskLight {
            radii: radii,
            directions: directions,
            colors: colors,
        }
    }
}

impl WorldLightSource for DistantDiskLight {
    fn sample(&self, u: f32, v: f32, wavelength: f32, time: f32) -> (SpectralSample, Vector, f32) {
        // Calculate time interpolated values
        let radius: f64 = lerp_slice(&self.radii, time) as f64;
        let direction = lerp_slice(&self.directions, time);
        let col = lerp_slice(&self.colors, time);
        let solid_angle_inv = 1.0 / (2.0 * PI_64 * (1.0 - radius.cos()));

        // Create a coordinate system from the vector pointing at the center of
        // of the light.
        let (z, x, y) = coordinate_system_from_vector(-direction.normalized());

        // Sample the cone subtended by the light.
        let cos_theta_max: f64 = radius.cos();
        let sample = uniform_sample_cone(u, v, cos_theta_max).normalized();

        // Calculate the final values and return everything.
        let spectral_sample = (col * solid_angle_inv as f32).to_spectral_sample(wavelength);
        let shadow_vec = (x * sample.x()) + (y * sample.y()) + (z * sample.z());
        let pdf = uniform_sample_cone_pdf(cos_theta_max);

        return (spectral_sample, shadow_vec, pdf as f32);
    }

    fn sample_pdf(&self, sample_dir: Vector, wavelength: f32, time: f32) -> f32 {
        // We're not using these, silence warnings
        let _ = (sample_dir, wavelength);

        let radius: f64 = lerp_slice(&self.radii, time) as f64;
        return uniform_sample_cone_pdf(radius.cos()) as f32;
    }

    fn outgoing(&self, dir: Vector, wavelength: f32, time: f32) -> SpectralSample {
        // We're not using this, silence warning
        let _ = dir;

        let radius = lerp_slice(&self.radii, time) as f64;
        let col = lerp_slice(&self.colors, time);
        let solid_angle = 2.0 * PI_64 * (1.0 - radius.cos());
        (col / solid_angle as f32).to_spectral_sample(wavelength)
    }

    fn is_delta(&self) -> bool {
        false
    }

    fn approximate_energy(&self) -> f32 {
        let color: XYZ = self.colors.iter().fold(XYZ::new(0.0, 0.0, 0.0), |a, &b| a + b) /
                         self.colors.len() as f32;
        color.y
    }
}
