#![allow(dead_code)]

use mem_arena::MemArena;

use algorithm::partition;
use bbox::BBox;
use boundable::Boundable;
use lerp::lerp_slice;
use ray::AccelRay;
use timer::Timer;

use super::bvh_base::{BVHBase, BVHBaseNode, BVH_MAX_DEPTH};
use super::ACCEL_TRAV_TIME;


#[derive(Copy, Clone, Debug)]
pub struct BVH<'a> {
    root: Option<&'a BVHNode<'a>>,
    depth: usize,
}

#[derive(Copy, Clone, Debug)]
pub enum BVHNode<'a> {
    Internal {
        bounds: &'a [BBox],
        children: (&'a BVHNode<'a>, &'a BVHNode<'a>),
        split_axis: u8,
    },

    Leaf {
        bounds: &'a [BBox],
        object_range: (usize, usize),
    },
}

impl<'a> BVH<'a> {
    pub fn from_objects<'b, T, F>(arena: &'a MemArena,
                                  objects: &mut [T],
                                  objects_per_leaf: usize,
                                  bounder: F)
                                  -> BVH<'a>
        where F: 'b + Fn(&T) -> &'b [BBox]
    {
        if objects.len() == 0 {
            BVH {
                root: None,
                depth: 0,
            }
        } else {
            let base = BVHBase::from_objects(objects, objects_per_leaf, bounder);

            BVH {
                root: Some(BVH::construct_from_base(arena, &base, base.root_node_index())),
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
        if self.root.is_none() {
            return;
        }

        let mut trav_time: f64 = 0.0;
        let mut timer = Timer::new();

        // +2 of max depth for root and last child
        let mut node_stack = [self.root.unwrap(); BVH_MAX_DEPTH + 2];
        let mut ray_i_stack = [rays.len(); BVH_MAX_DEPTH + 2];
        let mut stack_ptr = 1;

        while stack_ptr > 0 {
            match node_stack[stack_ptr] {
                &BVHNode::Internal { bounds, children, split_axis } => {
                    let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
                        (!r.is_done()) && lerp_slice(bounds, r.time).intersect_accel_ray(r)
                    });
                    if part > 0 {
                        node_stack[stack_ptr] = children.0;
                        node_stack[stack_ptr + 1] = children.1;
                        ray_i_stack[stack_ptr] = part;
                        ray_i_stack[stack_ptr + 1] = part;
                        if rays[0].dir_inv.get_n(split_axis as usize) >= 0.0 {
                            node_stack.swap(stack_ptr, stack_ptr + 1);
                        }
                        stack_ptr += 1;
                    } else {
                        stack_ptr -= 1;
                    }
                }

                &BVHNode::Leaf { bounds, object_range } => {
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
    }

    fn construct_from_base(arena: &'a MemArena,
                           base: &BVHBase,
                           node_index: usize)
                           -> &'a mut BVHNode<'a> {
        match &base.nodes[node_index] {
            &BVHBaseNode::Internal { bounds_range, children_indices, split_axis } => {
                let mut node = unsafe { arena.alloc_uninitialized_with_alignment::<BVHNode>(32) };

                let bounds =
                    arena.copy_slice_with_alignment(&base.bounds[bounds_range.0..bounds_range.1],
                                                    32);
                let child1 = BVH::construct_from_base(arena, base, children_indices.0);
                let child2 = BVH::construct_from_base(arena, base, children_indices.1);

                *node = BVHNode::Internal {
                    bounds: bounds,
                    children: (child1, child2),
                    split_axis: split_axis,
                };

                return node;
            }

            &BVHBaseNode::Leaf { bounds_range, object_range } => {
                let mut node = unsafe { arena.alloc_uninitialized::<BVHNode>() };
                let bounds = arena.copy_slice(&base.bounds[bounds_range.0..bounds_range.1]);

                *node = BVHNode::Leaf {
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

impl<'a> Boundable for BVH<'a> {
    fn bounds<'b>(&'b self) -> &'b [BBox] {
        match self.root {
            None => &DEGENERATE_BOUNDS[..],
            Some(root) => {
                match root {
                    &BVHNode::Internal { bounds, .. } => bounds,

                    &BVHNode::Leaf { bounds, .. } => bounds,
                }
            }
        }
    }
}
