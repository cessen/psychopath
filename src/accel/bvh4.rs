//! This BVH4 implementation pulls a lot of ideas from the paper
//! "Efficient Ray Tracing Kernels for Modern CPU Architectures"
//! by Fuetterling et al.
//!
//! Specifically, the table-based traversal order approach they
//! propose is largely followed by this implementation.

#![allow(dead_code)]

use mem_arena::MemArena;

use algorithm::partition;
use bbox::BBox;
use bbox4::BBox4;
use boundable::Boundable;
use lerp::lerp_slice;
use ray::AccelRay;

use super::bvh_base::{BVHBase, BVHBaseNode, BVH_MAX_DEPTH};

// TRAVERSAL_TABLE
include!("bvh4_table.inc");

// Calculates the traversal code for a BVH4 node based on the splits and topology
// of its children.
//
// split_1 is the top split.
//
// split_2 is either the left or right split depending on topology, and is only
// relevant for topologies 0-2.  For topology 3 it should be 0.
//
// split_3 is always the right split, and is only relevant for topology 0. For
// topologies 1-3 it should be 0.
//
// topology can be 0-3:
//     0: All three splits exist, representing 4 BVH4 children.
//     1: Two splits exist: top split and left split, representing 3 BVH4 children.
//     2: Two splits exist: top split and right split, representing 3 BVH4 children.
//     3: Only the top split exists, representing 2 BVH4 children.
fn calc_traversal_code(split_1: u8, split_2: u8, split_3: u8, topology: u8) -> u8 {
    debug_assert!(!(topology > 0 && split_3 > 0));
    debug_assert!(!(topology > 2 && split_2 > 0));

    static T_TABLE: [u8; 4] = [0, 27, 27 + 9, 27 + 9 + 9];
    split_1 + (split_2 * 3) + (split_3 * 9) + T_TABLE[topology as usize]
}

#[derive(Copy, Clone, Debug)]
pub struct BVH4<'a> {
    root: Option<&'a BVH4Node<'a>>,
    depth: usize,
}

#[derive(Copy, Clone, Debug)]
enum BVH4Node<'a> {
    // Internal {
    //     bounds: &'a [BBox4],
    //     children: [&'a BVH4Node<'a>; 4],
    //     children_count: u8,
    //     traversal_code: u8,
    // },
    Internal {
        bounds: &'a [BBox],
        children: (&'a BVH4Node<'a>, &'a BVH4Node<'a>),
        split_axis: u8,
    },

    Leaf {
        bounds: &'a [BBox],
        object_range: (usize, usize),
    },
}

impl<'a> BVH4<'a> {
    pub fn from_objects<'b, T, F>(arena: &'a MemArena,
                                  objects: &mut [T],
                                  objects_per_leaf: usize,
                                  bounder: F)
                                  -> BVH4<'a>
        where F: 'b + Fn(&T) -> &'b [BBox]
    {
        if objects.len() == 0 {
            BVH4 {
                root: None,
                depth: 0,
            }
        } else {
            let base = BVHBase::from_objects(objects, objects_per_leaf, bounder);

            BVH4 {
                root: Some(BVH4::construct_from_base(arena, &base, base.root_node_index())),
                depth: base.depth,
            }
        }
    }

    pub fn tree_depth(&self) -> usize {
        self.depth
    }

    pub fn traverse<T, F>(&self, rays: &mut [AccelRay], objects: &[T], mut obj_ray_test: F)
        where F: FnMut(&T, &mut [AccelRay])
    {
        match self.root {
            None => {}

            Some(root) => {
                // +2 of max depth for root and last child
                let mut node_stack = [root; BVH_MAX_DEPTH + 2];
                let mut ray_i_stack = [rays.len(); BVH_MAX_DEPTH + 2];
                let mut stack_ptr = 1;

                while stack_ptr > 0 {
                    match node_stack[stack_ptr] {
                        &BVH4Node::Internal { bounds, children, split_axis } => {
                            let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
                                (!r.is_done()) && lerp_slice(bounds, r.time).intersect_accel_ray(r)
                            });
                            if part > 0 {
                                node_stack[stack_ptr] = children.0;
                                node_stack[stack_ptr + 1] = children.1;
                                ray_i_stack[stack_ptr] = part;
                                ray_i_stack[stack_ptr + 1] = part;
                                if rays[0].dir_inv.get_n(split_axis as usize).is_sign_positive() {
                                    node_stack.swap(stack_ptr, stack_ptr + 1);
                                }
                                stack_ptr += 1;
                            } else {
                                stack_ptr -= 1;
                            }
                        }

                        &BVH4Node::Leaf { bounds, object_range } => {
                            let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
                                (!r.is_done()) && lerp_slice(bounds, r.time).intersect_accel_ray(r)
                            });
                            if part > 0 {
                                for obj in &objects[object_range.0..object_range.1] {
                                    obj_ray_test(obj, &mut rays[..part]);
                                }
                            }

                            stack_ptr -= 1;
                        }
                    }
                }
            }
        }
    }

    fn construct_from_base(arena: &'a MemArena,
                           base: &BVHBase,
                           node_index: usize)
                           -> &'a mut BVH4Node<'a> {
        match &base.nodes[node_index] {
            &BVHBaseNode::Internal { bounds_range, children_indices, split_axis } => {
                let mut node = unsafe { arena.alloc_uninitialized::<BVH4Node>() };

                let bounds = arena.copy_slice(&base.bounds[bounds_range.0..bounds_range.1]);
                let child1 = BVH4::construct_from_base(arena, base, children_indices.0);
                let child2 = BVH4::construct_from_base(arena, base, children_indices.1);

                *node = BVH4Node::Internal {
                    bounds: bounds,
                    children: (child1, child2),
                    split_axis: split_axis,
                };

                return node;
            }

            &BVHBaseNode::Leaf { bounds_range, object_range } => {
                let mut node = unsafe { arena.alloc_uninitialized::<BVH4Node>() };
                let bounds = arena.copy_slice(&base.bounds[bounds_range.0..bounds_range.1]);

                *node = BVH4Node::Leaf {
                    bounds: bounds,
                    object_range: object_range,
                };

                return node;
            }
        }
    }
}

lazy_static! {
    static ref DEGENERATE_BOUNDS: [BBox; 1] = [BBox::new()];
}

impl<'a> Boundable for BVH4<'a> {
    fn bounds<'b>(&'b self) -> &'b [BBox] {
        match self.root {
            None => &DEGENERATE_BOUNDS[..],
            Some(root) => {
                match root {
                    &BVH4Node::Internal { bounds, .. } => bounds,

                    &BVH4Node::Leaf { bounds, .. } => bounds,
                }
            }
        }
    }
}
