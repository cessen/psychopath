use std::collections::HashMap;

use accel::{LightAccel, LightTree};
use accel::BVH;
use bbox::{BBox, transform_bbox_slice_from};
use boundable::Boundable;
use color::SpectralSample;
use lerp::lerp_slice;
use light::LightSource;
use math::{Matrix4x4, Vector};
use surface::{Surface, SurfaceIntersection};
use transform_stack::TransformStack;


#[derive(Debug)]
pub struct Assembly {
    // Instance list
    pub instances: Vec<Instance>,
    pub light_instances: Vec<Instance>,
    pub xforms: Vec<Matrix4x4>,

    // Object list
    pub objects: Vec<Object>,

    // Assembly list
    pub assemblies: Vec<Assembly>,

    // Object accel
    pub object_accel: BVH,

    // Light accel
    pub light_accel: LightTree,
}

impl Assembly {
    // Returns (light_color, shadow_vector, pdf, selection_pdf)
    pub fn sample_lights(&self,
                         xform_stack: &mut TransformStack,
                         n: f32,
                         uvw: (f32, f32, f32),
                         wavelength: f32,
                         time: f32,
                         intr: &SurfaceIntersection)
                         -> Option<(SpectralSample, Vector, f32, f32)> {
        if let &SurfaceIntersection::Hit { pos, incoming, nor, closure, .. } = intr {
            let sel_xform = if xform_stack.top().len() > 0 {
                lerp_slice(xform_stack.top(), time)
            } else {
                Matrix4x4::new()
            };
            if let Some((light_i, sel_pdf, whittled_n)) =
                self.light_accel
                    .select(incoming * sel_xform,
                            pos * sel_xform,
                            nor * sel_xform,
                            closure.as_surface_closure(),
                            time,
                            n) {
                let inst = self.light_instances[light_i];
                match inst.instance_type {

                    InstanceType::Object => {
                        match &self.objects[inst.data_index] {
                            &Object::Light(ref light) => {
                                // Get the world-to-object space transform of the light
                                let xform = if let Some((a, b)) = inst.transform_indices {
                                    let pxforms = xform_stack.top();
                                    let xform = lerp_slice(&self.xforms[a..b], time);
                                    if pxforms.len() > 0 {
                                        lerp_slice(pxforms, time) * xform
                                    } else {
                                        xform
                                    }
                                } else {
                                    let pxforms = xform_stack.top();
                                    if pxforms.len() > 0 {
                                        lerp_slice(pxforms, time)
                                    } else {
                                        Matrix4x4::new()
                                    }
                                };

                                // Sample the light
                                let (color, shadow_vec, pdf) =
                                    light.sample(&xform, pos, uvw.0, uvw.1, wavelength, time);
                                return Some((color, shadow_vec, pdf, sel_pdf));
                            }

                            _ => unimplemented!(),
                        }
                    }

                    InstanceType::Assembly => {
                        // Push the world-to-object space transforms of the assembly onto
                        // the transform stack.
                        if let Some((a, b)) = inst.transform_indices {
                            xform_stack.push(&self.xforms[a..b]);
                        }

                        // Sample sub-assembly lights
                        let sample = self.assemblies[inst.data_index]
                            .sample_lights(xform_stack, whittled_n, uvw, wavelength, time, intr);

                        // Pop the assembly's transforms off the transform stack.
                        if let Some(_) = inst.transform_indices {
                            xform_stack.pop();
                        }

                        // Return sample
                        return sample.map(|(ss, v, pdf, spdf)| (ss, v, pdf, spdf * sel_pdf));
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }
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

        // Map zero-length transforms to None
        let xforms = if let Some(xf) = xforms {
            if xf.len() > 0 { Some(xf) } else { None }
        } else {
            None
        };

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

        // Calculate instance bounds, used for building object accel and light accel.
        let (bis, bbs) = self.instance_bounds();

        // Build object accel
        let object_accel = BVH::from_objects(&mut self.instances[..],
                                             1,
                                             |inst| &bbs[bis[inst.id]..bis[inst.id + 1]]);

        // Get list of instances that are for light sources or assemblies that contain light
        // sources.
        let mut light_instances: Vec<_> = self.instances
            .iter()
            .filter(|inst| match inst.instance_type {
                InstanceType::Object => {
                    if let Object::Light(_) = self.objects[inst.data_index] {
                        true
                    } else {
                        false
                    }
                }

                InstanceType::Assembly => {
                    self.assemblies[inst.data_index].light_accel.approximate_energy() > 0.0
                }
            })
            .map(|&a| a)
            .collect();

        // Build light accel
        let light_accel = LightTree::from_objects(&mut light_instances[..], |inst| {
            let bounds = &bbs[bis[inst.id]..bis[inst.id + 1]];
            let energy = match inst.instance_type {
                InstanceType::Object => {
                    if let Object::Light(ref light) = self.objects[inst.data_index] {
                        light.approximate_energy()
                    } else {
                        0.0
                    }
                }

                InstanceType::Assembly => {
                    self.assemblies[inst.data_index].light_accel.approximate_energy()
                }
            };
            (bounds, energy)
        });

        Assembly {
            instances: self.instances,
            light_instances: light_instances,
            xforms: self.xforms,
            objects: self.objects,
            assemblies: self.assemblies,
            object_accel: object_accel,
            light_accel: light_accel,
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
