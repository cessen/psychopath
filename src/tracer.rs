use std::iter;

use crate::{
    accel::ray_code,
    color::{rec709_to_xyz, Color},
    lerp::lerp_slice,
    math::Matrix4x4,
    ray::{RayBatch, RayStack},
    scene::{Assembly, InstanceType, Object},
    shading::{SimpleSurfaceShader, SurfaceShader},
    surface::SurfaceIntersection,
    transform_stack::TransformStack,
};

pub struct Tracer<'a> {
    ray_stack: RayStack,
    inner: TracerInner<'a>,
}

impl<'a> Tracer<'a> {
    pub fn from_assembly(assembly: &'a Assembly) -> Tracer<'a> {
        Tracer {
            ray_stack: RayStack::new(),
            inner: TracerInner {
                root: assembly,
                xform_stack: TransformStack::new(),
                isects: Vec::new(),
            },
        }
    }

    pub fn trace<'b>(&'b mut self, rays: &mut RayBatch) -> &'b [SurfaceIntersection] {
        self.inner.trace(rays, &mut self.ray_stack)
    }
}

struct TracerInner<'a> {
    root: &'a Assembly<'a>,
    xform_stack: TransformStack,
    isects: Vec<SurfaceIntersection>,
}

impl<'a> TracerInner<'a> {
    fn trace<'b>(
        &'b mut self,
        rays: &mut RayBatch,
        ray_stack: &mut RayStack,
    ) -> &'b [SurfaceIntersection] {
        ray_stack.clear();

        // Ready the isects
        self.isects.clear();
        self.isects.reserve(rays.len());
        self.isects
            .extend(iter::repeat(SurfaceIntersection::Miss).take(rays.len()));

        // Prep the accel part of the rays.
        {
            let ident = Matrix4x4::new();
            for i in 0..rays.len() {
                rays.update_local(i, &ident);
            }
        }

        // Divide the rays into 8 different lanes by direction.
        ray_stack.ensure_lane_count(8);
        for i in 0..rays.len() {
            ray_stack.push_ray_index(i, ray_code(rays.dir(i)));
        }
        ray_stack.push_lanes_to_tasks(&[0, 1, 2, 3, 4, 5, 6, 7]);

        // Trace each of the 8 lanes separately.
        while !ray_stack.is_empty() {
            self.trace_assembly(self.root, rays, ray_stack);
        }

        &self.isects
    }

    fn trace_assembly<'b>(
        &'b mut self,
        assembly: &Assembly,
        rays: &mut RayBatch,
        ray_stack: &mut RayStack,
    ) {
        assembly.object_accel.traverse(
            rays,
            ray_stack,
            &assembly.instances[..],
            |inst, rays, ray_stack| {
                // Transform rays if needed
                if let Some((xstart, xend)) = inst.transform_indices {
                    // Push transforms to stack
                    self.xform_stack.push(&assembly.xforms[xstart..xend]);

                    // Do transforms
                    // TODO: re-divide rays based on direction (maybe?).
                    let xforms = self.xform_stack.top();
                    ray_stack.pop_do_next_task(2, |ray_idx| {
                        let t = rays.time(ray_idx);
                        rays.update_local(ray_idx, &lerp_slice(xforms, t));
                        ([0, 1, 2, 3, 4, 5, 6, 7], 2)
                    });
                    ray_stack.push_lanes_to_tasks(&[0, 1]);
                }

                // Trace rays
                match inst.instance_type {
                    InstanceType::Object => {
                        self.trace_object(
                            &assembly.objects[inst.data_index],
                            inst.surface_shader_index
                                .map(|i| assembly.surface_shaders[i]),
                            rays,
                            ray_stack,
                        );
                    }

                    InstanceType::Assembly => {
                        self.trace_assembly(&assembly.assemblies[inst.data_index], rays, ray_stack);
                    }
                }

                // Un-transform rays if needed
                if inst.transform_indices.is_some() {
                    // Pop transforms off stack
                    self.xform_stack.pop();

                    // Undo transforms
                    let xforms = self.xform_stack.top();
                    if !xforms.is_empty() {
                        ray_stack.pop_do_next_task(0, |ray_idx| {
                            let t = rays.time(ray_idx);
                            rays.update_local(ray_idx, &lerp_slice(xforms, t));
                            ([0, 1, 2, 3, 4, 5, 6, 7], 0)
                        });
                    } else {
                        let ident = Matrix4x4::new();
                        ray_stack.pop_do_next_task(0, |ray_idx| {
                            rays.update_local(ray_idx, &ident);
                            ([0, 1, 2, 3, 4, 5, 6, 7], 0)
                        });
                    }
                }
            },
        );
    }

    fn trace_object<'b>(
        &'b mut self,
        obj: &Object,
        surface_shader: Option<&SurfaceShader>,
        rays: &mut RayBatch,
        ray_stack: &mut RayStack,
    ) {
        match *obj {
            Object::Surface(surface) => {
                let unassigned_shader = SimpleSurfaceShader::Emit {
                    color: Color::new_xyz(rec709_to_xyz((1.0, 0.0, 1.0))),
                };
                let shader = surface_shader.unwrap_or(&unassigned_shader);

                surface.intersect_rays(
                    rays,
                    ray_stack,
                    &mut self.isects,
                    shader,
                    self.xform_stack.top(),
                );
            }

            Object::SurfaceLight(surface) => {
                // Lights don't use shaders
                let bogus_shader = SimpleSurfaceShader::Emit {
                    color: Color::new_xyz(rec709_to_xyz((1.0, 0.0, 1.0))),
                };

                surface.intersect_rays(
                    rays,
                    ray_stack,
                    &mut self.isects,
                    &bogus_shader,
                    self.xform_stack.top(),
                );
            }
        }
    }
}
