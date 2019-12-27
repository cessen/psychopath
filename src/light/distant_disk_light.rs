use std::f64::consts::PI as PI_64;

use kioku::Arena;

use crate::{
    color::{Color, SpectralSample},
    lerp::lerp_slice,
    math::{coordinate_system_from_vector, Vector},
    sampling::{uniform_sample_cone, uniform_sample_cone_pdf},
};

use super::WorldLightSource;

// TODO: handle case where radius = 0.0.

#[derive(Copy, Clone, Debug)]
pub struct DistantDiskLight<'a> {
    radii: &'a [f32],
    directions: &'a [Vector],
    colors: &'a [Color],
}

impl<'a> DistantDiskLight<'a> {
    pub fn new(
        arena: &'a Arena,
        radii: &[f32],
        directions: &[Vector],
        colors: &[Color],
    ) -> DistantDiskLight<'a> {
        DistantDiskLight {
            radii: arena.copy_slice(&radii),
            directions: arena.copy_slice(&directions),
            colors: arena.copy_slice(&colors),
        }
    }

    // fn sample_pdf(&self, sample_dir: Vector, wavelength: f32, time: f32) -> f32 {
    //     // We're not using these, silence warnings
    //     let _ = (sample_dir, wavelength);

    //     let radius: f64 = lerp_slice(self.radii, time) as f64;

    //     uniform_sample_cone_pdf(radius.cos()) as f32
    // }

    // fn outgoing(&self, dir: Vector, wavelength: f32, time: f32) -> SpectralSample {
    //     // We're not using this, silence warning
    //     let _ = dir;

    //     let radius = lerp_slice(self.radii, time) as f64;
    //     let col = lerp_slice(self.colors, time);
    //     let solid_angle = 2.0 * PI_64 * (1.0 - radius.cos());

    //     (col / solid_angle as f32).to_spectral_sample(wavelength)
    // }
}

impl<'a> WorldLightSource for DistantDiskLight<'a> {
    fn sample_from_point(
        &self,
        u: f32,
        v: f32,
        wavelength: f32,
        time: f32,
    ) -> (SpectralSample, Vector, f32) {
        // Calculate time interpolated values
        let radius: f64 = lerp_slice(self.radii, time) as f64;
        let direction = lerp_slice(self.directions, time);
        let col = lerp_slice(self.colors, time);
        let solid_angle_inv = 1.0 / (2.0 * PI_64 * (1.0 - radius.cos()));

        // Create a coordinate system from the vector pointing at the center of
        // of the light.
        let (z, x, y) = coordinate_system_from_vector(-direction.normalized());

        // Sample the cone subtended by the light.
        let cos_theta_max: f64 = radius.cos();
        let sample = uniform_sample_cone(u, v, cos_theta_max).normalized();

        // Calculate the final values and return everything.
        let spectral_sample = col.to_spectral_sample(wavelength) * solid_angle_inv as f32;
        let shadow_vec = (x * sample.x()) + (y * sample.y()) + (z * sample.z());
        let pdf = uniform_sample_cone_pdf(cos_theta_max);
        (spectral_sample, shadow_vec, pdf as f32)
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
