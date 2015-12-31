#![allow(dead_code)]

use bbox::BBox;
use math::Point;
use ray::Ray;
use triangle;
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


pub fn intersect_bvh(bvh: &BVH<(Point, Point, Point)>, ray: &mut Ray) -> Option<(f32, f32, f32)> {
    if bvh.nodes.len() == 0 {
        return None;
    }

    let mut i_stack = [0; 65];
    let mut stack_ptr: usize = 1;
    let mut hit = false;
    let mut u = 0.0;
    let mut v = 0.0;

    while stack_ptr > 0 {
        match bvh.nodes[i_stack[stack_ptr]] {
            BVHNode::Internal { bounds, second_child_index } => {
                if bounds.intersect_ray(ray) {
                    i_stack[stack_ptr] += 1;
                    i_stack[stack_ptr + 1] = second_child_index;
                    stack_ptr += 1;
                } else {
                    stack_ptr -= 1;
                }
            }

            BVHNode::Leaf { bounds: _, object_index } => {
                if let Some((t, tri_u, tri_v)) =
                       triangle::intersect_ray(ray, bvh.objects[object_index]) {
                    if t < ray.max_t {
                        hit = true;
                        ray.max_t = t;
                        u = tri_u;
                        v = tri_v;
                    }
                }

                stack_ptr -= 1;
            }
        }
    }

    if hit {
        return Some((ray.max_t, u, v));
    } else {
        return None;
    }
}
