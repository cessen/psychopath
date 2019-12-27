use std::mem::{transmute, MaybeUninit};

use kioku::Arena;

use crate::{
    algorithm::merge_slices_append,
    bbox::BBox,
    lerp::lerp_slice,
    math::{Normal, Point, Vector},
    shading::surface_closure::SurfaceClosure,
};

use super::{objects_split::sah_split, LightAccel};

const ARITY_LOG2: usize = 3; // Determines how much to collapse the binary tree,
                             // implicitly defining the light tree's arity.  1 = no collapsing, leave as binary
                             // tree.
const ARITY: usize = 1 << ARITY_LOG2; // Arity of the final tree

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
        match *self {
            Node::Inner { bounds, .. } | Node::Leaf { bounds, .. } => bounds,
        }
    }

    fn energy(&self) -> f32 {
        match *self {
            Node::Inner { energy, .. } | Node::Leaf { energy, .. } => energy,
        }
    }

    fn light_index(&self) -> usize {
        match *self {
            Node::Inner { .. } => panic!(),
            Node::Leaf { light_index, .. } => light_index,
        }
    }
}

impl<'a> LightTree<'a> {
    pub fn from_objects<'b, T, F>(
        arena: &'a Arena,
        objects: &mut [T],
        info_getter: F,
    ) -> LightTree<'a>
    where
        F: 'b + Fn(&T) -> (&'b [BBox], f32),
    {
        if objects.is_empty() {
            LightTree {
                root: None,
                depth: 0,
            }
        } else {
            let mut builder = LightTreeBuilder::new();
            builder.recursive_build(0, 0, objects, &info_getter);

            let root = arena.alloc_uninit::<Node>();
            LightTree::construct_from_builder(arena, &builder, builder.root_node_index(), root);

            LightTree {
                root: Some(unsafe { transmute(root) }),
                depth: builder.depth,
            }
        }
    }

    fn construct_from_builder(
        arena: &'a Arena,
        base: &LightTreeBuilder,
        node_index: usize,
        node_mem: &mut MaybeUninit<Node<'a>>,
    ) {
        if base.nodes[node_index].is_leaf {
            // Leaf
            let bounds_range = base.nodes[node_index].bounds_range;
            let bounds = arena.copy_slice(&base.bounds[bounds_range.0..bounds_range.1]);

            unsafe {
                *node_mem.as_mut_ptr() = Node::Leaf {
                    light_index: base.nodes[node_index].child_index,
                    bounds: bounds,
                    energy: base.nodes[node_index].energy,
                };
            }
        } else {
            // Inner
            let bounds_range = base.nodes[node_index].bounds_range;
            let bounds = arena.copy_slice(&base.bounds[bounds_range.0..bounds_range.1]);

            let child_count = base.node_child_count(node_index);
            let children = arena.alloc_array_uninit::<Node>(child_count);
            for i in 0..child_count {
                LightTree::construct_from_builder(
                    arena,
                    base,
                    base.node_nth_child_index(node_index, i),
                    &mut children[i],
                );
            }

            unsafe {
                *node_mem.as_mut_ptr() = Node::Inner {
                    children: transmute(children),
                    bounds: bounds,
                    energy: base.nodes[node_index].energy,
                };
            }
        }
    }
}

impl<'a> LightAccel for LightTree<'a> {
    fn select(
        &self,
        inc: Vector,
        pos: Point,
        nor: Normal,
        nor_g: Normal,
        sc: &SurfaceClosure,
        time: f32,
        n: f32,
    ) -> Option<(usize, f32, f32)> {
        // Calculates the selection probability for a node
        let node_prob = |node_ref: &Node| {
            let bbox = lerp_slice(node_ref.bounds(), time);
            let d = bbox.center() - pos;
            let r2 = bbox.diagonal2() * 0.25;
            let inv_surface_area = 1.0 / r2;

            // Get the approximate amount of light contribution from the
            // composite light source.
            let approx_contrib = sc.estimate_eval_over_sphere_light(inc, d, r2, nor, nor_g);
            node_ref.energy() * inv_surface_area * approx_contrib
        };

        // Traverse down the tree, keeping track of the relative probabilities
        let mut node = self.root?;
        let mut tot_prob = 1.0;
        let mut n = n;
        while let Node::Inner { children, .. } = *node {
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
                    for prob in &mut ps[..] {
                        *prob = p;
                    }
                } else {
                    for prob in &mut ps[..] {
                        *prob /= total;
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
        }

        // Found our light!
        Some((node.light_index(), tot_prob, n))
    }

    fn approximate_energy(&self) -> f32 {
        if let Some(node) = self.root {
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

    // Returns the number of children of this node, assuming a collapse
    // number of `ARITY_LOG2`.
    pub fn node_child_count(&self, node_index: usize) -> usize {
        self.node_child_count_recurse(ARITY_LOG2, node_index)
    }

    // Returns the index of the nth child, assuming a collapse
    // number of `ARITY_LOG2`.
    pub fn node_nth_child_index(&self, node_index: usize, child_n: usize) -> usize {
        self.node_nth_child_index_recurse(ARITY_LOG2, node_index, child_n)
            .0
    }

    // Returns the number of children of this node, assuming a collapse
    // number of `level_collapse`.
    pub fn node_child_count_recurse(&self, level_collapse: usize, node_index: usize) -> usize {
        if level_collapse > 0 {
            if self.nodes[node_index].is_leaf {
                1
            } else {
                let a = self.node_child_count_recurse(level_collapse - 1, node_index + 1);
                let b = self.node_child_count_recurse(
                    level_collapse - 1,
                    self.nodes[node_index].child_index,
                );

                a + b
            }
        } else {
            1
        }
    }

    // Returns the index of the nth child, assuming a collapse
    // number of `level_collapse`.
    pub fn node_nth_child_index_recurse(
        &self,
        level_collapse: usize,
        node_index: usize,
        child_n: usize,
    ) -> (usize, usize) {
        if level_collapse > 0 && !self.nodes[node_index].is_leaf {
            let (index, rem) =
                self.node_nth_child_index_recurse(level_collapse - 1, node_index + 1, child_n);
            if rem == 0 {
                return (index, 0);
            }
            return self.node_nth_child_index_recurse(
                level_collapse - 1,
                self.nodes[node_index].child_index,
                rem - 1,
            );
        } else {
            return (node_index, child_n);
        }
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

        if objects.is_empty() {
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
