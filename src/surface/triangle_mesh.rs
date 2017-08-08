#![allow(dead_code)]

use mem_arena::MemArena;

use accel::BVH4;
use bbox::BBox;
use boundable::Boundable;
use fp_utils::fp_gamma;
use lerp::lerp_slice;
use math::{Point, Normal, Matrix4x4, dot, cross};
use ray::{Ray, AccelRay};
use shading::SurfaceShader;

use super::{Surface, SurfaceIntersection, SurfaceIntersectionData};
use super::triangle;


#[derive(Copy, Clone, Debug)]
pub struct TriangleMesh<'a> {
    time_sample_count: usize,
    vertices: &'a [Point], // Vertices, with the time samples for each vertex stored contiguously
    normals: Option<&'a [Normal]>, // Vertex normals, organized the same as `vertices`
    indices: &'a [(u32, u32, u32, u32)], // (v0_idx, v1_idx, v2_idx, original_tri_idx)
    accel: BVH4<'a>,
}

impl<'a> TriangleMesh<'a> {
    pub fn from_verts_and_indices<'b>(
        arena: &'b MemArena,
        verts: Vec<Vec<Point>>,
        vert_normals: Option<Vec<Vec<Normal>>>,
        tri_indices: Vec<(usize, usize, usize)>,
    ) -> TriangleMesh<'b> {
        let vert_count = verts[0].len();
        let time_sample_count = verts.len();

        // Copy verts over to a contiguous area of memory, reorganizing them
        // so that each vertices' time samples are contiguous in memory.
        let vertices = {
            let mut vertices =
                unsafe { arena.alloc_array_uninitialized(vert_count * time_sample_count) };

            for vi in 0..vert_count {
                for ti in 0..time_sample_count {
                    vertices[(vi * time_sample_count) + ti] = verts[ti][vi];
                }
            }

            vertices
        };

        // Copy vertex normals, if any, organizing them the same as vertices
        // above.
        let normals = match vert_normals {
            Some(ref vnors) => {
                let mut normals =
                    unsafe { arena.alloc_array_uninitialized(vert_count * time_sample_count) };

                for vi in 0..vert_count {
                    for ti in 0..time_sample_count {
                        normals[(vi * time_sample_count) + ti] = vnors[ti][vi];
                    }
                }

                Some(&normals[..])
            }

            None => None,
        };

        // Copy triangle vertex indices over, appending the triangle index itself to the tuple
        let mut indices = {
            let mut indices = unsafe { arena.alloc_array_uninitialized(tri_indices.len()) };
            for (i, tri_i) in tri_indices.iter().enumerate() {
                indices[i] = (tri_i.0 as u32, tri_i.2 as u32, tri_i.1 as u32, i as u32);
            }
            indices
        };

        // Create bounds array for use during BVH construction
        let bounds = {
            let mut bounds = Vec::with_capacity(indices.len() * time_sample_count);
            for tri in &tri_indices {
                for ti in 0..time_sample_count {
                    let p0 = verts[ti][tri.0];
                    let p1 = verts[ti][tri.1];
                    let p2 = verts[ti][tri.2];
                    let minimum = p0.min(p1.min(p2));
                    let maximum = p0.max(p1.max(p2));
                    bounds.push(BBox::from_points(minimum, maximum));
                }
            }
            bounds
        };

        // Build BVH
        let accel = BVH4::from_objects(arena, &mut indices[..], 3, |tri| {
            &bounds[(tri.3 as usize * time_sample_count)..
                        ((tri.3 as usize + 1) * time_sample_count)]
        });

        TriangleMesh {
            time_sample_count: time_sample_count,
            vertices: vertices,
            normals: normals,
            indices: indices,
            accel: accel,
        }
    }
}

impl<'a> Boundable for TriangleMesh<'a> {
    fn bounds(&self) -> &[BBox] {
        self.accel.bounds()
    }
}


impl<'a> Surface for TriangleMesh<'a> {
    fn intersect_rays(
        &self,
        accel_rays: &mut [AccelRay],
        wrays: &[Ray],
        isects: &mut [SurfaceIntersection],
        shader: &SurfaceShader,
        space: &[Matrix4x4],
    ) {
        // Precalculate transform for non-motion blur cases
        let static_mat_space = if space.len() == 1 {
            lerp_slice(space, 0.0).inverse()
        } else {
            Matrix4x4::new()
        };

        self.accel.traverse(
            &mut accel_rays[..],
            self.indices,
            |tri_indices, rs| {
                for r in rs {
                    let wr = &wrays[r.id as usize];

                    // Get triangle
                    let tri = {
                        let p0_slice = &self.vertices[(tri_indices.0 as usize *
                                                           self.time_sample_count)..
                                                          ((tri_indices.0 as usize + 1) *
                                                               self.time_sample_count)];
                        let p1_slice = &self.vertices[(tri_indices.1 as usize *
                                                           self.time_sample_count)..
                                                          ((tri_indices.1 as usize + 1) *
                                                               self.time_sample_count)];
                        let p2_slice = &self.vertices[(tri_indices.2 as usize *
                                                           self.time_sample_count)..
                                                          ((tri_indices.2 as usize + 1) *
                                                               self.time_sample_count)];

                        let p0 = lerp_slice(p0_slice, wr.time);
                        let p1 = lerp_slice(p1_slice, wr.time);
                        let p2 = lerp_slice(p2_slice, wr.time);

                        (p0, p1, p2)
                    };

                    // Transform triangle as necessary, and get transform
                    // space.
                    let (mat_space, tri) = if !space.is_empty() {
                        if space.len() > 1 {
                            // Per-ray transform, for motion blur
                            let mat_space = lerp_slice(space, wr.time).inverse();
                            (mat_space, (
                                tri.0 * mat_space,
                                tri.1 * mat_space,
                                tri.2 * mat_space,
                            ))
                        } else {
                            // Same transform for all rays
                            (static_mat_space, (
                                tri.0 * static_mat_space,
                                tri.1 * static_mat_space,
                                tri.2 * static_mat_space,
                            ))
                        }
                    } else {
                        // No transforms
                        (Matrix4x4::new(), tri)
                    };

                    // Test ray against triangle
                    if let Some((t, b0, b1, b2)) = triangle::intersect_ray(wr, tri) {
                        if t < r.max_t {
                            if r.is_occlusion() {
                                isects[r.id as usize] = SurfaceIntersection::Occlude;
                                r.mark_done();
                            } else {
                                // Calculate intersection point and error magnitudes
                                let pos = ((tri.0.into_vector() * b0) + (tri.1.into_vector() * b1) +
                                               (tri.2.into_vector() * b2))
                                    .into_point();

                                let pos_err = (((tri.0.into_vector().abs() * b0) +
                                                    (tri.1.into_vector().abs() * b1) +
                                                    (tri.2.into_vector().abs() * b2)) *
                                                   fp_gamma(7)).co
                                    .h_max();

                                // Calculate geometric surface normal
                                let geo_normal = cross(tri.0 - tri.1, tri.0 - tri.2).into_normal();

                                // Calculate interpolated surface normal, if any
                                let shading_normal = if let Some(normals) = self.normals {
                                    let n0_slice = &normals[(tri_indices.0 as usize *
                                                                 self.time_sample_count)..
                                                                ((tri_indices.0 as usize + 1) *
                                                                     self.time_sample_count)];
                                    let n1_slice = &normals[(tri_indices.1 as usize *
                                                                 self.time_sample_count)..
                                                                ((tri_indices.1 as usize + 1) *
                                                                     self.time_sample_count)];
                                    let n2_slice = &normals[(tri_indices.2 as usize *
                                                                 self.time_sample_count)..
                                                                ((tri_indices.2 as usize + 1) *
                                                                     self.time_sample_count)];

                                    let n0 = lerp_slice(n0_slice, wr.time).normalized();
                                    let n1 = lerp_slice(n1_slice, wr.time).normalized();
                                    let n2 = lerp_slice(n2_slice, wr.time).normalized();

                                    let s_nor = ((n0 * b0) + (n1 * b1) + (n2 * b2)) * mat_space;
                                    if dot(s_nor, geo_normal) >= 0.0 {
                                        s_nor
                                    } else {
                                        -s_nor
                                    }
                                } else {
                                    geo_normal
                                };

                                let intersection_data = SurfaceIntersectionData {
                                    incoming: wr.dir,
                                    t: t,
                                    pos: pos,
                                    pos_err: pos_err,
                                    nor: shading_normal,
                                    nor_g: geo_normal,
                                    uv: (0.0, 0.0), // TODO
                                    local_space: mat_space,
                                };

                                // Fill in intersection data
                                isects[r.id as usize] = SurfaceIntersection::Hit {
                                    intersection_data: intersection_data,
                                    closure: shader.shade(
                                        &intersection_data,
                                        wr.time,
                                        wr.wavelength,
                                    ),
                                };
                                r.max_t = t;
                            }
                        }
                    }
                }
            },
        );
    }
}
