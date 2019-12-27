//! This BVH4 implementation is based on the ideas from the paper
//! "Efficient Ray Tracing Kernels for Modern CPU Architectures"
//! by Fuetterling et al.

#![allow(dead_code)]

use std::mem::{transmute, MaybeUninit};

use glam::Vec4Mask;

use kioku::Arena;

use crate::{
    bbox::BBox,
    bbox4::BBox4,
    boundable::Boundable,
    lerp::lerp_slice,
    math::Vector,
    ray::{RayBatch, RayStack},
};

use super::{
    bvh_base::{BVHBase, BVHBaseNode, BVH_MAX_DEPTH},
    ACCEL_NODE_RAY_TESTS,
};

use bvh_order::{calc_traversal_code, SplitAxes, TRAVERSAL_TABLE};

pub fn ray_code(dir: Vector) -> usize {
    let ray_sign_is_neg = [dir.x() < 0.0, dir.y() < 0.0, dir.z() < 0.0];
    ray_sign_is_neg[0] as usize
        + ((ray_sign_is_neg[1] as usize) << 1)
        + ((ray_sign_is_neg[2] as usize) << 2)
}

#[derive(Copy, Clone, Debug)]
pub struct BVH4<'a> {
    root: Option<&'a BVH4Node<'a>>,
    depth: usize,
    node_count: usize,
    _bounds: Option<&'a [BBox]>,
}

#[derive(Copy, Clone, Debug)]
pub enum BVH4Node<'a> {
    Internal {
        bounds: &'a [BBox4],
        children: &'a [BVH4Node<'a>],
        traversal_code: u8,
    },

    Leaf {
        object_range: (usize, usize),
    },
}

impl<'a> BVH4<'a> {
    pub fn from_objects<'b, T, F>(
        arena: &'a Arena,
        objects: &mut [T],
        objects_per_leaf: usize,
        bounder: F,
    ) -> BVH4<'a>
    where
        F: 'b + Fn(&T) -> &'b [BBox],
    {
        if objects.is_empty() {
            BVH4 {
                root: None,
                depth: 0,
                node_count: 0,
                _bounds: None,
            }
        } else {
            let base = BVHBase::from_objects(objects, objects_per_leaf, bounder);

            let fill_node = arena.alloc_align_uninit::<BVH4Node>(32);
            let node_count = BVH4::construct_from_base(
                arena,
                &base,
                &base.nodes[base.root_node_index()],
                fill_node,
            );

            BVH4 {
                root: Some(unsafe { transmute(fill_node) }),
                depth: (base.depth / 2) + 1,
                node_count: node_count,
                _bounds: {
                    let range = base.nodes[base.root_node_index()].bounds_range();
                    Some(arena.copy_slice(&base.bounds[range.0..range.1]))
                },
            }
        }
    }

    pub fn tree_depth(&self) -> usize {
        self.depth
    }

    pub fn traverse<F>(&self, rays: &mut RayBatch, ray_stack: &mut RayStack, mut obj_ray_test: F)
    where
        F: FnMut(std::ops::Range<usize>, &mut RayBatch, &mut RayStack),
    {
        if self.root.is_none() {
            return;
        }

        let mut node_tests: u64 = 0;

        let traversal_table =
            &TRAVERSAL_TABLE[ray_code(rays.dir_inv_local(ray_stack.next_task_ray_idx(0)))];

        // +2 of max depth for root and last child
        let mut node_stack = [self.root.unwrap(); (BVH_MAX_DEPTH * 3) + 2];
        let mut stack_ptr = 1;

        while stack_ptr > 0 {
            match *node_stack[stack_ptr] {
                BVH4Node::Internal {
                    bounds,
                    children,
                    traversal_code,
                } => {
                    node_tests += ray_stack.ray_count_in_next_task() as u64;
                    let mut all_hits = Vec4Mask::default();

                    // Ray testing
                    ray_stack.pop_do_next_task_and_push_rays(children.len(), |ray_idx| {
                        if rays.is_done(ray_idx) {
                            Vec4Mask::default()
                        } else {
                            let hits = if bounds.len() == 1 {
                                bounds[0].intersect_ray(
                                    rays.orig_local(ray_idx),
                                    rays.dir_inv_local(ray_idx),
                                    rays.max_t(ray_idx),
                                )
                            } else {
                                lerp_slice(bounds, rays.time(ray_idx)).intersect_ray(
                                    rays.orig_local(ray_idx),
                                    rays.dir_inv_local(ray_idx),
                                    rays.max_t(ray_idx),
                                )
                            };
                            all_hits |= hits;
                            hits
                        }
                    });

                    // If there were any intersections, create tasks.
                    if all_hits.any() {
                        let order_code = traversal_table[traversal_code as usize];
                        let mut lane_count = 0;
                        let mut i = children.len() as u8;
                        while i > 0 {
                            i -= 1;
                            let child_i = ((order_code >> (i * 2)) & 3) as usize;
                            if ray_stack.push_lane_to_task(child_i) {
                                node_stack[stack_ptr + lane_count] = &children[child_i];
                                lane_count += 1;
                            }
                        }

                        stack_ptr += lane_count - 1;
                    } else {
                        stack_ptr -= 1;
                    }
                }

                BVH4Node::Leaf { object_range } => {
                    // Do the ray tests.
                    obj_ray_test(object_range.0..object_range.1, rays, ray_stack);

                    stack_ptr -= 1;
                }
            }
        }

        ACCEL_NODE_RAY_TESTS.with(|anv| {
            let v = anv.get();
            anv.set(v + node_tests);
        });
    }

    fn construct_from_base(
        arena: &'a Arena,
        base: &BVHBase,
        node: &BVHBaseNode,
        fill_node: &mut MaybeUninit<BVH4Node<'a>>,
    ) -> usize {
        let mut node_count = 0;

        match *node {
            // Create internal node
            BVHBaseNode::Internal {
                children_indices,
                split_axis,
                ..
            } => {
                let child_l = &base.nodes[children_indices.0];
                let child_r = &base.nodes[children_indices.1];

                // Prepare convenient access to the stuff we need.
                let child_count: usize;
                let children; // [Optional, Optional, Optional, Optional]
                let split_info: SplitAxes;
                match *child_l {
                    BVHBaseNode::Internal {
                        children_indices: i_l,
                        split_axis: s_l,
                        ..
                    } => {
                        match *child_r {
                            BVHBaseNode::Internal {
                                children_indices: i_r,
                                split_axis: s_r,
                                ..
                            } => {
                                // Four nodes
                                child_count = 4;
                                children = [
                                    Some(&base.nodes[i_l.0]),
                                    Some(&base.nodes[i_l.1]),
                                    Some(&base.nodes[i_r.0]),
                                    Some(&base.nodes[i_r.1]),
                                ];
                                split_info = SplitAxes::Full((split_axis, s_l, s_r));
                            }
                            BVHBaseNode::Leaf { .. } => {
                                // Three nodes with left split
                                child_count = 3;
                                children = [
                                    Some(&base.nodes[i_l.0]),
                                    Some(&base.nodes[i_l.1]),
                                    Some(child_r),
                                    None,
                                ];
                                split_info = SplitAxes::Left((split_axis, s_l));
                            }
                        }
                    }
                    BVHBaseNode::Leaf { .. } => {
                        match *child_r {
                            BVHBaseNode::Internal {
                                children_indices: i_r,
                                split_axis: s_r,
                                ..
                            } => {
                                // Three nodes with right split
                                child_count = 3;
                                children = [
                                    Some(child_l),
                                    Some(&base.nodes[i_r.0]),
                                    Some(&base.nodes[i_r.1]),
                                    None,
                                ];
                                split_info = SplitAxes::Right((split_axis, s_r));
                            }
                            BVHBaseNode::Leaf { .. } => {
                                // Two nodes
                                child_count = 2;
                                children = [Some(child_l), Some(child_r), None, None];
                                split_info = SplitAxes::TopOnly(split_axis);
                            }
                        }
                    }
                }

                node_count += child_count;

                // Construct bounds
                let bounds = {
                    let bounds_len = children
                        .iter()
                        .map(|c| {
                            if let Some(n) = *c {
                                let len = n.bounds_range().1 - n.bounds_range().0;
                                debug_assert!(len >= 1);
                                len
                            } else {
                                0
                            }
                        })
                        .max()
                        .unwrap();
                    debug_assert!(bounds_len >= 1);
                    let bounds = arena.alloc_array_align_uninit(bounds_len, 32);
                    if bounds_len < 2 {
                        let b1 =
                            children[0].map_or(BBox::new(), |c| base.bounds[c.bounds_range().0]);
                        let b2 =
                            children[1].map_or(BBox::new(), |c| base.bounds[c.bounds_range().0]);
                        let b3 =
                            children[2].map_or(BBox::new(), |c| base.bounds[c.bounds_range().0]);
                        let b4 =
                            children[3].map_or(BBox::new(), |c| base.bounds[c.bounds_range().0]);
                        unsafe {
                            *bounds[0].as_mut_ptr() = BBox4::from_bboxes(b1, b2, b3, b4);
                        }
                    } else {
                        for (i, b) in bounds.iter_mut().enumerate() {
                            let time = i as f32 / (bounds_len - 1) as f32;

                            let b1 = children[0].map_or(BBox::new(), |c| {
                                let (x, y) = c.bounds_range();
                                lerp_slice(&base.bounds[x..y], time)
                            });
                            let b2 = children[1].map_or(BBox::new(), |c| {
                                let (x, y) = c.bounds_range();
                                lerp_slice(&base.bounds[x..y], time)
                            });
                            let b3 = children[2].map_or(BBox::new(), |c| {
                                let (x, y) = c.bounds_range();
                                lerp_slice(&base.bounds[x..y], time)
                            });
                            let b4 = children[3].map_or(BBox::new(), |c| {
                                let (x, y) = c.bounds_range();
                                lerp_slice(&base.bounds[x..y], time)
                            });
                            unsafe {
                                *b.as_mut_ptr() = BBox4::from_bboxes(b1, b2, b3, b4);
                            }
                        }
                    }
                    bounds
                };

                // Construct child nodes
                let child_nodes = arena.alloc_array_align_uninit::<BVH4Node>(child_count, 32);
                for (i, c) in children[0..child_count].iter().enumerate() {
                    node_count +=
                        BVH4::construct_from_base(arena, base, c.unwrap(), &mut child_nodes[i]);
                }

                // Build this node
                unsafe {
                    *fill_node.as_mut_ptr() = BVH4Node::Internal {
                        bounds: transmute(bounds),
                        children: transmute(child_nodes),
                        traversal_code: calc_traversal_code(split_info),
                    };
                }
            }

            // Create internal node
            BVHBaseNode::Leaf { object_range, .. } => {
                unsafe {
                    *fill_node.as_mut_ptr() = BVH4Node::Leaf {
                        object_range: object_range,
                    };
                }
                node_count += 1;
            }
        }

        return node_count;
    }
}

impl<'a> Boundable for BVH4<'a> {
    fn bounds<'b>(&'b self) -> &'b [BBox] {
        self._bounds.unwrap_or(&[])
    }
}
