#![allow(dead_code)]

use mem_arena::MemArena;

use algorithm::{partition, merge_slices_append};
use bbox::BBox;
use boundable::Boundable;
use lerp::lerp_slice;
use math::log2_64;
use ray::AccelRay;

use super::objects_split::{sah_split, median_split};


const BVH_MAX_DEPTH: usize = 64;

#[derive(Copy, Clone, Debug)]
pub struct BVH<'a> {
    nodes: &'a [BVHNode],
    bounds: &'a [BBox],
    depth: usize,
}

#[derive(Copy, Clone, Debug)]
enum BVHNode {
    Internal {
        bounds_range: (usize, usize),
        second_child_index: usize,
        split_axis: u8,
    },

    Leaf {
        bounds_range: (usize, usize),
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
        let mut builder = BVHBuilder::new();

        builder.recursive_build(0, 0, objects_per_leaf, objects, &bounder);

        BVH {
            nodes: arena.copy_slice(&builder.nodes),
            bounds: arena.copy_slice(&builder.bounds),
            depth: builder.depth,
        }
    }

    pub fn tree_depth(&self) -> usize {
        self.depth
    }

    pub fn traverse<T, F>(&self, rays: &mut [AccelRay], objects: &[T], mut obj_ray_test: F)
        where F: FnMut(&T, &mut [AccelRay])
    {
        if self.nodes.len() == 0 {
            return;
        }

        // +2 of max depth for root and last child
        let mut i_stack = [0; BVH_MAX_DEPTH + 2];
        let mut ray_i_stack = [rays.len(); BVH_MAX_DEPTH + 2];
        let mut stack_ptr = 1;

        while stack_ptr > 0 {
            match self.nodes[i_stack[stack_ptr]] {
                BVHNode::Internal { bounds_range: br, second_child_index, split_axis } => {
                    let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
                        (!r.is_done()) &&
                        lerp_slice(&self.bounds[br.0..br.1], r.time).intersect_accel_ray(r)
                    });
                    if part > 0 {
                        i_stack[stack_ptr] += 1;
                        i_stack[stack_ptr + 1] = second_child_index;
                        ray_i_stack[stack_ptr] = part;
                        ray_i_stack[stack_ptr + 1] = part;
                        if rays[0].dir_inv.get_n(split_axis as usize).is_sign_positive() {
                            i_stack.swap(stack_ptr, stack_ptr + 1);
                        }
                        stack_ptr += 1;
                    } else {
                        stack_ptr -= 1;
                    }
                }

                BVHNode::Leaf { bounds_range: br, object_range } => {
                    let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
                        (!r.is_done()) &&
                        lerp_slice(&self.bounds[br.0..br.1], r.time).intersect_accel_ray(r)
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

impl<'a> Boundable for BVH<'a> {
    fn bounds<'b>(&'b self) -> &'b [BBox] {
        match self.nodes[0] {
            BVHNode::Internal { bounds_range, .. } => &self.bounds[bounds_range.0..bounds_range.1],

            BVHNode::Leaf { bounds_range, .. } => &self.bounds[bounds_range.0..bounds_range.1],
        }
    }
}


#[derive(Debug)]
struct BVHBuilder {
    nodes: Vec<BVHNode>,
    bounds: Vec<BBox>,
    depth: usize,
    bounds_cache: Vec<BBox>,
}

impl BVHBuilder {
    fn new() -> BVHBuilder {
        BVHBuilder {
            nodes: Vec::new(),
            bounds: Vec::new(),
            depth: 0,
            bounds_cache: Vec::new(),
        }
    }

    fn acc_bounds<'a, T, F>(&mut self, objects: &mut [T], bounder: &F)
        where F: 'a + Fn(&T) -> &'a [BBox]
    {
        // TODO: do all of this without the temporary cache
        let max_len = objects.iter().map(|obj| bounder(obj).len()).max().unwrap();

        self.bounds_cache.clear();
        self.bounds_cache.resize(max_len, BBox::new());

        for obj in objects.iter() {
            let bounds = bounder(obj);
            debug_assert!(bounds.len() > 0);
            if bounds.len() == max_len {
                for i in 0..bounds.len() {
                    self.bounds_cache[i] |= bounds[i];
                }
            } else {
                let s = (max_len - 1) as f32;
                for (i, bbc) in self.bounds_cache.iter_mut().enumerate() {
                    *bbc |= lerp_slice(bounds, i as f32 / s);
                }
            }
        }
    }

    fn recursive_build<'a, T, F>(&mut self,
                                 offset: usize,
                                 depth: usize,
                                 objects_per_leaf: usize,
                                 objects: &mut [T],
                                 bounder: &F)
                                 -> (usize, (usize, usize))
        where F: 'a + Fn(&T) -> &'a [BBox]
    {
        let me = self.nodes.len();

        if objects.len() == 0 {
            return (0, (0, 0));
        } else if objects.len() <= objects_per_leaf {
            // Leaf node
            self.acc_bounds(objects, bounder);
            let bi = self.bounds.len();
            for b in self.bounds_cache.iter() {
                self.bounds.push(*b);
            }
            self.nodes.push(BVHNode::Leaf {
                bounds_range: (bi, self.bounds.len()),
                object_range: (offset, offset + objects.len()),
            });

            if self.depth < depth {
                self.depth = depth;
            }

            return (me, (bi, self.bounds.len()));
        } else {
            // Not a leaf node
            self.nodes.push(BVHNode::Internal {
                bounds_range: (0, 0),
                second_child_index: 0,
                split_axis: 0,
            });

            // Partition objects.
            // If we're too near the max depth, we do balanced building to
            // avoid exceeding max depth.
            // Otherwise we do SAH splitting to build better trees.
            let (split_index, split_axis) = if (log2_64(objects.len() as u64) as usize) <
                                               (BVH_MAX_DEPTH - depth) {
                // SAH splitting, when we have room to play
                sah_split(objects, &bounder)
            } else {
                // Balanced splitting, when we don't have room to play
                median_split(objects, &bounder)
            };

            // Create child nodes
            let (_, c1_bounds) = self.recursive_build(offset,
                                                      depth + 1,
                                                      objects_per_leaf,
                                                      &mut objects[..split_index],
                                                      bounder);
            let (c2_index, c2_bounds) = self.recursive_build(offset + split_index,
                                                             depth + 1,
                                                             objects_per_leaf,
                                                             &mut objects[split_index..],
                                                             bounder);

            // Determine bounds
            // TODO: do merging without the temporary vec.
            let bi = self.bounds.len();
            let mut merged = Vec::new();
            merge_slices_append(&self.bounds[c1_bounds.0..c1_bounds.1],
                                &self.bounds[c2_bounds.0..c2_bounds.1],
                                &mut merged,
                                |b1, b2| *b1 | *b2);
            self.bounds.extend(merged.drain(0..));

            // Set node
            self.nodes[me] = BVHNode::Internal {
                bounds_range: (bi, self.bounds.len()),
                second_child_index: c2_index,
                split_axis: split_axis as u8,
            };

            return (me, (bi, self.bounds.len()));
        }
    }
}
