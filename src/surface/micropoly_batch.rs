#![allow(dead_code)]

use mem_arena::MemArena;

use crate::{
    accel::BVH4,
    bbox::BBox,
    boundable::Boundable,
    lerp::lerp_slice,
    math::{cross, dot, Matrix4x4, Normal, Point},
    ray::{RayBatch, RayStack, RayTask}
    shading::surface_closure::SurfaceClosure,
};

use super::{triangle, SurfaceIntersection, SurfaceIntersectionData};

/// This is the core surface primitive for rendering: all surfaces are
/// ultimately processed into pre-shaded micropolygon batches for rendering.
///
/// It is essentially a triangle soup that shares the same surface shader.
/// The parameters of that shader can vary over the triangles, but its
/// structure cannot.
#[derive(Copy, Clone, Debug)]
pub struct MicropolyBatch<'a> {
    // Vertices and associated normals.  Time samples for the same vertex are
    // laid out next to each other in a contiguous slice.
    time_sample_count: usize,
    vertices: &'a [Point],
    normals: &'a [Normal],

    // Per-vertex shading data.
    vertex_closures: &'a [SurfaceClosure],

    // Micro-triangle indices.  Each element of the tuple specifies the index
    // of a vertex, which indexes into all of the arrays above.
    indices: &'a [(u32, u32, u32)],

    // Acceleration structure for fast ray intersection testing.
    accel: BVH4<'a>,
}

impl<'a> MicropolyBatch<'a> {
    pub fn from_verts_and_indices<'b>(
        arena: &'b MemArena,
        geo_time_sample_count: usize,
        verts: &[Point],
        vert_normals: &[Normal],
        vert_closures: &[SurfaceClosure],
        triangles: &[(u32, u32, u32)],
    ) -> MicropolyBatch<'b> {
        // Create bounds array for use during BVH construction
        let bounds = {
            let mut bounds = Vec::with_capacity(triangles.len() * geo_time_sample_count);
            for tri in triangles {
                for ti in 0..geo_time_sample_count {
                    let p0 = verts[(tri.0 as usize * geo_time_sample_count) + ti];
                    let p1 = verts[(tri.1 as usize * geo_time_sample_count) + ti];
                    let p2 = verts[(tri.2 as usize * geo_time_sample_count) + ti];
                    let minimum = p0.min(p1.min(p2));
                    let maximum = p0.max(p1.max(p2));
                    bounds.push(BBox::from_points(minimum, maximum));
                }
            }
            bounds
        };

        // Create an array of triangle indices for use during the BVH build.
        let mut tmp_indices: Vec<_> = (0u32..(triangles.len() as u32)).collect();

        // Build BVH
        let accel = BVH4::from_objects(arena, &mut tmp_indices[..], 3, |index| {
            &bounds[(*index as usize * geo_time_sample_count)
                ..((*index as usize + 1) * geo_time_sample_count)]
        });

        // Copy triangle vertex indices over in the post-bvh-build order.
        let indices = {
            let indices = unsafe { arena.alloc_array_uninitialized(triangles.len()) };
            for (i, tmp_i) in tmp_indices.iter().enumerate() {
                indices[i] = triangles[*tmp_i as usize];
            }
            indices
        };

        MicropolyBatch {
            time_sample_count: geo_time_sample_count,
            vertices: arena.copy_slice(verts),
            normals: arena.copy_slice(vert_normals),

            vertex_closures: arena.copy_slice(vert_closures),

            indices: indices,

            accel: accel,
        }
    }
}

impl<'a> MicropolyBatch<'a> {
    fn intersect_rays(
        &self,
        rays: &mut RayBatch,
        ray_stack: &mut RayStack,
        isects: &mut [SurfaceIntersection],
        space: &[Matrix4x4],
    ) {
        // Precalculate transform for non-motion blur cases
        let static_mat_space = if space.len() == 1 {
            space[0].inverse()
        } else {
            Matrix4x4::new()
        };

        self.accel
            .traverse(rays, ray_stack, self.indices, |tri_indices, rs| {
                // For static triangles with static transforms, cache them.
                let is_cached = self.time_sample_count == 1 && space.len() <= 1;
                let mut tri = if is_cached {
                    let tri = (
                        self.vertices[tri_indices.0 as usize],
                        self.vertices[tri_indices.1 as usize],
                        self.vertices[tri_indices.2 as usize],
                    );
                    if space.is_empty() {
                        tri
                    } else {
                        (
                            tri.0 * static_mat_space,
                            tri.1 * static_mat_space,
                            tri.2 * static_mat_space,
                        )
                    }
                } else {
                    unsafe { std::mem::uninitialized() }
                };

                // Test each ray against the current triangle.
                for r in rs {
                    let wr = &wrays[r.id as usize];

                    // Get triangle if necessary
                    if !is_cached {
                        tri = if self.time_sample_count == 1 {
                            // No deformation motion blur, so fast-path it.
                            (
                                self.vertices[tri_indices.0 as usize],
                                self.vertices[tri_indices.1 as usize],
                                self.vertices[tri_indices.2 as usize],
                            )
                        } else {
                            // Deformation motion blur, need to interpolate.
                            let p0_slice = &self.vertices[(tri_indices.0 as usize
                                * self.time_sample_count)
                                ..((tri_indices.0 as usize + 1) * self.time_sample_count)];
                            let p1_slice = &self.vertices[(tri_indices.1 as usize
                                * self.time_sample_count)
                                ..((tri_indices.1 as usize + 1) * self.time_sample_count)];
                            let p2_slice = &self.vertices[(tri_indices.2 as usize
                                * self.time_sample_count)
                                ..((tri_indices.2 as usize + 1) * self.time_sample_count)];

                            let p0 = lerp_slice(p0_slice, wr.time);
                            let p1 = lerp_slice(p1_slice, wr.time);
                            let p2 = lerp_slice(p2_slice, wr.time);

                            (p0, p1, p2)
                        };
                    }

                    // Transform triangle if necessary, and get transform space.
                    let mat_space = if !space.is_empty() {
                        if space.len() > 1 {
                            // Per-ray transform, for motion blur
                            let mat_space = lerp_slice(space, wr.time).inverse();
                            tri = (tri.0 * mat_space, tri.1 * mat_space, tri.2 * mat_space);
                            mat_space
                        } else {
                            // Same transform for all rays
                            if !is_cached {
                                tri = (
                                    tri.0 * static_mat_space,
                                    tri.1 * static_mat_space,
                                    tri.2 * static_mat_space,
                                );
                            }
                            static_mat_space
                        }
                    } else {
                        // No transforms
                        Matrix4x4::new()
                    };

                    // Test ray against triangle
                    if let Some((t, b0, b1, b2)) = triangle::intersect_ray(wr, tri) {
                        if t < r.max_t {
                            if r.is_occlusion() {
                                isects[r.id as usize] = SurfaceIntersection::Occlude;
                                r.mark_done();
                            } else {
                                // Calculate intersection point and error magnitudes
                                let (pos, pos_err) = triangle::surface_point(tri, (b0, b1, b2));

                                // Calculate geometric surface normal
                                let geo_normal = cross(tri.0 - tri.1, tri.0 - tri.2).into_normal();

                                // Calculate interpolated surface normal
                                let shading_normal = {
                                    let n0_slice = &self.normals[(tri_indices.0 as usize
                                        * self.time_sample_count)
                                        ..((tri_indices.0 as usize + 1) * self.time_sample_count)];
                                    let n1_slice = &self.normals[(tri_indices.1 as usize
                                        * self.time_sample_count)
                                        ..((tri_indices.1 as usize + 1) * self.time_sample_count)];
                                    let n2_slice = &self.normals[(tri_indices.2 as usize
                                        * self.time_sample_count)
                                        ..((tri_indices.2 as usize + 1) * self.time_sample_count)];

                                    let n0 = lerp_slice(n0_slice, wr.time).normalized();
                                    let n1 = lerp_slice(n1_slice, wr.time).normalized();
                                    let n2 = lerp_slice(n2_slice, wr.time).normalized();

                                    let s_nor = ((n0 * b0) + (n1 * b1) + (n2 * b2)) * mat_space;
                                    if dot(s_nor, geo_normal) >= 0.0 {
                                        s_nor
                                    } else {
                                        -s_nor
                                    }
                                };

                                // Calculate surface closure
                                // TODO: use interpolation between the vertices
                                let surface_closure = self.vertex_closures[tri_indices.0 as usize];

                                // Fill in intersection data
                                isects[r.id as usize] = SurfaceIntersection::Hit {
                                    intersection_data: SurfaceIntersectionData {
                                        incoming: wr.dir,
                                        t: t,
                                        pos: pos,
                                        pos_err: pos_err,
                                        nor: shading_normal,
                                        nor_g: geo_normal,
                                        local_space: mat_space,
                                        sample_pdf: 0.0,
                                    },
                                    closure: surface_closure,
                                };
                                r.max_t = t;
                            }
                        }
                    }
                }
            });
    }
}

impl<'a> Boundable for MicropolyBatch<'a> {
    fn bounds(&self) -> &[BBox] {
        self.accel.bounds()
    }
}
