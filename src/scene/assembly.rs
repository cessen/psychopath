use std::{collections::HashMap, ops::Range};

use crate::{light::SurfaceLight, math::Matrix4x4, surface::Surface};

/// Stores the objects of a scene and its acceleration structures.
#[derive(Debug)]
pub struct Assembly<'a> {
    pub objects: HashMap<String, Object<'a>>, // Name, Object.
    pub xforms: Vec<Matrix4x4>,
}

impl<'a> Assembly<'a> {
    pub fn new() -> Assembly<'a> {
        Assembly {
            objects: HashMap::new(),
            xforms: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Object<'a> {
    pub data: ObjectData<'a>,

    // One range per instance, indexing into the assembly's xforms
    // array.  An empty Vec means a single instance with no transforms.
    pub instance_xform_idxs: Vec<Range<usize>>,
}

#[derive(Debug)]
pub enum ObjectData<'a> {
    Empty,
    Surface(Box<dyn Surface + 'a>),
    Light(Box<dyn SurfaceLight + 'a>),
}
