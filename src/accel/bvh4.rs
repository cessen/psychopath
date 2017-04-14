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

#[derive(Copy, Clone, Debug)]
pub struct BVH4<'a> {
    root: Option<&'a BVH4Node<'a>>,
    depth: usize,
    _bounds: Option<&'a [BBox]>,
}

#[derive(Copy, Clone, Debug)]
pub enum BVH4Node<'a> {
    Internal {
        bounds: &'a [BBox4],
        children: &'a [BVH4Node<'a>],
        traversal_code: u8,
    },

    Leaf { object_range: (usize, usize) },
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
                _bounds: None,
            }
        } else {
            let base = BVHBase::from_objects(objects, objects_per_leaf, bounder);

            let mut fill_node = unsafe { arena.alloc_uninitialized_with_alignment::<BVH4Node>(32) };
            BVH4::construct_from_base(arena, &base, &base.nodes[base.root_node_index()], fill_node);

            BVH4 {
                root: Some(fill_node),
                depth: (base.depth / 2) + 1,
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

    pub fn traverse<T, F>(&self, rays: &mut [AccelRay], objects: &[T], mut obj_ray_test: F)
        where F: FnMut(&T, &mut [AccelRay])
    {
        match self.root {
            None => {}

            Some(root) => {
                // +2 of max depth for root and last child
                let mut node_stack = [Some(root); BVH_MAX_DEPTH + 2];
                let mut ray_i_stack = [rays.len(); BVH_MAX_DEPTH + 2];
                let mut stack_ptr = 1;
                let mut first_loop = true;

                let ray_code = (rays[0].dir_inv.x().is_sign_negative() as u8) |
                               ((rays[0].dir_inv.y().is_sign_negative() as u8) << 1) |
                               ((rays[0].dir_inv.z().is_sign_negative() as u8) << 2);

                while stack_ptr > 0 {
                    match node_stack[stack_ptr] {
                        Some(&BVH4Node::Internal { bounds, children, traversal_code }) => {
                            let node_order_code = {
                                TRAVERSAL_TABLE[ray_code as usize][traversal_code as usize]
                            };

                            // Ray testing
                            let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
                                if (!r.is_done()) && (first_loop || r.trav_stack.pop()) {
                                    let hits = lerp_slice(bounds, r.time)
                                        .intersect_accel_ray(r)
                                        .to_bitmask();

                                    if hits != 0 {
                                        // Push hit bits onto ray's traversal stack
                                        let mut shuffled_hits = 0;
                                        for i in 0..children.len() {
                                            let ii = (node_order_code >> (i * 2)) & 3;
                                            shuffled_hits |= ((hits >> ii) & 1) << i;
                                        }
                                        r.trav_stack.push_n(shuffled_hits, children.len() as u8);

                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            });

                            // Update stack based on ray testing results
                            if part > 0 {
                                for i in 0..children.len() {
                                    let inv_i = (children.len() - 1) - i;
                                    node_stack[stack_ptr + i] =
                                        Some(&children[((node_order_code >> (inv_i * 2)) & 3) as
                                              usize]);
                                    ray_i_stack[stack_ptr + i] = part;
                                }

                                stack_ptr += children.len() - 1;
                            } else {
                                stack_ptr -= 1;
                            }
                        }

                        Some(&BVH4Node::Leaf { object_range }) => {
                            let part = if !first_loop {
                                partition(&mut rays[..ray_i_stack[stack_ptr]],
                                          |r| r.trav_stack.pop())
                            } else {
                                ray_i_stack[stack_ptr]
                            };

                            for obj in &objects[object_range.0..object_range.1] {
                                obj_ray_test(obj, &mut rays[..part]);
                            }

                            stack_ptr -= 1;
                        }

                        None => {
                            if !first_loop {
                                for r in (&mut rays[..ray_i_stack[stack_ptr]]).iter_mut() {
                                    r.trav_stack.pop();
                                }
                            }
                            stack_ptr -= 1;
                        }
                    }

                    first_loop = false;
                }
            }
        }
    }

    fn construct_from_base(arena: &'a MemArena,
                           base: &BVHBase,
                           node: &BVHBaseNode,
                           fill_node: &mut BVH4Node<'a>) {
        match node {
            // Create internal node
            &BVHBaseNode::Internal { bounds_range: _, children_indices, split_axis } => {
                let child_l = &base.nodes[children_indices.0];
                let child_r = &base.nodes[children_indices.1];

                // Prepare convenient access to the stuff we need.
                let child_count;
                let children; // [Optional, Optional, Optional, Optional]
                let split_axis_l; // Optional
                let split_axis_r; // Optional
                match child_l {
                    &BVHBaseNode::Internal { children_indices: i_l, split_axis: s_l, .. } => {
                        match child_r {
                            &BVHBaseNode::Internal { children_indices: i_r,
                                                     split_axis: s_r,
                                                     .. } => {
                                // Four nodes
                                child_count = 4;
                                children = [Some(&base.nodes[i_l.0]),
                                            Some(&base.nodes[i_l.1]),
                                            Some(&base.nodes[i_r.0]),
                                            Some(&base.nodes[i_r.1])];
                                split_axis_l = Some(s_l);
                                split_axis_r = Some(s_r);
                            }
                            &BVHBaseNode::Leaf { .. } => {
                                // Three nodes with left split
                                child_count = 3;
                                children = [Some(&base.nodes[i_l.0]),
                                            Some(&base.nodes[i_l.1]),
                                            Some(child_r),
                                            None];
                                split_axis_l = Some(s_l);
                                split_axis_r = None;
                            }
                        }
                    }
                    &BVHBaseNode::Leaf { .. } => {
                        match child_r {
                            &BVHBaseNode::Internal { children_indices: i_r,
                                                     split_axis: s_r,
                                                     .. } => {
                                // Three nodes with right split
                                child_count = 3;
                                children = [Some(child_l),
                                            Some(&base.nodes[i_r.0]),
                                            Some(&base.nodes[i_r.1]),
                                            None];
                                split_axis_l = None;
                                split_axis_r = Some(s_r);
                            }
                            &BVHBaseNode::Leaf { .. } => {
                                // Two nodes
                                child_count = 2;
                                children = [Some(child_l), Some(child_r), None, None];
                                split_axis_l = None;
                                split_axis_r = None;
                            }
                        }
                    }
                }

                // Construct bounds
                let bounds = {
                    let bounds_len = children.iter()
                        .map(|c| if let &Some(n) = c {
                            n.bounds_range().1 - n.bounds_range().0
                        } else {
                            0
                        })
                        .max()
                        .unwrap();
                    let mut bounds =
                        unsafe { arena.alloc_array_uninitialized_with_alignment(bounds_len, 32) };
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
                        *b = BBox4::from_bboxes(b1, b2, b3, b4);
                    }
                    bounds
                };

                // Construct child nodes
                let mut child_nodes =
                    unsafe {
                        arena.alloc_array_uninitialized_with_alignment::<BVH4Node>(child_count, 32)
                    };
                for (i, c) in children[0..child_count].iter().enumerate() {
                    BVH4::construct_from_base(arena, base, c.unwrap(), &mut child_nodes[i]);
                }

                // Build this node
                let traversal_code = {
                    let topology_code = if child_count == 4 {
                        0
                    } else if child_count == 2 {
                        3
                    } else if split_axis_l.is_some() {
                        1
                    } else {
                        2
                    };
                    calc_traversal_code(split_axis,
                                        split_axis_l.unwrap_or(split_axis_r.unwrap_or(0)),
                                        if child_count == 4 {
                                            split_axis_r.unwrap()
                                        } else {
                                            0
                                        },
                                        topology_code)
                };
                *fill_node = BVH4Node::Internal {
                    bounds: bounds,
                    children: child_nodes,
                    traversal_code: traversal_code,
                };
            }

            // Create internal node
            &BVHBaseNode::Leaf { object_range, .. } => {
                *fill_node = BVH4Node::Leaf { object_range: object_range };
            }
        }
    }
}


impl<'a> Boundable for BVH4<'a> {
    fn bounds<'b>(&'b self) -> &'b [BBox] {
        self._bounds.unwrap_or(&[])
    }
}


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
