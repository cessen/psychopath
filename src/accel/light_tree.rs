use algorithm::merge_slices_append;
use bbox::BBox;
use lerp::lerp_slice;
use math::{Vector, Point, Normal};
use shading::surface_closure::SurfaceClosure;

use super::LightAccel;
use super::objects_split::sah_split;


#[derive(Debug)]
pub struct LightTree {
    nodes: Vec<Node>,
    bounds: Vec<BBox>,
    depth: usize,
    bounds_cache: Vec<BBox>,
}

#[derive(Debug)]
struct Node {
    is_leaf: bool,
    bounds_range: (usize, usize),
    energy: f32,
    child_index: usize,
}

impl LightTree {
    pub fn from_objects<'a, T, F>(objects: &mut [T], info_getter: F) -> LightTree
        where F: 'a + Fn(&T) -> (&'a [BBox], f32)
    {
        let mut tree = LightTree {
            nodes: Vec::new(),
            bounds: Vec::new(),
            depth: 0,
            bounds_cache: Vec::new(),
        };

        tree.recursive_build(0, 0, objects, &info_getter);
        tree.bounds_cache.clear();
        tree.bounds_cache.shrink_to_fit();

        tree
    }


    fn recursive_build<'a, T, F>(&mut self,
                                 offset: usize,
                                 depth: usize,
                                 objects: &mut [T],
                                 info_getter: &F)
                                 -> (usize, (usize, usize))
        where F: 'a + Fn(&T) -> (&'a [BBox], f32)
    {
        let me_index = self.nodes.len();

        if objects.len() == 0 {
            return (0, (0, 0));
        } else if objects.len() == 1 {
            // Leaf node
            let bi = self.bounds.len();
            let (obj_bounds, energy) = info_getter(&objects[0]);
            self.bounds.extend(obj_bounds);
            self.nodes.push(Node {
                is_leaf: true,
                bounds_range: (bi, self.bounds.len()),
                energy: energy,
                child_index: offset,
            });

            if self.depth < depth {
                self.depth = depth;
            }

            return (me_index, (bi, self.bounds.len()));
        } else {
            // Not a leaf node
            self.nodes.push(Node {
                is_leaf: false,
                bounds_range: (0, 0),
                energy: 0.0,
                child_index: 0,
            });

            // Partition objects.
            let (split_index, _) = sah_split(objects, &|obj_ref| info_getter(obj_ref).0);

            // Create child nodes
            let (_, c1_bounds) =
                self.recursive_build(offset, depth + 1, &mut objects[..split_index], info_getter);
            let (c2_index, c2_bounds) = self.recursive_build(offset + split_index,
                                                             depth + 1,
                                                             &mut objects[split_index..],
                                                             info_getter);

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
            let energy = self.nodes[me_index + 1].energy + self.nodes[c2_index].energy;
            self.nodes[me_index] = Node {
                is_leaf: false,
                bounds_range: (bi, self.bounds.len()),
                energy: energy,
                child_index: c2_index,
            };

            return (me_index, (bi, self.bounds.len()));
        }
    }
}


impl LightAccel for LightTree {
    fn select(&self,
              inc: Vector,
              pos: Point,
              nor: Normal,
              sc: &SurfaceClosure,
              time: f32,
              n: f32)
              -> Option<(usize, f32, f32)> {
        if self.nodes.len() == 0 {
            return None;
        }

        let mut node_index = 0;
        let mut tot_prob = 1.0;
        let mut n = n;

        // Calculates the selection probability for a node
        let node_prob = |node_ref: &Node| {
            let bounds = &self.bounds[node_ref.bounds_range.0..node_ref.bounds_range.1];
            let bbox = lerp_slice(bounds, time);
            let d = bbox.center() - pos;
            let dist2 = d.length2();
            let r = bbox.diagonal() * 0.5;
            let inv_surface_area = 1.0 / (r * r);

            // Get the approximate amount of light contribution from the
            // composite light source.
            let approx_contrib = {
                let mut approx_contrib = 0.0;
                let steps = 2;
                let fstep = 1.0 / (steps as f32);
                for i in 0..steps {
                    let r2 = {
                        let step = fstep * (i + 1) as f32;
                        let r = r * step;
                        r * r
                    };
                    let cos_theta_max = if dist2 <= r2 {
                        -1.0
                    } else {
                        let sin_theta_max2 = (r2 / dist2).min(1.0);
                        (1.0 - sin_theta_max2).sqrt()
                    };
                    approx_contrib += sc.estimate_eval_over_solid_angle(inc, d, nor, cos_theta_max);
                }
                approx_contrib * fstep
            };

            node_ref.energy * inv_surface_area * approx_contrib
        };

        // Traverse down the tree, keeping track of the relative probabilities
        while !self.nodes[node_index].is_leaf {
            // Calculate the relative probabilities of the two children
            let (p1, p2) = {
                let p1 = node_prob(&self.nodes[node_index + 1]);
                let p2 = node_prob(&self.nodes[self.nodes[node_index].child_index]);
                let total = p1 + p2;

                if total <= 0.0 {
                    (0.5, 0.5)
                } else {
                    (p1 / total, p2 / total)
                }
            };

            if n <= p1 {
                tot_prob *= p1;
                node_index = node_index + 1;
                n /= p1;
            } else {
                tot_prob *= p2;
                node_index = self.nodes[node_index].child_index;
                n = (n - p1) / p2;
            }
        }

        // Found our light!
        Some((self.nodes[node_index].child_index, tot_prob, n))
    }

    fn approximate_energy(&self) -> f32 {
        if self.nodes.len() > 0 {
            self.nodes[0].energy
        } else {
            0.0
        }
    }
}
