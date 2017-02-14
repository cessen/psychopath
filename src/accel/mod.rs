mod bvh;
mod light_array;
mod light_tree;
mod objects_split;

use math::{Vector, Point, Normal};
use shading::surface_closure::SurfaceClosure;

pub use self::bvh::BVH;
pub use self::light_tree::LightTree;


pub trait LightAccel {
    /// Returns (index_of_light, selection_pdf, whittled_n)
    fn select(&self,
              inc: Vector,
              pos: Point,
              nor: Normal,
              sc: &SurfaceClosure,
              time: f32,
              n: f32)
              -> Option<(usize, f32, f32)>;

    fn approximate_energy(&self) -> f32;
}
