use std::iter;
use std::slice;
use std::cell::UnsafeCell;

use math::{Matrix4x4, multiply_matrix_slices};
use lerp::lerp_slice;
use assembly::{Assembly, Object, Instance, InstanceType};
use ray::Ray;
use surface::SurfaceIntersection;

pub struct Tracer<'a> {
    root: &'a Assembly,
    rays: UnsafeCell<Vec<Ray>>, // Should only be used from trace(), not any other methods
    xform_stack: Vec<Matrix4x4>,
    xform_stack_indices: Vec<usize>,
    isects: Vec<SurfaceIntersection>,
}

impl<'a> Tracer<'a> {
    pub fn from_assembly(assembly: &'a Assembly) -> Tracer<'a> {
        Tracer {
            root: assembly,
            rays: UnsafeCell::new(Vec::new()),
            xform_stack: Vec::new(),
            xform_stack_indices: vec![0],
            isects: Vec::new(),
        }
    }

    pub fn trace<'b>(&'b mut self, wrays: &[Ray]) -> &'b [SurfaceIntersection] {
        // Ready the rays
        let rays_ptr = self.rays.get();
        unsafe {
            (*rays_ptr).clear();
            (*rays_ptr).reserve(wrays.len());
            (*rays_ptr).extend(wrays.iter());
        }

        // Ready the isects
        self.isects.clear();
        self.isects.reserve(wrays.len());
        self.isects.extend(iter::repeat(SurfaceIntersection::Miss).take(wrays.len()));

        // Start tracing
        let ray_refs = unsafe {
            // IMPORTANT NOTE:
            // We're creating an unsafe non-lifetime-bound slice of self.rays
            // here so that we can pass it to trace_assembly() without
            // conflicting with self.
            // Because of this, it is absolutely CRITICAL that self.rays
            // NOT be used in any other methods.  The rays should only be
            // accessed in other methods via the mutable slice passed directly
            // to them in their function parameters.
            &mut (*rays_ptr)[..]
        };
        self.trace_assembly(self.root, wrays, ray_refs);

        return &self.isects;
    }

    fn trace_assembly<'b>(&'b mut self, assembly: &Assembly, wrays: &[Ray], rays: &mut [Ray]) {
        assembly.object_accel.traverse(&mut rays[..], &assembly.instances[..], |inst, rs| {
            // Transform rays if needed
            if let Some((xstart, xend)) = inst.transform_indices {
                // Push transforms to stack
                let mut combined = Vec::new();
                if self.xform_stack.len() == 0 {
                    self.xform_stack.extend(&assembly.xforms[xstart..xend]);
                } else {
                    let x2start = self.xform_stack_indices[self.xform_stack_indices.len() - 2];
                    let x2end = self.xform_stack_indices[self.xform_stack_indices.len() - 1];
                    multiply_matrix_slices(&self.xform_stack[x2start..x2end],
                                           &assembly.xforms[xstart..xend],
                                           &mut combined);
                    self.xform_stack.extend(&combined);
                }
                self.xform_stack_indices.push(self.xform_stack.len());

                // Do transforms
                let xstart = self.xform_stack_indices[self.xform_stack_indices.len() - 2];
                let xend = self.xform_stack_indices[self.xform_stack_indices.len() - 1];
                let xforms = &self.xform_stack[xstart..xend];
                for ray in &mut rs[..] {
                    let id = ray.id;
                    let t = ray.time;
                    *ray = wrays[id as usize];
                    ray.transform(&lerp_slice(xforms, t));
                }
            }

            // Trace rays
            match inst.instance_type {
                InstanceType::Object => {
                    self.trace_object(&assembly.objects[inst.data_index], wrays, rs);
                }

                InstanceType::Assembly => {
                    self.trace_assembly(&assembly.assemblies[inst.data_index], wrays, rs);
                }
            }

            // Un-transform rays if needed
            if let Some(_) = inst.transform_indices {
                // Pop transforms off stack
                let xstart = self.xform_stack_indices[self.xform_stack_indices.len() - 2];
                let xend = self.xform_stack_indices[self.xform_stack_indices.len() - 1];
                let l = self.xform_stack.len();
                self.xform_stack.resize(l - (xend - xstart), Matrix4x4::new());
                self.xform_stack_indices.pop();

                // Undo transforms
                if self.xform_stack.len() > 0 {
                    let xstart = self.xform_stack_indices[self.xform_stack_indices.len() - 2];
                    let xend = self.xform_stack_indices[self.xform_stack_indices.len() - 1];
                    let xforms = &self.xform_stack[xstart..xend];
                    for ray in &mut rs[..] {
                        let id = ray.id;
                        let t = ray.time;
                        *ray = wrays[id as usize];
                        ray.transform(&lerp_slice(xforms, t));
                    }
                } else {
                    for ray in &mut rs[..] {
                        let id = ray.id;
                        let t = ray.time;
                        *ray = wrays[id as usize];
                    }
                }
            }
        });
    }

    fn trace_object<'b>(&'b mut self, obj: &Object, wrays: &[Ray], rays: &mut [Ray]) {
        match obj {
            &Object::Surface(ref surface) => {
                surface.intersect_rays(rays, &mut self.isects);
            }
        }
    }
}
