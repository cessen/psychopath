use std::collections::HashMap;

use math::Matrix4x4;
use bvh::BVH;
use boundable::Boundable;
use surface::Surface;
use light::LightSource;
use bbox::{BBox, transform_bbox_slice_from};


#[derive(Debug)]
pub struct Assembly {
    // Instance list
    pub instances: Vec<Instance>,
    pub xforms: Vec<Matrix4x4>,

    // Object list
    pub objects: Vec<Object>,

    // Assembly list
    pub assemblies: Vec<Assembly>,

    // Object accel
    pub object_accel: BVH,
}

impl Boundable for Assembly {
    fn bounds<'a>(&'a self) -> &'a [BBox] {
        self.object_accel.bounds()
    }
}


#[derive(Debug)]
pub struct AssemblyBuilder {
    // Instance list
    instances: Vec<Instance>,
    xforms: Vec<Matrix4x4>,

    // Object list
    objects: Vec<Object>,
    object_map: HashMap<String, usize>, // map Name -> Index

    // Assembly list
    assemblies: Vec<Assembly>,
    assembly_map: HashMap<String, usize>, // map Name -> Index
}


impl AssemblyBuilder {
    pub fn new() -> AssemblyBuilder {
        AssemblyBuilder {
            instances: Vec::new(),
            xforms: Vec::new(),
            objects: Vec::new(),
            object_map: HashMap::new(),
            assemblies: Vec::new(),
            assembly_map: HashMap::new(),
        }
    }

    pub fn add_object(&mut self, name: &str, obj: Object) {
        // Make sure the name hasn't already been used.
        if self.name_exists(name) {
            panic!("Attempted to add object to assembly with a name that already exists.");
        }

        // Add object
        self.object_map.insert(name.to_string(), self.objects.len());
        self.objects.push(obj);
    }

    pub fn add_assembly(&mut self, name: &str, asmb: Assembly) {
        // Make sure the name hasn't already been used.
        if self.name_exists(name) {
            panic!("Attempted to add assembly to another assembly with a name that already \
                    exists.");
        }

        // Add assembly
        self.assembly_map.insert(name.to_string(), self.assemblies.len());
        self.assemblies.push(asmb);
    }

    pub fn add_instance(&mut self, name: &str, xforms: Option<&[Matrix4x4]>) {
        // Make sure name exists
        if !self.name_exists(name) {
            panic!("Attempted to add instance with a name that doesn't exist.");
        }

        // Create instance
        let instance = if self.object_map.contains_key(name) {
            Instance {
                instance_type: InstanceType::Object,
                data_index: self.object_map[name],
                id: self.instances.len(),
                transform_indices:
                    xforms.map(|xf| (self.xforms.len(), self.xforms.len() + xf.len())),
            }
        } else {
            Instance {
                instance_type: InstanceType::Assembly,
                data_index: self.assembly_map[name],
                id: self.instances.len(),
                transform_indices:
                    xforms.map(|xf| (self.xforms.len(), self.xforms.len() + xf.len())),
            }
        };

        self.instances.push(instance);

        // Store transforms
        if let Some(xf) = xforms {
            self.xforms.extend(xf);
        }
    }

    pub fn name_exists(&self, name: &str) -> bool {
        self.object_map.contains_key(name) || self.assembly_map.contains_key(name)
    }

    pub fn build(mut self) -> Assembly {
        // Shrink storage to minimum.
        self.instances.shrink_to_fit();
        self.xforms.shrink_to_fit();
        self.objects.shrink_to_fit();
        self.assemblies.shrink_to_fit();

        // Build object accel
        let (bis, bbs) = self.instance_bounds();
        let object_accel = BVH::from_objects(&mut self.instances[..],
                                             1,
                                             |inst| &bbs[bis[inst.id]..bis[inst.id + 1]]);

        println!("Assembly BVH Depth: {}", object_accel.tree_depth());

        Assembly {
            instances: self.instances,
            xforms: self.xforms,
            objects: self.objects,
            assemblies: self.assemblies,
            object_accel: object_accel,
        }
    }


    /// Returns a pair of vectors with the bounds of all instances.
    /// This is used for building the assembly's BVH.
    fn instance_bounds(&self) -> (Vec<usize>, Vec<BBox>) {
        let mut indices = vec![0];
        let mut bounds = Vec::new();

        for inst in self.instances.iter() {
            let mut bbs = Vec::new();
            let mut bbs2 = Vec::new();

            // Get bounding boxes
            match inst.instance_type {
                InstanceType::Object => {
                    // Push bounds onto bbs
                    let obj = &self.objects[inst.data_index];
                    match obj {
                        &Object::Surface(ref s) => bbs.extend(s.bounds()),
                        &Object::Light(ref l) => bbs.extend(l.bounds()),
                    }
                }

                InstanceType::Assembly => {
                    // Push bounds onto bbs
                    let asmb = &self.assemblies[inst.data_index];
                    bbs.extend(asmb.bounds());
                }
            }

            // Transform the bounding boxes, if necessary
            if let Some((xstart, xend)) = inst.transform_indices {
                let xf = &self.xforms[xstart..xend];
                transform_bbox_slice_from(&bbs, &xf, &mut bbs2);
            } else {
                bbs2.clear();
                bbs2.extend(bbs);
            }

            // Push transformed bounds onto vec
            bounds.extend(bbs2);
            indices.push(bounds.len());
        }

        return (indices, bounds);
    }
}



#[derive(Debug)]
pub enum Object {
    Surface(Box<Surface>),
    Light(Box<LightSource>),
}


#[derive(Debug, Copy, Clone)]
pub struct Instance {
    pub instance_type: InstanceType,
    pub data_index: usize,
    pub id: usize,
    pub transform_indices: Option<(usize, usize)>,
}

#[derive(Debug, Copy, Clone)]
pub enum InstanceType {
    Object,
    Assembly,
}
