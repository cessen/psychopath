#![allow(dead_code)]

use std::collections::HashMap;

use kioku::Arena;

use crate::{
    accel::BVH4,
    bbox::BBox,
    boundable::Boundable,
    lerp::lerp_slice,
    math::{cross, dot, Matrix4x4, Normal, Point},
    ray::{RayBatch, RayStack},
    shading::SurfaceClosure,
};

use super::{triangle, SurfaceIntersection, SurfaceIntersectionData};

const MAX_LEAF_TRIANGLE_COUNT: usize = 3;

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
    compressed_vertex_closure_size: usize, // Size in bites of a single compressed closure
    vertex_closure_time_sample_count: usize,
    compressed_vertex_closures: &'a [u8], // Packed compressed closures

    // Micro-triangle indices.  Each element of the tuple specifies the index
    // of a vertex, which indexes into all of the arrays above.
    indices: &'a [(u32, u32, u32)],

    // Acceleration structure for fast ray intersection testing.
    accel: BVH4<'a>,
}

impl<'a> MicropolyBatch<'a> {
    pub fn from_verts_and_indices<'b>(
        arena: &'b Arena,
        verts: &[Vec<Point>],
        vert_normals: &[Vec<Normal>],
        tri_indices: &[(usize, usize, usize)],
    ) -> MicropolyBatch<'b> {
        let vert_count = verts[0].len();
        let time_sample_count = verts.len();

        // Copy verts over to a contiguous area of memory, reorganizing them
        // so that each vertices' time samples are contiguous in memory.
        let vertices = {
            let vertices = arena.alloc_array_uninit(vert_count * time_sample_count);

            for vi in 0..vert_count {
                for ti in 0..time_sample_count {
                    unsafe {
                        *vertices[(vi * time_sample_count) + ti].as_mut_ptr() = verts[ti][vi];
                    }
                }
            }

            unsafe { std::mem::transmute(vertices) }
        };

        // Copy vertex normals, if any, organizing them the same as vertices
        // above.
        let normals = {
            let normals = arena.alloc_array_uninit(vert_count * time_sample_count);

            for vi in 0..vert_count {
                for ti in 0..time_sample_count {
                    unsafe {
                        *normals[(vi * time_sample_count) + ti].as_mut_ptr() = vert_normals[ti][vi];
                    }
                }
            }

            unsafe { std::mem::transmute(&normals[..]) }
        };

        // Copy triangle vertex indices over, appending the triangle index itself to the tuple
        let indices: &mut [(u32, u32, u32)] = {
            let indices = arena.alloc_array_uninit(tri_indices.len());
            for (i, tri_i) in tri_indices.iter().enumerate() {
                unsafe {
                    *indices[i].as_mut_ptr() = (tri_i.0 as u32, tri_i.2 as u32, tri_i.1 as u32);
                }
            }
            unsafe { std::mem::transmute(indices) }
        };

        // Create bounds array for use during BVH construction
        let (bounds, bounds_map) = {
            let mut bounds = Vec::with_capacity(indices.len() * time_sample_count);
            let mut bounds_map = HashMap::new();

            for tri in tri_indices {
                let start = bounds.len();
                for ti in 0..time_sample_count {
                    let p0 = verts[ti][tri.0];
                    let p1 = verts[ti][tri.1];
                    let p2 = verts[ti][tri.2];
                    let minimum = p0.min(p1.min(p2));
                    let maximum = p0.max(p1.max(p2));
                    bounds.push(BBox::from_points(minimum, maximum));
                }
                let end = bounds.len();
                bounds_map.insert((tri.0 as u32, tri.1 as u32, tri.2 as u32), (start, end));
            }
            (bounds, bounds_map)
        };

        // Build BVH
        let accel = BVH4::from_objects(arena, &mut indices[..], MAX_LEAF_TRIANGLE_COUNT, |tri| {
            let (start, end) = bounds_map[tri];
            &bounds[start..end]
        });

        MicropolyBatch {
            time_sample_count: time_sample_count,
            vertices: vertices,
            normals: normals,
            compressed_vertex_closure_size: 0,
            vertex_closure_time_sample_count: 1,
            compressed_vertex_closures: &[],
            indices: indices,
            accel: accel,
        }
    }
}

impl<'a> Boundable for MicropolyBatch<'a> {
    fn bounds(&self) -> &[BBox] {
        self.accel.bounds()
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
            lerp_slice(space, 0.0).inverse()
        } else {
            Matrix4x4::new()
        };

        self.accel
            .traverse(rays, ray_stack, |idx_range, rays, ray_stack| {
                let tri_count = idx_range.end - idx_range.start;

                // Build the triangle cache if we can!
                let is_cached = ray_stack.ray_count_in_next_task() >= tri_count
                    && self.time_sample_count == 1
                    && space.len() <= 1;
                let mut tri_cache = [std::mem::MaybeUninit::uninit(); MAX_LEAF_TRIANGLE_COUNT];
                if is_cached {
                    for tri_idx in idx_range.clone() {
                        let i = tri_idx - idx_range.start;
                        let tri_indices = self.indices[tri_idx];

                        // For static triangles with static transforms, cache them.
                        unsafe {
                            *tri_cache[i].as_mut_ptr() = (
                                self.vertices[tri_indices.0 as usize],
                                self.vertices[tri_indices.1 as usize],
                                self.vertices[tri_indices.2 as usize],
                            );
                            if !space.is_empty() {
                                (*tri_cache[i].as_mut_ptr()).0 =
                                    (*tri_cache[i].as_mut_ptr()).0 * static_mat_space;
                                (*tri_cache[i].as_mut_ptr()).1 =
                                    (*tri_cache[i].as_mut_ptr()).1 * static_mat_space;
                                (*tri_cache[i].as_mut_ptr()).2 =
                                    (*tri_cache[i].as_mut_ptr()).2 * static_mat_space;
                            }
                        }
                    }
                }

                // Test each ray against the triangles.
                ray_stack.do_next_task(|ray_idx| {
                    let ray_idx = ray_idx as usize;

                    if rays.is_done(ray_idx) {
                        return;
                    }

                    let ray_time = rays.time(ray_idx);

                    // Calculate the ray space, if necessary.
                    let mat_space = if space.len() > 1 {
                        // Per-ray transform, for motion blur
                        lerp_slice(space, ray_time).inverse()
                    } else {
                        static_mat_space
                    };

                    // Iterate through the triangles and test the ray against them.
                    let mut non_shadow_hit = false;
                    let mut hit_tri = std::mem::MaybeUninit::uninit();
                    let mut hit_tri_indices = std::mem::MaybeUninit::uninit();
                    let mut hit_tri_data = std::mem::MaybeUninit::uninit();
                    let ray_pre = triangle::RayTriPrecompute::new(rays.dir(ray_idx));
                    for tri_idx in idx_range.clone() {
                        let tri_indices = self.indices[tri_idx];

                        // Get triangle if necessary
                        let tri = if is_cached {
                            let i = tri_idx - idx_range.start;
                            unsafe { tri_cache[i].assume_init() }
                        } else {
                            let mut tri = if self.time_sample_count == 1 {
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

                                let p0 = lerp_slice(p0_slice, ray_time);
                                let p1 = lerp_slice(p1_slice, ray_time);
                                let p2 = lerp_slice(p2_slice, ray_time);

                                (p0, p1, p2)
                            };

                            if !space.is_empty() {
                                tri.0 = tri.0 * mat_space;
                                tri.1 = tri.1 * mat_space;
                                tri.2 = tri.2 * mat_space;
                            }

                            tri
                        };

                        // Test ray against triangle
                        if let Some((t, b0, b1, b2)) = triangle::intersect_ray(
                            rays.orig(ray_idx),
                            ray_pre,
                            rays.max_t(ray_idx),
                            tri,
                        ) {
                            if rays.is_occlusion(ray_idx) {
                                isects[ray_idx] = SurfaceIntersection::Occlude;
                                rays.mark_done(ray_idx);
                                break;
                            } else {
                                non_shadow_hit = true;
                                rays.set_max_t(ray_idx, t);
                                unsafe {
                                    *hit_tri.as_mut_ptr() = tri;
                                    *hit_tri_indices.as_mut_ptr() = tri_indices;
                                    *hit_tri_data.as_mut_ptr() = (t, b0, b1, b2);
                                }
                            }
                        }
                    }

                    // Calculate intersection data if necessary.
                    if non_shadow_hit {
                        let hit_tri = unsafe { hit_tri.assume_init() };
                        let hit_tri_indices = unsafe { hit_tri_indices.assume_init() };
                        let (t, b0, b1, b2) = unsafe { hit_tri_data.assume_init() };

                        // Calculate intersection point and error magnitudes
                        let (pos, pos_err) = triangle::surface_point(hit_tri, (b0, b1, b2));

                        // Calculate geometric surface normal
                        let geo_normal =
                            cross(hit_tri.0 - hit_tri.1, hit_tri.0 - hit_tri.2).into_normal();

                        // Calculate interpolated surface normal
                        let shading_normal = {
                            let n0_slice = &self.normals[(hit_tri_indices.0 as usize
                                * self.time_sample_count)
                                ..((hit_tri_indices.0 as usize + 1) * self.time_sample_count)];
                            let n1_slice = &self.normals[(hit_tri_indices.1 as usize
                                * self.time_sample_count)
                                ..((hit_tri_indices.1 as usize + 1) * self.time_sample_count)];
                            let n2_slice = &self.normals[(hit_tri_indices.2 as usize
                                * self.time_sample_count)
                                ..((hit_tri_indices.2 as usize + 1) * self.time_sample_count)];

                            let n0 = lerp_slice(n0_slice, ray_time).normalized();
                            let n1 = lerp_slice(n1_slice, ray_time).normalized();
                            let n2 = lerp_slice(n2_slice, ray_time).normalized();

                            let s_nor = ((n0 * b0) + (n1 * b1) + (n2 * b2)) * mat_space;
                            if dot(s_nor, geo_normal) >= 0.0 {
                                s_nor
                            } else {
                                -s_nor
                            }
                        };

                        // Calculate interpolated surface closure.
                        // TODO: actually interpolate.
                        let closure = {
                            let start_byte = hit_tri_indices.0 as usize
                                * self.compressed_vertex_closure_size
                                * self.vertex_closure_time_sample_count;
                            let end_byte = start_byte + self.compressed_vertex_closure_size;
                            let (closure, _) = SurfaceClosure::from_compressed(
                                &self.compressed_vertex_closures[start_byte..end_byte],
                            );
                            closure
                        };

                        let intersection_data = SurfaceIntersectionData {
                            incoming: rays.dir(ray_idx),
                            t: t,
                            pos: pos,
                            pos_err: pos_err,
                            nor: shading_normal,
                            nor_g: geo_normal,
                            local_space: mat_space,
                            sample_pdf: 0.0,
                        };

                        // Fill in intersection data
                        isects[ray_idx] = SurfaceIntersection::Hit {
                            intersection_data: intersection_data,
                            closure: closure,
                        };
                    }
                });
                ray_stack.pop_task();
            });
    }
}
