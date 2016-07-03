#![allow(dead_code)]

use std::fmt::Debug;

pub mod triangle_mesh;

use ray::{Ray, AccelRay};
use math::{Point, Vector, Normal, Matrix4x4};
use boundable::Boundable;


#[derive(Debug, Copy, Clone)]
pub enum SurfaceIntersection {
    Miss,
    Occlude,
    Hit {
        t: f32,
        pos: Point,
        incoming: Vector,
        nor: Normal,
        local_space: Matrix4x4,
        uv: (f32, f32),
    },
}

pub trait Surface: Boundable + Debug + Sync {
    fn intersect_rays(&self,
                      accel_rays: &mut [AccelRay],
                      wrays: &[Ray],
                      isects: &mut [SurfaceIntersection],
                      space: &[Matrix4x4]);
}
