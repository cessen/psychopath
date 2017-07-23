use std::collections::HashMap;

use mem_arena::MemArena;

use accel::{LightAccel, LightTree};
use accel::BVH4;
use bbox::{BBox, transform_bbox_slice_from};
use boundable::Boundable;
use color::SpectralSample;
use lerp::lerp_slice;
use light::LightSource;
use math::{Matrix4x4, Vector};
use surface::{Surface, SurfaceIntersection};
use transform_stack::TransformStack;


#[derive(Copy, Clone, Debug)]
pub struct Assembly<'a> {
    // Instance list
    pub instances: &'a [Instance],
    pub light_instances: &'a [Instance],
    pub xforms: &'a [Matrix4x4],

    // Object list
    pub objects: &'a [Object<'a>],

    // Assembly list
    pub assemblies: &'a [Assembly<'a>],

    // Object accel
    pub object_accel: BVH4<'a>,

    // Light accel
    pub light_accel: LightTree<'a>,
}

impl<'a> Assembly<'a> {
    // Returns (light_color, shadow_vector, pdf, selection_pdf)
    pub fn sample_lights(
        &self,
        xform_stack: &mut TransformStack,
        n: f32,
        uvw: (f32, f32, f32),
        wavelength: f32,
        time: f32,
        intr: &SurfaceIntersection,
    ) -> Option<(SpectralSample, Vector, f32, f32)> {
        if let SurfaceIntersection::Hit {
            intersection_data: idata,
            closure,
        } = *intr
        {
            let sel_xform = if !xform_stack.top().is_empty() {
                lerp_slice(xform_stack.top(), time)
            } else {
                Matrix4x4::new()
            };
            if let Some((light_i, sel_pdf, whittled_n)) =
                self.light_accel.select(
                    idata.incoming * sel_xform,
                    idata.pos * sel_xform,
                    idata.nor * sel_xform,
                    closure.as_surface_closure(),
                    time,
                    n,
                )
            {
                let inst = self.light_instances[light_i];
                match inst.instance_type {

                    InstanceType::Object => {
                        match self.objects[inst.data_index] {
                            Object::Light(light) => {
                                // Get the world-to-object space transform of the light
                                let xform = if let Some((a, b)) = inst.transform_indices {
                                    let pxforms = xform_stack.top();
                                    let xform = lerp_slice(&self.xforms[a..b], time);
                                    if !pxforms.is_empty() {
                                        lerp_slice(pxforms, time) * xform
                                    } else {
                                        xform
                                    }
                                } else {
                                    let pxforms = xform_stack.top();
                                    if !pxforms.is_empty() {
                                        lerp_slice(pxforms, time)
                                    } else {
                                        Matrix4x4::new()
                                    }
                                };

                                // Sample the light
                                let (color, shadow_vec, pdf) =
                                    light.sample(&xform, idata.pos, uvw.0, uvw.1, wavelength, time);
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
                        let sample = self.assemblies[inst.data_index].sample_lights(
                            xform_stack,
                            whittled_n,
                            uvw,
                            wavelength,
                            time,
                            intr,
                        );

                        // Pop the assembly's transforms off the transform stack.
                        if inst.transform_indices.is_some() {
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

impl<'a> Boundable for Assembly<'a> {
    fn bounds(&self) -> &[BBox] {
        self.object_accel.bounds()
    }
}


#[derive(Debug)]
pub struct AssemblyBuilder<'a> {
    arena: &'a MemArena,

    // Instance list
    instances: Vec<Instance>,
    xforms: Vec<Matrix4x4>,

    // Object list
    objects: Vec<Object<'a>>,
    object_map: HashMap<String, usize>, // map Name -> Index

    // Assembly list
    assemblies: Vec<Assembly<'a>>,
    assembly_map: HashMap<String, usize>, // map Name -> Index
}


impl<'a> AssemblyBuilder<'a> {
    pub fn new(arena: &'a MemArena) -> AssemblyBuilder<'a> {
        AssemblyBuilder {
            arena: arena,
            instances: Vec::new(),
            xforms: Vec::new(),
            objects: Vec::new(),
            object_map: HashMap::new(),
            assemblies: Vec::new(),
            assembly_map: HashMap::new(),
        }
    }

    pub fn add_object(&mut self, name: &str, obj: Object<'a>) {
        // Make sure the name hasn't already been used.
        if self.name_exists(name) {
            panic!("Attempted to add object to assembly with a name that already exists.");
        }

        // Add object
        self.object_map.insert(name.to_string(), self.objects.len());
        self.objects.push(obj);
    }

    pub fn add_assembly(&mut self, name: &str, asmb: Assembly<'a>) {
        // Make sure the name hasn't already been used.
        if self.name_exists(name) {
            panic!(
                "Attempted to add assembly to another assembly with a name that already \
                    exists."
            );
        }

        // Add assembly
        self.assembly_map.insert(
            name.to_string(),
            self.assemblies.len(),
        );
        self.assemblies.push(asmb);
    }

    pub fn add_instance(&mut self, name: &str, xforms: Option<&[Matrix4x4]>) {
        // Make sure name exists
        if !self.name_exists(name) {
            panic!("Attempted to add instance with a name that doesn't exist.");
        }

        // Map zero-length transforms to None
        let xforms = if let Some(xf) = xforms {
            if !xf.is_empty() { Some(xf) } else { None }
        } else {
            None
        };

        // Create instance
        let instance = if self.object_map.contains_key(name) {
            Instance {
                instance_type: InstanceType::Object,
                data_index: self.object_map[name],
                id: self.instances.len(),
                transform_indices: xforms.map(
                    |xf| (self.xforms.len(), self.xforms.len() + xf.len()),
                ),
            }
        } else {
            Instance {
                instance_type: InstanceType::Assembly,
                data_index: self.assembly_map[name],
                id: self.instances.len(),
                transform_indices: xforms.map(
                    |xf| (self.xforms.len(), self.xforms.len() + xf.len()),
                ),
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

    pub fn build(mut self) -> Assembly<'a> {
        // Calculate instance bounds, used for building object accel and light accel.
        let (bis, bbs) = self.instance_bounds();

        // Build object accel
        let object_accel = BVH4::from_objects(self.arena, &mut self.instances[..], 1, |inst| {
            &bbs[bis[inst.id]..bis[inst.id + 1]]
        });

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
                    self.assemblies[inst.data_index]
                        .light_accel
                        .approximate_energy() > 0.0
                }
            })
            .cloned()
            .collect();

        // Build light accel
        let light_accel = LightTree::from_objects(self.arena, &mut light_instances[..], |inst| {
            let bounds = &bbs[bis[inst.id]..bis[inst.id + 1]];
            let energy = match inst.instance_type {
                InstanceType::Object => {
                    if let Object::Light(light) = self.objects[inst.data_index] {
                        light.approximate_energy()
                    } else {
                        0.0
                    }
                }

                InstanceType::Assembly => {
                    self.assemblies[inst.data_index]
                        .light_accel
                        .approximate_energy()
                }
            };
            (bounds, energy)
        });

        Assembly {
            instances: self.arena.copy_slice(&self.instances),
            light_instances: self.arena.copy_slice(&light_instances),
            xforms: self.arena.copy_slice(&self.xforms),
            objects: self.arena.copy_slice(&self.objects),
            assemblies: self.arena.copy_slice(&self.assemblies),
            object_accel: object_accel,
            light_accel: light_accel,
        }
    }


    /// Returns a pair of vectors with the bounds of all instances.
    /// This is used for building the assembly's BVH4.
    fn instance_bounds(&self) -> (Vec<usize>, Vec<BBox>) {
        let mut indices = vec![0];
        let mut bounds = Vec::new();

        for inst in &self.instances {
            let mut bbs = Vec::new();
            let mut bbs2 = Vec::new();

            // Get bounding boxes
            match inst.instance_type {
                InstanceType::Object => {
                    // Push bounds onto bbs
                    let obj = &self.objects[inst.data_index];
                    match *obj {
                        Object::Surface(s) => bbs.extend(s.bounds()),
                        Object::Light(l) => bbs.extend(l.bounds()),
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
                transform_bbox_slice_from(&bbs, xf, &mut bbs2);
            } else {
                bbs2.clear();
                bbs2.extend(bbs);
            }

            // Push transformed bounds onto vec
            bounds.extend(bbs2);
            indices.push(bounds.len());
        }

        (indices, bounds)
    }
}



#[derive(Copy, Clone, Debug)]
pub enum Object<'a> {
    Surface(&'a Surface),
    Light(&'a LightSource),
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
