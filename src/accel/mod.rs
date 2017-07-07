mod bvh_base;
mod bvh;
mod bvh4;
mod light_array;
mod light_tree;
mod objects_split;

use std::cell::Cell;

use math::{Vector, Point, Normal};
use shading::surface_closure::SurfaceClosure;

pub use self::bvh::{BVH, BVHNode};
pub use self::bvh4::{BVH4, BVH4Node};
pub use self::light_tree::LightTree;
pub use self::light_array::LightArray;

// Track BVH traversal time
thread_local! {
    pub static ACCEL_TRAV_TIME: Cell<f64> = Cell::new(0.0);
    pub static ACCEL_NODE_RAY_TESTS: Cell<u64> = Cell::new(0);
}

pub trait LightAccel {
    /// Returns (index_of_light, selection_pdf, whittled_n)
    fn select(
        &self,
        inc: Vector,
        pos: Point,
        nor: Normal,
        sc: &SurfaceClosure,
        time: f32,
        n: f32,
    ) -> Option<(usize, f32, f32)>;

    fn approximate_energy(&self) -> f32;
}
