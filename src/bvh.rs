#![allow(dead_code)]

use bbox::BBox;
use ray::Ray;
use algorithm::partition;

#[derive(Debug)]
pub struct BVH {
    nodes: Vec<BVHNode>,
    depth: usize,
}

#[derive(Debug)]
enum BVHNode {
    Internal {
        bounds: BBox,
        second_child_index: usize,
        split_axis: u8,
    },

    Leaf {
        bounds: BBox,
        object_range: (usize, usize),
    },
}

impl BVH {
    pub fn from_objects<T, F>(objects: &mut [T], objects_per_leaf: usize, bounder: F) -> BVH
        where F: Fn(&T) -> BBox
    {
        let mut bvh = BVH {
            nodes: Vec::new(),
            depth: 0,
        };

        bvh.recursive_build(0, 0, objects_per_leaf, objects, &bounder);

        println!("BVH Depth: {}", bvh.depth);

        bvh
    }


    fn recursive_build<T, F>(&mut self,
                             offset: usize,
                             depth: usize,
                             objects_per_leaf: usize,
                             objects: &mut [T],
                             bounder: &F)
                             -> usize
        where F: Fn(&T) -> BBox
    {
        let me = self.nodes.len();

        if objects.len() == 0 {
            return 0;
        } else if objects.len() <= objects_per_leaf {
            // Leaf node
            self.nodes.push(BVHNode::Leaf {
                bounds: {
                    let mut bounds = bounder(&objects[0]);
                    for obj in &objects[1..] {
                        bounds = bounds | bounder(obj);
                    }
                    bounds
                },
                object_range: (offset, offset + objects.len()),
            });

            if self.depth < depth {
                self.depth = depth;
            }
        } else {
            // Not a leaf node
            self.nodes.push(BVHNode::Internal {
                bounds: BBox::new(),
                second_child_index: 0,
                split_axis: 0,
            });

            // Determine which axis to split on
            let bounds = {
                let mut bounds = BBox::new();
                for obj in objects.iter() {
                    bounds = bounds | bounder(obj);
                }
                bounds
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
                let mut split_i = partition(objects, |obj| {
                    let tb = bounder(obj);
                    let centroid = (tb.min[split_axis] + tb.max[split_axis]) * 0.5;
                    centroid < split_pos
                });
                if split_i < 1 {
                    split_i = 1;
                }

                split_i
            };

            // Create child nodes
            self.recursive_build(offset,
                                 depth + 1,
                                 objects_per_leaf,
                                 &mut objects[..split_index],
                                 bounder);
            let child2_index = self.recursive_build(offset + split_index,
                                                    depth + 1,
                                                    objects_per_leaf,
                                                    &mut objects[split_index..],
                                                    bounder);

            // Set node
            self.nodes[me] = BVHNode::Internal {
                bounds: bounds,
                second_child_index: child2_index,
                split_axis: split_axis as u8,
            };
        }

        return me;
    }


    pub fn traverse<T, F>(&self, rays: &mut [Ray], objects: &[T], mut obj_ray_test: F)
        where F: FnMut(&T, &mut [Ray])
    {
        let mut i_stack = [0; 65];
        let mut ray_i_stack = [rays.len(); 65];
        let mut stack_ptr = 1;

        while stack_ptr > 0 {
            match self.nodes[i_stack[stack_ptr]] {
                BVHNode::Internal { bounds, second_child_index, split_axis } => {
                    let part = partition(&mut rays[..ray_i_stack[stack_ptr]],
                                         |r| bounds.intersect_ray(r));
                    if part > 0 {
                        i_stack[stack_ptr] += 1;
                        i_stack[stack_ptr + 1] = second_child_index;
                        ray_i_stack[stack_ptr] = part;
                        ray_i_stack[stack_ptr + 1] = part;
                        if rays[0].dir[split_axis as usize] > 0.0 {
                            i_stack.swap(stack_ptr, stack_ptr + 1);
                        }
                        stack_ptr += 1;
                    } else {
                        stack_ptr -= 1;
                    }
                }

                BVHNode::Leaf { bounds, object_range } => {
                    let part = partition(&mut rays[..ray_i_stack[stack_ptr]],
                                         |r| bounds.intersect_ray(r));
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
