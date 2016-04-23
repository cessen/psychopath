use std::iter;
use std::slice;
use std::cell::UnsafeCell;

use math::Matrix4x4;
use assembly::{Assembly, Object, Instance, InstanceType};
use ray::Ray;
use surface::SurfaceIntersection;

pub struct Tracer<'a> {
    root: &'a Assembly,
    rays: UnsafeCell<Vec<Ray>>, // Should only be used from trace(), not any other methods
    xform_stack: Vec<Matrix4x4>,
    isects: Vec<SurfaceIntersection>,
}

impl<'a> Tracer<'a> {
    pub fn from_assembly(assembly: &'a Assembly) -> Tracer<'a> {
        Tracer {
            root: assembly,
            xform_stack: Vec::new(),
            rays: UnsafeCell::new(Vec::new()),
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
            // TODO: transform rays
            match inst.instance_type {
                InstanceType::Object => {
                    self.trace_object(&assembly.objects[inst.data_index], wrays, rs);
                }

                InstanceType::Assembly => {
                    self.trace_assembly(&assembly.assemblies[inst.data_index], wrays, rs);
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
