#![allow(dead_code)]

use algorithm::merge_slices_append;
use bbox::BBox;
use lerp::lerp_slice;
use math::log2_64;

use super::objects_split::{median_split, sah_split};

pub const BVH_MAX_DEPTH: usize = 42;

// Amount bigger the union of all time samples can be
// and still use the union rather than preserve the
// individual time samples.
const USE_UNION_FACTOR: f32 = 1.4;

/// An intermediary structure for creating a BVH.
#[derive(Debug)]
pub struct BVHBase {
    pub nodes: Vec<BVHBaseNode>,
    pub bounds: Vec<BBox>,
    pub depth: usize,
    bounds_cache: Vec<BBox>,
}

#[derive(Copy, Clone, Debug)]
pub enum BVHBaseNode {
    Internal {
        bounds_range: (usize, usize),
        children_indices: (usize, usize),
        split_axis: u8,
    },

    Leaf {
        bounds_range: (usize, usize),
        object_range: (usize, usize),
    },
}

impl BVHBaseNode {
    pub fn bounds_range(&self) -> (usize, usize) {
        match *self {
            BVHBaseNode::Internal { bounds_range, .. } | BVHBaseNode::Leaf { bounds_range, .. } => {
                bounds_range
            }
        }
    }
}

impl BVHBase {
    fn new() -> BVHBase {
        BVHBase {
            nodes: Vec::new(),
            bounds: Vec::new(),
            depth: 0,
            bounds_cache: Vec::new(),
        }
    }

    pub fn from_objects<'b, T, F>(objects: &mut [T], objects_per_leaf: usize, bounder: F) -> BVHBase
    where
        F: 'b + Fn(&T) -> &'b [BBox],
    {
        let mut bvh = BVHBase::new();
        bvh.recursive_build(0, 0, objects_per_leaf, objects, &bounder);
        bvh
    }

    pub fn root_node_index(&self) -> usize {
        0
    }

    fn acc_bounds<'a, T, F>(&mut self, objects: &mut [T], bounder: &F)
    where
        F: 'a + Fn(&T) -> &'a [BBox],
    {
        // TODO: do all of this without the temporary cache
        let max_len = objects.iter().map(|obj| bounder(obj).len()).max().unwrap();

        self.bounds_cache.clear();
        self.bounds_cache.resize(max_len, BBox::new());

        for obj in objects.iter() {
            let bounds = bounder(obj);
            debug_assert!(!bounds.is_empty());
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

    fn recursive_build<'a, T, F>(
        &mut self,
        offset: usize,
        depth: usize,
        objects_per_leaf: usize,
        objects: &mut [T],
        bounder: &F,
    ) -> (usize, (usize, usize))
    where
        F: 'a + Fn(&T) -> &'a [BBox],
    {
        let me = self.nodes.len();

        if objects.is_empty() {
            return (0, (0, 0));
        } else if objects.len() <= objects_per_leaf {
            // Leaf node
            let bi = self.bounds.len();
            // Get bounds
            {
                // We make sure that it's worth having multiple time samples, and if not
                // we reduce to the union of the time samples.
                self.acc_bounds(objects, bounder);
                let union_bounds = self
                    .bounds_cache
                    .iter()
                    .fold(BBox::new(), |b1, b2| (b1 | *b2));
                let average_area = self
                    .bounds_cache
                    .iter()
                    .fold(0.0, |area, bb| area + bb.surface_area())
                    / self.bounds_cache.len() as f32;
                if union_bounds.surface_area() <= (average_area * USE_UNION_FACTOR) {
                    self.bounds.push(union_bounds);
                } else {
                    self.bounds.extend(&self.bounds_cache);
                }
            }

            // Create node
            self.nodes.push(BVHBaseNode::Leaf {
                bounds_range: (bi, self.bounds.len()),
                object_range: (offset, offset + objects.len()),
            });

            if self.depth < depth {
                self.depth = depth;
            }

            return (me, (bi, self.bounds.len()));
        } else {
            // Not a leaf node
            self.nodes.push(BVHBaseNode::Internal {
                bounds_range: (0, 0),
                children_indices: (0, 0),
                split_axis: 0,
            });

            // Partition objects.
            // If we're too near the max depth, we do balanced building to
            // avoid exceeding max depth.
            // Otherwise we do SAH splitting to build better trees.
            let (split_index, split_axis) =
                if (log2_64(objects.len() as u64) as usize) < (BVH_MAX_DEPTH - depth) {
                    // SAH splitting, when we have room to play
                    sah_split(objects, &bounder)
                } else {
                    // Balanced splitting, when we don't have room to play
                    median_split(objects, &bounder)
                };

            // Create child nodes
            let (c1_index, c1_bounds) = self.recursive_build(
                offset,
                depth + 1,
                objects_per_leaf,
                &mut objects[..split_index],
                bounder,
            );
            let (c2_index, c2_bounds) = self.recursive_build(
                offset + split_index,
                depth + 1,
                objects_per_leaf,
                &mut objects[split_index..],
                bounder,
            );

            // Determine bounds
            // TODO: do merging without the temporary vec.
            let bi = self.bounds.len();
            {
                let mut merged = Vec::new();
                merge_slices_append(
                    &self.bounds[c1_bounds.0..c1_bounds.1],
                    &self.bounds[c2_bounds.0..c2_bounds.1],
                    &mut merged,
                    |b1, b2| *b1 | *b2,
                );
                // We make sure that it's worth having multiple time samples, and if not
                // we reduce to the union of the time samples.
                let union_bounds = merged.iter().fold(BBox::new(), |b1, b2| (b1 | *b2));
                let average_area = merged.iter().fold(0.0, |area, bb| area + bb.surface_area())
                    / merged.len() as f32;
                if union_bounds.surface_area() <= (average_area * USE_UNION_FACTOR) {
                    self.bounds.push(union_bounds);
                } else {
                    self.bounds.extend(merged.drain(0..));
                }
            }

            // Set node
            self.nodes[me] = BVHBaseNode::Internal {
                bounds_range: (bi, self.bounds.len()),
                children_indices: (c1_index, c2_index),
                split_axis: split_axis as u8,
            };

            return (me, (bi, self.bounds.len()));
        }
    }
}
