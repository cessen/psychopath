#![allow(dead_code)]

use std::fmt::Debug;

pub mod triangle_mesh;

use ray::Ray;
use math::{Point, Normal, Matrix4x4};


#[derive(Debug)]
pub enum SurfaceIntersection {
    Miss,
    Occlude,
    Hit {
        t: f32,
        pos: Point,
        nor: Normal,
        space: Matrix4x4,
        uv: (f32, f32),
    },
}

pub trait Surface: Debug {
    fn intersect_rays(&self, rays: &mut [Ray], isects: &mut [SurfaceIntersection]);
}
