use std::collections::HashMap;

use math::Matrix4x4;
use bvh::BVH;
use surface::{Surface, SurfaceIntersection};
use ray::Ray;


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

    // TODO: this is just temporary.  Remove this and move tracing functionality
    // into the tracer.
    pub fn intersect_rays(&self, rays: &mut [Ray], isects: &mut [SurfaceIntersection]) {
        for obj in self.objects.iter() {
            match obj {
                &Object::Surface(ref surface) => {
                    surface.intersect_rays(rays, isects);
                }
            }
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
