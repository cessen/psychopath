use std::collections::HashMap;

use math::Matrix4x4;
use bvh::BVH;
use surface::Surface;


#[derive(Debug)]
pub struct Assembly {
    // Instance list
    instances: Vec<Instance>,
    xforms: Vec<Matrix4x4>,

    // Object list
    objects: Vec<Object>,
    object_map: HashMap<String, usize>, // map Name -> Index

    // Assembly list
    assemblies: Vec<Assembly>,
    assembly_map: HashMap<String, usize>, // map Name -> Index

    // Object accel
    object_accel: BVH,
}


#[derive(Debug)]
pub enum Object {
    Surface(Box<Surface>),
}


#[derive(Debug, Copy, Clone)]
enum Instance {
    Object {
        data_index: usize,
        transform_indices: (usize, usize),
        shader_index: usize,
    },

    Assembly {
        data_index: usize,
        transform_indices: (usize, usize),
    },
}
