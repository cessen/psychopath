#![allow(dead_code)]

use kioku::Arena;

use crate::{
    lerp::lerp_slice,
    math::{Matrix4x4, Point, Vector},
    ray::Ray,
    sampling::square_to_circle,
};

#[derive(Copy, Clone, Debug)]
pub struct Camera<'a> {
    transforms: &'a [Matrix4x4],
    fovs: &'a [f32],
    tfovs: &'a [f32],
    aperture_radii: &'a [f32],
    focus_distances: &'a [f32],
}

impl<'a> Camera<'a> {
    pub fn new(
        arena: &'a Arena,
        transforms: &[Matrix4x4],
        fovs: &[f32],
        mut aperture_radii: &[f32],
        mut focus_distances: &[f32],
    ) -> Camera<'a> {
        assert!(!transforms.is_empty(), "Camera has no transform(s)!");
        assert!(!fovs.is_empty(), "Camera has no fov(s)!");

        // Aperture needs focus distance and vice-versa.
        if aperture_radii.is_empty() || focus_distances.is_empty() {
            aperture_radii = &[0.0];
            focus_distances = &[1.0];

            if aperture_radii.is_empty() && !focus_distances.is_empty() {
                println!(
                    "WARNING: camera has aperture radius but no focus distance.  Disabling \
                     focal blur."
                );
            } else if !aperture_radii.is_empty() && focus_distances.is_empty() {
                println!(
                    "WARNING: camera has focus distance but no aperture radius.  Disabling \
                     focal blur."
                );
            }
        }

        // Can't have focus distance of zero.
        if focus_distances.iter().any(|d| *d == 0.0) {
            if aperture_radii.iter().any(|a| *a > 0.0) {
                println!("WARNING: camera focal distance is zero or less.  Disabling focal blur.");
            }
            aperture_radii = &[0.0];
            focus_distances = &[1.0];
        }

        // Convert angle fov into linear fov.
        let tfovs: Vec<f32> = fovs
            .iter()
            .map(|n| (n / 2.0).sin() / (n / 2.0).cos())
            .collect();

        Camera {
            transforms: arena.copy_slice(&transforms),
            fovs: arena.copy_slice(&fovs),
            tfovs: arena.copy_slice(&tfovs),
            aperture_radii: arena.copy_slice(&aperture_radii),
            focus_distances: arena.copy_slice(&focus_distances),
        }
    }

    pub fn generate_ray(&self, x: f32, y: f32, time: f32, wavelength: f32, u: f32, v: f32) -> Ray {
        // Get time-interpolated camera settings
        let transform = lerp_slice(self.transforms, time);
        let tfov = lerp_slice(self.tfovs, time);
        let aperture_radius = lerp_slice(self.aperture_radii, time);
        let focus_distance = lerp_slice(self.focus_distances, time);

        // Ray origin
        let orig = {
            let (u, v) = square_to_circle((u * 2.0) - 1.0, (v * 2.0) - 1.0);
            Point::new(aperture_radius * u, aperture_radius * v, 0.0)
        };

        // Ray direction
        let dir = Vector::new(
            (x * tfov) - (orig.x() / focus_distance),
            (y * tfov) - (orig.y() / focus_distance),
            1.0,
        )
        .normalized();

        Ray {
            orig: orig * transform,
            dir: dir * transform,
            time: time,
            wavelength: wavelength,
            max_t: std::f32::INFINITY,
        }
    }
}
