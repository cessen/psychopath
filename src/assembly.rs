use std::collections::HashMap;

use math::Matrix4x4;
use bvh::BVH;
use surface::{Surface, SurfaceIntersection};
use ray::Ray;


#[derive(Debug)]
pub struct Assembly {
    // Instance list
    pub instances: Vec<Instance>,
    pub xforms: Vec<Matrix4x4>,

    // Object list
    pub objects: Vec<Object>,
    object_map: HashMap<String, usize>, // map Name -> Index

    // Assembly list
    pub assemblies: Vec<Assembly>,
    assembly_map: HashMap<String, usize>, // map Name -> Index

    // Object accel
    pub object_accel: BVH,
}

impl Assembly {
    pub fn new() -> Assembly {
        Assembly {
            instances: Vec::new(),
            xforms: Vec::new(),
            objects: Vec::new(),
            object_map: HashMap::new(),
            assemblies: Vec::new(),
            assembly_map: HashMap::new(),
            object_accel: BVH::new_empty(),
        }
    }



    pub fn add_object(&mut self, name: &str, obj: Object) {
        self.object_map.insert(name.to_string(), self.objects.len());
        self.objects.push(obj);
    }
}


#[derive(Debug)]
pub enum Object {
    Surface(Box<Surface>),
}


#[derive(Debug, Copy, Clone)]
pub enum Instance {
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
