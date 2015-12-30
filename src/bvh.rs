#![allow(dead_code)]

use bbox::BBox;
use math::Point;
use ray::Ray;
use triangle;
use algorithm::partition;

#[derive(Debug)]
pub struct BVH {
    nodes: Vec<BVHNode>,
}

#[derive(Debug)]
enum BVHNode {
    Internal {
        bounds: BBox,
        second_child_index: usize,
    },

    Leaf {
        bounds: BBox,
        triangle: (Point, Point, Point),
    },
}

impl BVH {
    pub fn from_triangles(triangles: &mut [(Point, Point, Point)]) -> BVH {
        let mut bvh = BVH { nodes: Vec::new() };

        bvh.recursive_build(triangles);

        bvh
    }

    // Recursively builds the BVH starting at the given node with the given
    // first and last primitive indices (in bag).
    fn recursive_build(&mut self, triangles: &mut [(Point, Point, Point)]) -> usize {
        let me = self.nodes.len();

        if triangles.len() == 1 {
            // Leaf node
            let tri = triangles[0];

            self.nodes.push(BVHNode::Leaf {
                bounds: {
                    let minimum = tri.0.min(tri.1.min(tri.2));
                    let maximum = tri.0.max(tri.1.max(tri.2));
                    BBox::from_points(minimum, maximum)
                },
                triangle: tri,
            });
        } else {
            // Not a leaf node
            self.nodes.push(BVHNode::Internal {
                bounds: BBox::new(),
                second_child_index: 0,
            });

            // Determine which axis to split on
            fn tri_bounds(tri: (Point, Point, Point)) -> BBox {
                let minimum = tri.0.min(tri.1.min(tri.2));
                let maximum = tri.0.max(tri.1.max(tri.2));
                BBox {
                    min: minimum,
                    max: maximum,
                }
            }
            let bounds = {
                let mut bounds = BBox::new();
                for tri in &triangles[..] {
                    bounds = bounds | tri_bounds(*tri);
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

            // Partition triangles based on split
            let split_index = {
                let mut split_i = partition(triangles, |tri| {
                    let tb = tri_bounds(*tri);
                    let centroid = (tb.min[split_axis] + tb.max[split_axis]) * 0.5;
                    centroid < split_pos
                });
                if split_i < 1 {
                    split_i = 1;
                }

                split_i
            };

            // Create child nodes
            self.recursive_build(&mut triangles[..split_index]);
            let child2_index = self.recursive_build(&mut triangles[split_index..]);

            // Set node
            self.nodes[me] = BVHNode::Internal {
                bounds: bounds,
                second_child_index: child2_index,
            };
        }

        return me;
    }
}


pub fn intersect_bvh(bvh: &BVH, ray: &Ray) -> bool {
    let mut i_stack = [0; 64];
    let mut stack_ptr = 0;

    loop {
        match bvh.nodes[i_stack[stack_ptr]] {
            BVHNode::Internal { bounds, second_child_index } => {
                if bounds.intersect_ray(ray) {
                    i_stack[stack_ptr] += 1;
                    i_stack[stack_ptr + 1] = second_child_index;
                    stack_ptr += 1;
                } else {
                    if stack_ptr == 0 {
                        break;
                    }
                    stack_ptr -= 1;
                }
            }

            BVHNode::Leaf{bounds: _, triangle: tri} => {
                if let Some(_) = triangle::intersect_ray(ray, tri) {
                    return true;
                } else {
                    if stack_ptr == 0 {
                        break;
                    }
                    stack_ptr -= 1;
                }
            }
        }
    }

    return false;
}
