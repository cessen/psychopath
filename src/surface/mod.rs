#![allow(dead_code)]

// pub mod micropoly_batch;
pub mod bilinear_patch;
pub mod micropoly_batch;
pub mod triangle;
pub mod triangle_mesh;

use std::fmt::Debug;

use crate::{
    boundable::Boundable,
    math::{Normal, Point, Transform, Vector},
    ray::{RayBatch, RayStack},
    shading::surface_closure::SurfaceClosure,
    shading::SurfaceShader,
};

const MAX_EDGE_DICE: u32 = 128;

pub trait Surface: Boundable + Debug + Sync {
    fn intersect_rays(
        &self,
        rays: &mut RayBatch,
        ray_stack: &mut RayStack,
        isects: &mut [SurfaceIntersection],
        shader: &dyn SurfaceShader,
        space: &[Transform],
    );
}

pub trait Splitable: Copy {
    /// Splits the surface into two pieces if necessary.
    fn split<F>(&self, metric: F) -> Option<(Self, Self)>
    where
        F: Fn(Point, Point) -> f32;
}

#[derive(Debug, Copy, Clone)]
pub enum PointOrder {
    AsIs,
    Flip,
}

pub fn point_order(p1: Point, p2: Point) -> PointOrder {
    let max_diff = {
        let v = p2 - p1;
        let v_abs = v.abs();

        let mut diff = v.x();
        let mut diff_abs = v_abs.x();
        if v_abs.y() > diff_abs {
            diff = v.y();
            diff_abs = v_abs.y();
        }
        if v_abs.z() > diff_abs {
            diff = v.z();
        }

        diff
    };

    if max_diff <= 0.0 {
        PointOrder::AsIs
    } else {
        PointOrder::Flip
    }
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
    pub local_space: Transform, // Matrix from global space to local space
    pub t: f32,                 // Ray t-value at the intersection point
    pub sample_pdf: f32,        // The PDF of getting this point by explicitly sampling the surface
}
