#![allow(dead_code)]

use lerp::lerp_slice;
use bbox::BBox;
use boundable::Boundable;
use ray::{Ray, AccelRay};
use algorithm::partition;

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

    fn acc_bounds<'a, T, F>(&mut self, objects1: &mut [T], bounder: &F)
        where F: 'a + Fn(&T) -> &'a [BBox]
    {
        // TODO: merging of different length bounds
        self.bounds_cache.clear();
        for bb in bounder(&objects1[0]).iter() {
            self.bounds_cache.push(*bb);
        }
        for obj in &objects1[1..] {
            let bounds = bounder(obj);
            debug_assert!(self.bounds_cache.len() == bounds.len());
            for i in 0..bounds.len() {
                self.bounds_cache[i] = self.bounds_cache[i] | bounds[i];
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

            // Determine which axis to split on
            let bounds = {
                let mut bb = BBox::new();
                for obj in &objects[..] {
                    bb = bb | lerp_slice(bounder(obj), 0.5);
                }
                bb
            };
            let split_axis = {
                let x_ext = bounds.max[0] - bounds.min[0];
                let y_ext = bounds.max[1] - bounds.min[1];
                let z_ext = bounds.max[2] - bounds.min[2];
                if x_ext > y_ext && x_ext > z_ext {
                    0
                } else if y_ext > z_ext {
                    1
                } else {
                    2
                }
            };
            let split_pos = (bounds.min[split_axis] + bounds.max[split_axis]) * 0.5;

            // Partition objects based on split
            let split_index = {
                let mut split_i = partition(&mut objects[..], |obj| {
                    let tb = lerp_slice(bounder(obj), 0.5);
                    let centroid = (tb.min[split_axis] + tb.max[split_axis]) * 0.5;
                    centroid < split_pos
                });
                if split_i < 1 {
                    split_i = 1;
                }

                split_i
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
            // TODO: merging of different length bounds
            let bi = self.bounds.len();
            for (i1, i2) in Iterator::zip(c1_bounds.0..c1_bounds.1, c2_bounds.0..c2_bounds.1) {
                let bb = self.bounds[i1] | self.bounds[i2];
                self.bounds.push(bb);
            }

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

        let mut i_stack = [0; 65];
        let mut ray_i_stack = [rays.len(); 65];
        let mut stack_ptr = 1;

        while stack_ptr > 0 {
            match self.nodes[i_stack[stack_ptr]] {
                BVHNode::Internal { bounds_range: br, second_child_index, split_axis } => {
                    let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
                        lerp_slice(&self.bounds[br.0..br.1], r.time).intersect_accel_ray(r)
                    });
                    if part > 0 {
                        i_stack[stack_ptr] += 1;
                        i_stack[stack_ptr + 1] = second_child_index;
                        ray_i_stack[stack_ptr] = part;
                        ray_i_stack[stack_ptr + 1] = part;
                        if rays[0].dir_inv[split_axis as usize].is_sign_positive() {
                            i_stack.swap(stack_ptr, stack_ptr + 1);
                        }
                        stack_ptr += 1;
                    } else {
                        stack_ptr -= 1;
                    }
                }

                BVHNode::Leaf { bounds_range: br, object_range } => {
                    let part = partition(&mut rays[..ray_i_stack[stack_ptr]], |r| {
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
