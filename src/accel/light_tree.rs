use mem_arena::MemArena;

use algorithm::merge_slices_append;
use bbox::BBox;
use lerp::lerp_slice;
use math::{Vector, Point, Normal};
use shading::surface_closure::SurfaceClosure;

use super::LightAccel;
use super::objects_split::sah_split;

const LEVEL_COLLAPSE: usize = 1; // Number of levels of the binary tree to
// collapse together (1 = no collapsing)
const ARITY: usize = 1 << LEVEL_COLLAPSE; // Arity of the final tree


#[derive(Copy, Clone, Debug)]
pub struct LightTree<'a> {
    root: Option<&'a Node<'a>>,
    depth: usize,
}

#[derive(Copy, Clone, Debug)]
enum Node<'a> {
    Inner {
        children: &'a [Node<'a>],
        bounds: &'a [BBox],
        energy: f32,
    },
    Leaf {
        light_index: usize,
        bounds: &'a [BBox],
        energy: f32,
    },
}

impl<'a> Node<'a> {
    fn bounds(&self) -> &'a [BBox] {
        match self {
            &Node::Inner { ref bounds, .. } => bounds,
            &Node::Leaf { ref bounds, .. } => bounds,
        }
    }

    fn energy(&self) -> f32 {
        match self {
            &Node::Inner { energy, .. } => energy,
            &Node::Leaf { energy, .. } => energy,
        }
    }

    fn light_index(&self) -> usize {
        match self {
            &Node::Inner { .. } => panic!(),
            &Node::Leaf { light_index, .. } => light_index,
        }
    }
}

impl<'a> LightTree<'a> {
    pub fn from_objects<'b, T, F>(
        arena: &'a MemArena,
        objects: &mut [T],
        info_getter: F,
    ) -> LightTree<'a>
    where
        F: 'b + Fn(&T) -> (&'b [BBox], f32),
    {
        if objects.len() == 0 {
            LightTree {
                root: None,
                depth: 0,
            }
        } else {
            let mut builder = LightTreeBuilder::new();
            builder.recursive_build(0, 0, objects, &info_getter);

            let mut root = unsafe { arena.alloc_uninitialized::<Node>() };
            LightTree::construct_from_builder(arena, &builder, builder.root_node_index(), root);

            LightTree {
                root: Some(root),
                depth: builder.depth,
            }
        }
    }

    fn construct_from_builder(
        arena: &'a MemArena,
        base: &LightTreeBuilder,
        node_index: usize,
        node_mem: &mut Node<'a>,
    ) {
        if base.nodes[node_index].is_leaf {
            // Leaf
            let bounds_range = base.nodes[node_index].bounds_range;
            let bounds = arena.copy_slice(&base.bounds[bounds_range.0..bounds_range.1]);

            *node_mem = Node::Leaf {
                light_index: base.nodes[node_index].child_index,
                bounds: bounds,
                energy: base.nodes[node_index].energy,
            };
        } else {
            // Inner
            let bounds_range = base.nodes[node_index].bounds_range;
            let bounds = arena.copy_slice(&base.bounds[bounds_range.0..bounds_range.1]);

            let mut children = unsafe { arena.alloc_array_uninitialized::<Node>(2) };
            LightTree::construct_from_builder(arena, base, node_index + 1, &mut children[0]);
            LightTree::construct_from_builder(
                arena,
                base,
                base.nodes[node_index].child_index,
                &mut children[1],
            );

            *node_mem = Node::Inner {
                children: children,
                bounds: bounds,
                energy: base.nodes[node_index].energy,
            };
        }
    }
}


impl<'a> LightAccel for LightTree<'a> {
    fn select(
        &self,
        inc: Vector,
        pos: Point,
        nor: Normal,
        sc: &SurfaceClosure,
        time: f32,
        n: f32,
    ) -> Option<(usize, f32, f32)> {
        if self.root.is_none() {
            return None;
        }

        // Calculates the selection probability for a node
        let node_prob = |node_ref: &Node| {
            let bbox = lerp_slice(node_ref.bounds(), time);
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

            node_ref.energy() * inv_surface_area * approx_contrib
        };

        // Traverse down the tree, keeping track of the relative probabilities
        let mut node = self.root.unwrap();
        let mut tot_prob = 1.0;
        let mut n = n;
        loop {
            if let Node::Inner { children, .. } = *node {
                // Calculate the relative probabilities of the children
                let ps = {
                    let mut ps = [0.0; ARITY];
                    let mut total = 0.0;
                    for (i, child) in children.iter().enumerate() {
                        let p = node_prob(child);
                        ps[i] = p;
                        total += p;
                    }
                    if total <= 0.0 {
                        let p = 1.0 / children.len() as f32;
                        for prob in &mut ps {
                            *prob = p;
                        }
                    } else {
                        for prob in &mut ps {
                            *prob = *prob / total;
                        }
                    }
                    ps
                };

                // Pick child and update probabilities
                let mut base = 0.0;
                for (i, &p) in ps.iter().enumerate() {
                    if (n <= base + p) || (i == children.len() - 1) {
                        tot_prob *= p;
                        node = &children[i];
                        n = (n - base) / p;
                        break;
                    } else {
                        base += p;
                    }
                }
            } else {
                break;
            }
        }

        // Found our light!
        Some((node.light_index(), tot_prob, n))
    }

    fn approximate_energy(&self) -> f32 {
        if let Some(ref node) = self.root {
            node.energy()
        } else {
            0.0
        }
    }
}


struct LightTreeBuilder {
    nodes: Vec<BuilderNode>,
    bounds: Vec<BBox>,
    depth: usize,
}

#[derive(Copy, Clone, Debug)]
struct BuilderNode {
    is_leaf: bool,
    bounds_range: (usize, usize),
    energy: f32,
    child_index: usize,
}

impl LightTreeBuilder {
    fn new() -> LightTreeBuilder {
        LightTreeBuilder {
            nodes: Vec::new(),
            bounds: Vec::new(),
            depth: 0,
        }
    }

    pub fn root_node_index(&self) -> usize {
        0
    }

    fn recursive_build<'a, T, F>(
        &mut self,
        offset: usize,
        depth: usize,
        objects: &mut [T],
        info_getter: &F,
    ) -> (usize, (usize, usize))
    where
        F: 'a + Fn(&T) -> (&'a [BBox], f32),
    {
        let me_index = self.nodes.len();

        if objects.len() == 0 {
            return (0, (0, 0));
        } else if objects.len() == 1 {
            // Leaf node
            let bi = self.bounds.len();
            let (obj_bounds, energy) = info_getter(&objects[0]);
            self.bounds.extend(obj_bounds);
            self.nodes.push(BuilderNode {
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
            self.nodes.push(BuilderNode {
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
            let (c2_index, c2_bounds) = self.recursive_build(
                offset + split_index,
                depth + 1,
                &mut objects[split_index..],
                info_getter,
            );

            // Determine bounds
            // TODO: do merging without the temporary vec.
            let bi = self.bounds.len();
            let mut merged = Vec::new();
            merge_slices_append(
                &self.bounds[c1_bounds.0..c1_bounds.1],
                &self.bounds[c2_bounds.0..c2_bounds.1],
                &mut merged,
                |b1, b2| *b1 | *b2,
            );
            self.bounds.extend(merged.drain(0..));

            // Set node
            let energy = self.nodes[me_index + 1].energy + self.nodes[c2_index].energy;
            self.nodes[me_index] = BuilderNode {
                is_leaf: false,
                bounds_range: (bi, self.bounds.len()),
                energy: energy,
                child_index: c2_index,
            };

            return (me_index, (bi, self.bounds.len()));
        }
    }
}
