#![allow(dead_code)]

pub mod triangle_mesh;

use std::fmt::Debug;

use boundable::Boundable;
use math::{Point, Vector, Normal, Matrix4x4};
use ray::{Ray, AccelRay};
use shading::surface_closure::SurfaceClosureUnion;


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
        closure: SurfaceClosureUnion,
    },
}

pub trait Surface: Boundable + Debug + Sync {
    fn intersect_rays(&self,
                      accel_rays: &mut [AccelRay],
                      wrays: &[Ray],
                      isects: &mut [SurfaceIntersection],
                      space: &[Matrix4x4]);
}
