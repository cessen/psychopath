#![allow(dead_code)]

use mem_arena::MemArena;

use accel::BVH;
use bbox::BBox;
use boundable::Boundable;
use color::XYZ;
use lerp::{lerp, lerp_slice, lerp_slice_with};
use math::{Point, Matrix4x4, cross};
use ray::{Ray, AccelRay};
use shading::surface_closure::{SurfaceClosureUnion, GTRClosure, LambertClosure};

use super::{Surface, SurfaceIntersection, SurfaceIntersectionData};
use super::triangle;


#[derive(Copy, Clone, Debug)]
pub struct TriangleMesh<'a> {
    time_samples: usize,
    geo: &'a [(Point, Point, Point)],
    indices: &'a [usize],
    accel: BVH<'a>,
}

impl<'a> TriangleMesh<'a> {
    pub fn from_triangles<'b>(arena: &'b MemArena,
                              time_samples: usize,
                              triangles: Vec<(Point, Point, Point)>)
                              -> TriangleMesh<'b> {
        assert!(triangles.len() % time_samples == 0);

        let mut indices: Vec<usize> = (0..(triangles.len() / time_samples))
            .map(|n| n * time_samples)
            .collect();

        let bounds = {
            let mut bounds = Vec::new();
            for tri in triangles.iter() {
                let minimum = tri.0.min(tri.1.min(tri.2));
                let maximum = tri.0.max(tri.1.max(tri.2));
                bounds.push(BBox::from_points(minimum, maximum));
            }
            bounds
        };

        let accel = BVH::from_objects(arena,
                                      &mut indices[..],
                                      3,
                                      |tri_i| &bounds[*tri_i..(*tri_i + time_samples)]);

        TriangleMesh {
            time_samples: time_samples,
            geo: arena.copy_slice(&triangles),
            indices: arena.copy_slice(&indices),
            accel: accel,
        }
    }
}

impl<'a> Boundable for TriangleMesh<'a> {
    fn bounds<'b>(&'b self) -> &'b [BBox] {
        self.accel.bounds()
    }
}


impl<'a> Surface for TriangleMesh<'a> {
    fn intersect_rays(&self,
                      accel_rays: &mut [AccelRay],
                      wrays: &[Ray],
                      isects: &mut [SurfaceIntersection],
                      space: &[Matrix4x4]) {
        self.accel.traverse(&mut accel_rays[..], &self.indices, |tri_i, rs| {
            for r in rs {
                let wr = &wrays[r.id as usize];
                let tri =
                    lerp_slice_with(&self.geo[*tri_i..(*tri_i + self.time_samples)],
                                    wr.time,
                                    |a, b, t| {
                                        (lerp(a.0, b.0, t), lerp(a.1, b.1, t), lerp(a.2, b.2, t))
                                    });
                // TODO: when there's no transforms, we don't have to
                // transform the triangles at all.
                let mat_space = if space.len() > 0 {
                    lerp_slice(space, wr.time)
                } else {
                    Matrix4x4::new()
                };
                let mat_inv = mat_space.inverse();
                let tri = (tri.0 * mat_inv, tri.1 * mat_inv, tri.2 * mat_inv);
                if let Some((t, _, _)) = triangle::intersect_ray(wr, tri) {
                    if t < r.max_t {
                        if r.is_occlusion() {
                            isects[r.id as usize] = SurfaceIntersection::Occlude;
                            r.mark_done();
                        } else {
                            isects[r.id as usize] = SurfaceIntersection::Hit {
                                intersection_data: SurfaceIntersectionData {
                                    incoming: wr.dir,
                                    t: t,
                                    pos: wr.orig + (wr.dir * t),
                                    nor: cross(tri.0 - tri.1, tri.0 - tri.2).into_normal(), // TODO
                                    nor_g: cross(tri.0 - tri.1, tri.0 - tri.2).into_normal(),
                                    uv: (0.0, 0.0), // TODO
                                    local_space: mat_space,
                                },
                                // TODO: get surface closure from surface shader.
                                closure: SurfaceClosureUnion::LambertClosure(
                                   LambertClosure::new(XYZ::new(0.8, 0.8, 0.8))
                                ),
                                // closure:
                                //     SurfaceClosureUnion::GTRClosure(
                                //         GTRClosure::new(XYZ::new(0.8, 0.8, 0.8),
                                //                         0.1,
                                //                         2.0,
                                //                         1.0)),
                            };
                            r.max_t = t;
                        }
                    }
                }
            }
        });
    }
}
