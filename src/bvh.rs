#![allow(dead_code)]

use std::marker;
use std::slice;

use bbox::BBox;
use ray::Ray;
use algorithm::partition;

#[derive(Debug)]
pub struct BVH<'a, T: 'a> {
    nodes: Vec<BVHNode>,
    objects: &'a [T],
    depth: usize,
}

#[derive(Debug)]
enum BVHNode {
    Internal {
        bounds: BBox,
        second_child_index: usize,
    },

    Leaf {
        bounds: BBox,
        object_index: usize,
    },
}

impl<'a, T> BVH<'a, T> {
    pub fn from_objects<F>(objects: &'a mut [T], bounder: F) -> BVH<'a, T>
        where F: Fn(&T) -> BBox
    {
        let mut bvh = BVH {
            nodes: Vec::new(),
            objects: &[],
            depth: 0,
        };

        bvh.recursive_build(0, 0, objects, &bounder);
        bvh.objects = objects;

        println!("BVH Depth: {}", bvh.depth);

        bvh
    }


    fn recursive_build<F>(&mut self,
                          offset: usize,
                          depth: usize,
                          objects: &mut [T],
                          bounder: &F)
                          -> usize
        where F: Fn(&T) -> BBox
    {
        let me = self.nodes.len();

        if objects.len() == 0 {
            return 0;
        } else if objects.len() == 1 {
            // Leaf node
            self.nodes.push(BVHNode::Leaf {
                bounds: bounder(&objects[0]),
                object_index: offset,
            });

            if self.depth < depth {
                self.depth = depth;
            }
        } else {
            // Not a leaf node
            self.nodes.push(BVHNode::Internal {
                bounds: BBox::new(),
                second_child_index: 0,
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
            self.recursive_build(offset, depth + 1, &mut objects[..split_index], bounder);
            let child2_index = self.recursive_build(offset + split_index,
                                                    depth + 1,
                                                    &mut objects[split_index..],
                                                    bounder);

            // Set node
            self.nodes[me] = BVHNode::Internal {
                bounds: bounds,
                second_child_index: child2_index,
            };
        }

        return me;
    }
}


pub struct BVHTraverser<'a, T: 'a> {
    bvh: &'a BVH<'a, T>,
    rays: (*mut Ray, usize),
    _ray_marker: marker::PhantomData<&'a mut [Ray]>,
    i_stack: [usize; 65],
    ray_i_stack: [usize; 65],
    stack_ptr: usize,
}

impl<'a, T> BVHTraverser<'a, T> {
    pub fn from_bvh_and_ray(bvh: &'a BVH<'a, T>, rays: &'a mut [Ray]) -> BVHTraverser<'a, T> {
        BVHTraverser {
            bvh: bvh,
            rays: (&mut rays[0] as *mut Ray, rays.len()),
            _ray_marker: marker::PhantomData,
            i_stack: [0; 65],
            ray_i_stack: [rays.len(); 65],
            stack_ptr: 1,
        }
    }
}

impl<'a, T> Iterator for BVHTraverser<'a, T> {
    type Item = (&'a T, &'a mut [Ray]);
    fn next(&mut self) -> Option<(&'a T, &'a mut [Ray])> {
        let rays = unsafe { slice::from_raw_parts_mut(self.rays.0, self.rays.1) };
        while self.stack_ptr > 0 {
            match self.bvh.nodes[self.i_stack[self.stack_ptr]] {
                BVHNode::Internal { bounds, second_child_index } => {
                    let part = partition(&mut rays[..self.ray_i_stack[self.stack_ptr]],
                                         |r| bounds.intersect_ray(r));
                    if part > 0 {
                        self.i_stack[self.stack_ptr] += 1;
                        self.i_stack[self.stack_ptr + 1] = second_child_index;
                        self.ray_i_stack[self.stack_ptr] = part;
                        self.ray_i_stack[self.stack_ptr + 1] = part;
                        self.stack_ptr += 1;
                    } else {
                        self.stack_ptr -= 1;
                    }
                }

                BVHNode::Leaf { bounds: _, object_index } => {
                    self.stack_ptr -= 1;
                    return Some((&self.bvh.objects[object_index],
                                 &mut rays[..self.ray_i_stack[self.stack_ptr + 1]]));
                }
            }
        }

        return None;
    }
}
