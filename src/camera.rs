#![allow(dead_code)]

use std::f32::consts::FRAC_PI_4;

use math::{Vector, Point, Matrix4x4};
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
            let (u, v) = square_to_circle(aperture_radius * ((u * 2.0) - 1.0),
                                          aperture_radius * ((v * 2.0) - 1.0));
            Point::new(u, v, 0.0)
        };

        // Ray direction
        let dir = Vector::new((x * tfov) - (orig[0] / focus_distance),
                              (y * tfov) - (orig[1] / focus_distance),
                              1.0)
                      .normalized();

        Ray::new(orig * transform, dir * transform, time)
    }
}


/// Maps the unit square to the unit circle.
/// NOTE: x and y should be distributed within [-1, 1],
/// not [0, 1].
fn square_to_circle(x: f32, y: f32) -> (f32, f32) {
    debug_assert!(x >= -1.0 && x <= 1.0);
    debug_assert!(y >= -1.0 && y <= 1.0);

    if x == 0.0 && y == 0.0 {
        return (0.0, 0.0);
    }

    let (radius, angle) = {
        if x > y.abs() {
            // Quadrant 1
            (x, (y / x) * FRAC_PI_4)
        } else if y > x.abs() {
            // Quadrant 2
            (y, (2.0 - (x / y)) * FRAC_PI_4)
        } else if x < -(y.abs()) {
            // Quadrant 3
            (-x, (4.0 + (y / x)) * FRAC_PI_4)
        } else {
            // Quadrant 4
            (-y, (6.0 - (x / y)) * FRAC_PI_4)
        }
    };

    return (radius * angle.cos(), radius * angle.sin());
}
