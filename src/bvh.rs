#![allow(dead_code)]

use std;
use std::cmp::Ordering;
use lerp::lerp_slice;
use bbox::BBox;
use boundable::Boundable;
use ray::AccelRay;
use algorithm::{partition, quick_select, merge_slices_append};
use math::log2_64;
use sah::sah_split;

const BVH_MAX_DEPTH: usize = 64;

#[derive(Debug)]
pub struct BVH {
    nodes: Vec<BVHNode>,
    bounds: Vec<BBox>,
    depth: usize,
    bounds_cache: Vec<BBox>,
}

#[derive(Debug)]
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

impl BVH {
    pub fn new_empty() -> BVH {
        BVH {
            nodes: Vec::new(),
            bounds: Vec::new(),
            depth: 0,
            bounds_cache: Vec::new(),
        }
    }

    pub fn from_objects<'a, T, F>(objects: &mut [T], objects_per_leaf: usize, bounder: F) -> BVH
        where F: 'a + Fn(&T) -> &'a [BBox]
    {
        let mut bvh = BVH::new_empty();

        bvh.recursive_build(0, 0, objects_per_leaf, objects, &bounder);
        bvh.bounds_cache.clear();
        bvh.bounds_cache.shrink_to_fit();

        bvh
    }

    pub fn tree_depth(&self) -> usize {
        self.depth
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

            // Get combined object bounds
            let bounds = {
                let mut bb = BBox::new();
                for obj in &objects[..] {
                    bb |= lerp_slice(bounder(obj), 0.5);
                }
                bb
            };

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
                let split_axis = {
                    let mut axis = 0;
                    let mut largest = std::f32::NEG_INFINITY;
                    for i in 0..3 {
                        let extent = bounds.max.get_n(i) - bounds.min.get_n(i);
                        if extent > largest {
                            largest = extent;
                            axis = i;
                        }
                    }
                    axis
                };

                let place = {
                    let place = objects.len() / 2;
                    if place > 0 {
                        place
                    } else {
                        1
                    }
                };
                quick_select(objects, place, |a, b| {
                    let tb_a = lerp_slice(bounder(a), 0.5);
                    let tb_b = lerp_slice(bounder(b), 0.5);
                    let centroid_a = (tb_a.min.get_n(split_axis) + tb_a.max.get_n(split_axis)) *
                                     0.5;
                    let centroid_b = (tb_b.min.get_n(split_axis) + tb_b.max.get_n(split_axis)) *
                                     0.5;

                    if centroid_a < centroid_b {
                        Ordering::Less
                    } else if centroid_a == centroid_b {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                });

                (place, split_axis)
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


impl Boundable for BVH {
    fn bounds<'a>(&'a self) -> &'a [BBox] {
        match self.nodes[0] {
            BVHNode::Internal { bounds_range, .. } => &self.bounds[bounds_range.0..bounds_range.1],

            BVHNode::Leaf { bounds_range, .. } => &self.bounds[bounds_range.0..bounds_range.1],
        }
    }
}
