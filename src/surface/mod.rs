#![allow(dead_code)]

pub mod triangle;
pub mod triangle_mesh;

use std::fmt::Debug;

use crate::{
    boundable::Boundable,
    math::{Matrix4x4, Normal, Point, Vector},
    ray::{AccelRay, Ray},
    shading::surface_closure::SurfaceClosure,
    shading::SurfaceShader,
};

pub trait Surface: Boundable + Debug + Sync {
    fn intersect_rays(
        &self,
        accel_rays: &mut [AccelRay],
        wrays: &[Ray],
        isects: &mut [SurfaceIntersection],
        shader: &SurfaceShader,
        space: &[Matrix4x4],
    );
}

#[derive(Debug, Copy, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum SurfaceIntersection {
    Miss,
    Occlude,
    Hit {
        intersection_data: SurfaceIntersectionData,
        closure: SurfaceClosure,
    },
}

#[derive(Debug, Copy, Clone)]
pub struct SurfaceIntersectionData {
    pub incoming: Vector, // Direction of the incoming ray
    pub pos: Point,       // Position of the intersection
    pub pos_err: f32,     // Error magnitude of the intersection position.  Imagine
    // a cube centered around `pos` with dimensions of `2 * pos_err`.
    pub nor: Normal,            // Shading normal
    pub nor_g: Normal,          // True geometric normal
    pub local_space: Matrix4x4, // Matrix from global space to local space
    pub t: f32,                 // Ray t-value at the intersection point
    pub uv: (f32, f32),         // 2d surface parameters
    pub sample_pdf: f32,        // The PDF of getting this point by explicitly sampling the surface
}
