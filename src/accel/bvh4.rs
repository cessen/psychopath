#![allow(dead_code)]

use std;

use mem_arena::MemArena;

use algorithm::partition;
use bbox::BBox;
use boundable::Boundable;
use lerp::lerp_slice;
use ray::AccelRay;
use timer::Timer;

use bvh_order::{calc_traversal_code, SplitAxes, TRAVERSAL_TABLE};
use super::bvh_base::{BVHBase, BVHBaseNode, BVH_MAX_DEPTH};
use super::ACCEL_TRAV_TIME;
use super::ACCEL_NODE_RAY_TESTS;

#[derive(Copy, Clone, Debug)]
pub struct BVH4<'a> {
    root: Option<&'a BVH4Node<'a>>,
    depth: usize,
}

#[derive(Copy, Clone, Debug)]
pub enum BVH4Node<'a> {
    Inner {
        traversal_code: u8,
        bounds_start: &'a BBox,
        bounds_len: u16,
        children: &'a [BVH4Node<'a>],
    },

    Leaf {
        bounds_start: &'a BBox,
        bounds_len: u16,
        object_range: (usize, usize),
    },
}

impl<'a> BVH4<'a> {
    pub fn from_objects<'b, T, F>(
        arena: &'a MemArena,
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
            }
        } else {
            let base = BVHBase::from_objects(objects, objects_per_leaf, bounder);

            let root = unsafe { arena.alloc_uninitialized::<BVH4Node>() };
            BVH4::construct_from_base(arena, &base, base.root_node_index(), root);
            BVH4 {
                root: Some(root),
                depth: base.depth,
            }
        }
    }

    pub fn tree_depth(&self) -> usize {
        self.depth
    }

    pub fn traverse<T, F>(&self, rays: &mut [AccelRay], objects: &[T], mut obj_ray_test: F)
    where
        F: FnMut(&T, &mut [AccelRay]),
    {
        if self.root.is_none() {
            return;
        }

        let mut timer = Timer::new();
        let mut trav_time: f64 = 0.0;
        let mut node_tests: u64 = 0;

        let traversal_table = {
            let ray_sign_is_neg = [
                rays[0].dir_inv.x() < 0.0,
                rays[0].dir_inv.y() < 0.0,
                rays[0].dir_inv.z() < 0.0,
            ];
            let ray_code = ray_sign_is_neg[0] as usize + ((ray_sign_is_neg[1] as usize) << 1)
                + ((ray_sign_is_neg[2] as usize) << 2);
            &TRAVERSAL_TABLE[ray_code]
        };

        // +2 of max depth for root and last child
        let mut node_stack = [self.root.unwrap(); (BVH_MAX_DEPTH * 3) + 2];
        let mut ray_i_stack = [rays.len(); (BVH_MAX_DEPTH * 3) + 2];
        let mut stack_ptr = 1;

        while stack_ptr > 0 {
            node_tests += ray_i_stack[stack_ptr] as u64;
            match *node_stack[stack_ptr] {
                BVH4Node::Inner {
                    traversal_code,
                    bounds_start,
                    bounds_len,
                    children,
                } => {
                    let bounds =
                        unsafe { std::slice::from_raw_parts(bounds_start, bounds_len as usize) };
                    let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
                        (!r.is_done()) && lerp_slice(bounds, r.time).intersect_accel_ray(r)
                    });
                    if part > 0 {
                        let order_code = traversal_table[traversal_code as usize];
                        match children.len() {
                            4 => {
                                let i4 = ((order_code >> 6) & 0b11) as usize;
                                let i3 = ((order_code >> 4) & 0b11) as usize;
                                let i2 = ((order_code >> 2) & 0b11) as usize;
                                let i1 = (order_code & 0b11) as usize;

                                ray_i_stack[stack_ptr] = part;
                                ray_i_stack[stack_ptr + 1] = part;
                                ray_i_stack[stack_ptr + 2] = part;
                                ray_i_stack[stack_ptr + 3] = part;

                                node_stack[stack_ptr] = &children[i4];
                                node_stack[stack_ptr + 1] = &children[i3];
                                node_stack[stack_ptr + 2] = &children[i2];
                                node_stack[stack_ptr + 3] = &children[i1];

                                stack_ptr += 3;
                            }
                            3 => {
                                let i3 = ((order_code >> 4) & 0b11) as usize;
                                let i2 = ((order_code >> 2) & 0b11) as usize;
                                let i1 = (order_code & 0b11) as usize;

                                ray_i_stack[stack_ptr] = part;
                                ray_i_stack[stack_ptr + 1] = part;
                                ray_i_stack[stack_ptr + 2] = part;

                                node_stack[stack_ptr] = &children[i3];
                                node_stack[stack_ptr + 1] = &children[i2];
                                node_stack[stack_ptr + 2] = &children[i1];

                                stack_ptr += 2;
                            }
                            2 => {
                                let i2 = ((order_code >> 2) & 0b11) as usize;
                                let i1 = (order_code & 0b11) as usize;

                                ray_i_stack[stack_ptr] = part;
                                ray_i_stack[stack_ptr + 1] = part;

                                node_stack[stack_ptr] = &children[i2];
                                node_stack[stack_ptr + 1] = &children[i1];

                                stack_ptr += 1;
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        stack_ptr -= 1;
                    }
                }

                BVH4Node::Leaf {
                    object_range,
                    bounds_start,
                    bounds_len,
                } => {
                    let bounds =
                        unsafe { std::slice::from_raw_parts(bounds_start, bounds_len as usize) };
                    let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
                        (!r.is_done()) && lerp_slice(bounds, r.time).intersect_accel_ray(r)
                    });

                    trav_time += timer.tick() as f64;

                    if part > 0 {
                        for obj in &objects[object_range.0..object_range.1] {
                            obj_ray_test(obj, &mut rays[..part]);
                        }
                    }

                    timer.tick();

                    stack_ptr -= 1;
                }
            }
        }

        trav_time += timer.tick() as f64;
        ACCEL_TRAV_TIME.with(|att| {
            let v = att.get();
            att.set(v + trav_time);
        });
        ACCEL_NODE_RAY_TESTS.with(|anv| {
            let v = anv.get();
            anv.set(v + node_tests);
        });
    }

    fn construct_from_base(
        arena: &'a MemArena,
        base: &BVHBase,
        node_index: usize,
        node_mem: &mut BVH4Node<'a>,
    ) {
        match base.nodes[node_index] {
            BVHBaseNode::Internal {
                bounds_range,
                children_indices,
                split_axis,
            } => {
                let child_l = &base.nodes[children_indices.0];
                let child_r = &base.nodes[children_indices.1];

                // Prepare convenient access to the stuff we need.
                let child_count: usize;
                let child_indices: [usize; 4];
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
                                child_indices = [i_l.0, i_l.1, i_r.0, i_r.1];
                                split_info = SplitAxes::Full((split_axis, s_l, s_r));
                            }
                            BVHBaseNode::Leaf { .. } => {
                                // Three nodes with left split
                                child_count = 3;
                                child_indices = [i_l.0, i_l.1, children_indices.1, 0];
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
                                child_indices = [children_indices.0, i_r.0, i_r.1, 0];
                                split_info = SplitAxes::Right((split_axis, s_r));
                            }
                            BVHBaseNode::Leaf { .. } => {
                                // Two nodes
                                child_count = 2;
                                child_indices = [children_indices.0, children_indices.1, 0, 0];
                                split_info = SplitAxes::TopOnly(split_axis);
                            }
                        }
                    }
                }

                // Copy bounds
                let bounds = arena
                    .copy_slice_with_alignment(&base.bounds[bounds_range.0..bounds_range.1], 32);

                // Build children
                let mut children_mem = unsafe {
                    arena.alloc_array_uninitialized_with_alignment::<BVH4Node>(child_count, 32)
                };
                for i in 0..child_count {
                    BVH4::construct_from_base(arena, base, child_indices[i], &mut children_mem[i]);
                }

                // Fill in node
                *node_mem = BVH4Node::Inner {
                    traversal_code: calc_traversal_code(split_info),
                    bounds_start: &bounds[0],
                    bounds_len: bounds.len() as u16,
                    children: children_mem,
                };
            }

            BVHBaseNode::Leaf {
                bounds_range,
                object_range,
            } => {
                let bounds = arena.copy_slice(&base.bounds[bounds_range.0..bounds_range.1]);

                *node_mem = BVH4Node::Leaf {
                    bounds_start: &bounds[0],
                    bounds_len: bounds.len() as u16,
                    object_range: object_range,
                };
            }
        }
    }
}

lazy_static! {
    static ref DEGENERATE_BOUNDS: [BBox; 1] = [BBox::new()];
}

impl<'a> Boundable for BVH4<'a> {
    fn bounds(&self) -> &[BBox] {
        match self.root {
            None => &DEGENERATE_BOUNDS[..],
            Some(root) => match *root {
                BVH4Node::Inner {
                    bounds_start,
                    bounds_len,
                    ..
                }
                | BVH4Node::Leaf {
                    bounds_start,
                    bounds_len,
                    ..
                } => unsafe { std::slice::from_raw_parts(bounds_start, bounds_len as usize) },
            },
        }
    }
}
