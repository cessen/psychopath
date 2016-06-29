#![allow(dead_code)]

use math::{Vector, Point, Matrix4x4};
use sampling::square_to_circle;
use ray::Ray;
use lerp::lerp_slice;

#[derive(Debug)]
pub struct Camera {
    transforms: Vec<Matrix4x4>,
    fovs: Vec<f32>,
    tfovs: Vec<f32>,
    aperture_radii: Vec<f32>,
    focus_distances: Vec<f32>,
}

impl Camera {
    pub fn new(transforms: Vec<Matrix4x4>,
               fovs: Vec<f32>,
               mut aperture_radii: Vec<f32>,
               mut focus_distances: Vec<f32>)
               -> Camera {
        assert!(transforms.len() != 0, "Camera has no transform(s)!");
        assert!(fovs.len() != 0, "Camera has no fov(s)!");

        // Aperture needs focus distance and vice-versa.
        if aperture_radii.len() == 0 || focus_distances.len() == 0 {
            aperture_radii = vec![0.0];
            focus_distances = vec![1.0];

            if aperture_radii.len() == 0 && focus_distances.len() != 0 {
                println!("WARNING: camera has aperture radius but no focus distance.  Disabling \
                          focal blur.");
            } else if aperture_radii.len() != 0 && focus_distances.len() == 0 {
                println!("WARNING: camera has focus distance but no aperture radius.  Disabling \
                          focal blur.");
            }
        }

        // Can't have focus distance of zero.
        if focus_distances.iter().any(|d| *d == 0.0) {
            println!("WARNING: camera focal distance is zero or less.  Disabling focal blur.");
            aperture_radii = vec![0.0];
            focus_distances = vec![1.0];
        }

        // Convert angle fov into linear fov.
        let tfovs = fovs.iter().map(|n| (n / 2.0).sin() / (n / 2.0).cos()).collect();

        Camera {
            transforms: transforms,
            fovs: fovs,
            tfovs: tfovs,
            aperture_radii: aperture_radii,
            focus_distances: focus_distances,
        }
    }

    pub fn generate_ray(&self, x: f32, y: f32, time: f32, u: f32, v: f32) -> Ray {
        // Get time-interpolated camera settings
        let transform = lerp_slice(&self.transforms, time);
        let tfov = lerp_slice(&self.tfovs, time);
        let aperture_radius = lerp_slice(&self.aperture_radii, time);
        let focus_distance = lerp_slice(&self.focus_distances, time);

        // Ray origin
        let orig = {
            let (u, v) = square_to_circle((u * 2.0) - 1.0, (v * 2.0) - 1.0);
            Point::new(aperture_radius * u, aperture_radius * v, 0.0)
        };

        // Ray direction
        let dir = Vector::new((x * tfov) - (orig[0] / focus_distance),
                              (y * tfov) - (orig[1] / focus_distance),
                              1.0)
            .normalized();

        Ray::new(orig * transform, dir * transform, time, false)
    }
}
