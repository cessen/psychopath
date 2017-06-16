#![allow(dead_code)]

use std;
use std::cmp::Ordering;

use halton;

use algorithm::{partition, quick_select};
use bbox::BBox;
use lerp::lerp_slice;
use math::{Vector, dot};
use sampling::uniform_sample_hemisphere;


const SAH_BIN_COUNT: usize = 13; // Prime numbers work best, for some reason
const SPLIT_PLANE_COUNT: usize = 5;


/// Takes a slice of boundable objects and partitions them based on the Surface
/// Area Heuristic, but using arbitrarily oriented planes.
///
/// Returns the index of the partition boundary and the axis that it split on
/// (0 = x, 1 = y, 2 = z).
pub fn free_sah_split<'a, T, F>(seed: u32, objects: &mut [T], bounder: &F) -> (usize, usize)
where
    F: Fn(&T) -> &'a [BBox],
{
    // Generate the planes for splitting
    let planes = {
        let mut planes = [Vector::new(0.0, 0.0, 0.0); SPLIT_PLANE_COUNT];
        let offset = seed * SPLIT_PLANE_COUNT as u32;
        for i in 0..SPLIT_PLANE_COUNT {
            let u = halton::sample(0, offset + i as u32);
            let v = halton::sample(1, offset + i as u32);
            planes[i] = uniform_sample_hemisphere(u, v).normalized();
        }
        planes
    };

    // Get the extents of the objects with respect to the split planes
    let extents = {
        let mut extents = [(std::f32::INFINITY, std::f32::NEG_INFINITY); SPLIT_PLANE_COUNT];
        for obj in &objects[..] {
            let centroid = lerp_slice(bounder(obj), 0.5).center().into_vector();
            for i in 0..SPLIT_PLANE_COUNT {
                let dist = dot(centroid, planes[i]);
                extents[i].0 = extents[i].0.min(dist);
                extents[i].1 = extents[i].1.max(dist);
            }
        }
        extents
    };

    // Pre-calc SAH div distances
    let sah_divs = {
        let mut sah_divs = [[0.0f32; SAH_BIN_COUNT - 1]; SPLIT_PLANE_COUNT];
        for pi in 0..SPLIT_PLANE_COUNT {
            let extent = extents[pi].1 - extents[pi].0;
            for div in 0..(SAH_BIN_COUNT - 1) {
                let part = extent * ((div + 1) as f32 / SAH_BIN_COUNT as f32);
                sah_divs[pi][div] = extents[pi].0 + part;
            }
        }
        sah_divs
    };

    // Build SAH bins
    let sah_bins = {
        let mut sah_bins = [[(BBox::new(), BBox::new(), 0, 0); SAH_BIN_COUNT - 1];
            SPLIT_PLANE_COUNT];
        for obj in objects.iter() {
            let tb = lerp_slice(bounder(obj), 0.5);
            let centroid = tb.center().into_vector();

            for pi in 0..SPLIT_PLANE_COUNT {
                for div in 0..(SAH_BIN_COUNT - 1) {
                    let dist = dot(centroid, planes[pi]);
                    if dist <= sah_divs[pi][div] {
                        sah_bins[pi][div].0 |= tb;
                        sah_bins[pi][div].2 += 1;
                    } else {
                        sah_bins[pi][div].1 |= tb;
                        sah_bins[pi][div].3 += 1;
                    }
                }
            }
        }
        sah_bins
    };

    // Find best split axis and div point
    let (split_plane_i, div_n) = {
        let mut split_plane_i = 0;
        let mut div_n = 0.0;
        let mut smallest_cost = std::f32::INFINITY;

        for pi in 0..SPLIT_PLANE_COUNT {
            for div in 0..(SAH_BIN_COUNT - 1) {
                let left_cost = sah_bins[pi][div].0.surface_area() * sah_bins[pi][div].2 as f32;
                let right_cost = sah_bins[pi][div].1.surface_area() * sah_bins[pi][div].3 as f32;
                let tot_cost = left_cost + right_cost;
                if tot_cost < smallest_cost {
                    split_plane_i = pi;
                    div_n = sah_divs[pi][div];
                    smallest_cost = tot_cost;
                }
            }
        }

        (split_plane_i, div_n)
    };

    // Calculate the approximate axis-aligned split, along with flipping the split plane as
    // appropriate.
    let (plane, approx_axis, div) = {
        // Find axis with largest value
        let mut largest_axis = 0;
        let mut n = 0.0;
        for d in 0..3 {
            let m = planes[split_plane_i].get_n(d).abs();
            if n < m {
                largest_axis = d;
                n = m;
            }
        }

        // If it's negative, flip
        if planes[split_plane_i].get_n(largest_axis).is_sign_positive() {
            (planes[split_plane_i], largest_axis, div_n)
        } else {
            (planes[split_plane_i] * -1.0, largest_axis, div_n * -1.0)
        }
    };

    // Partition
    let mut split_i = partition(&mut objects[..], |obj| {
        let centroid = lerp_slice(bounder(obj), 0.5).center().into_vector();
        let dist = dot(centroid, plane);
        dist < div
    });

    if split_i < 1 {
        split_i = 1;
    } else if split_i >= objects.len() {
        split_i = objects.len() - 1;
    }

    (split_i, approx_axis)
}


/// Takes a slice of boundable objects and partitions them based on the Surface
/// Area Heuristic.
///
/// Returns the index of the partition boundary and the axis that it split on
/// (0 = x, 1 = y, 2 = z).
pub fn sah_split<'a, T, F>(objects: &mut [T], bounder: &F) -> (usize, usize)
where
    F: Fn(&T) -> &'a [BBox],
{
    // Get combined object centroid extents
    let bounds = {
        let mut bb = BBox::new();
        for obj in &objects[..] {
            bb |= lerp_slice(bounder(obj), 0.5).center();
        }
        bb
    };

    // Pre-calc SAH div points
    let sah_divs = {
        let mut sah_divs = [[0.0f32; SAH_BIN_COUNT - 1]; 3];
        for d in 0..3 {
            let extent = bounds.max.get_n(d) - bounds.min.get_n(d);
            for div in 0..(SAH_BIN_COUNT - 1) {
                let part = extent * ((div + 1) as f32 / SAH_BIN_COUNT as f32);
                sah_divs[d][div] = bounds.min.get_n(d) + part;
            }
        }
        sah_divs
    };

    // Build SAH bins
    let sah_bins = {
        let mut sah_bins = [[(BBox::new(), BBox::new(), 0, 0); SAH_BIN_COUNT - 1]; 3];
        for obj in objects.iter() {
            let tb = lerp_slice(bounder(obj), 0.5);
            let centroid = (tb.min.into_vector() + tb.max.into_vector()) * 0.5;

            for d in 0..3 {
                for div in 0..(SAH_BIN_COUNT - 1) {
                    if centroid.get_n(d) <= sah_divs[d][div] {
                        sah_bins[d][div].0 |= tb;
                        sah_bins[d][div].2 += 1;
                    } else {
                        sah_bins[d][div].1 |= tb;
                        sah_bins[d][div].3 += 1;
                    }
                }
            }
        }
        sah_bins
    };

    // Find best split axis and div point
    let (split_axis, div) = {
        let mut dim = 0;
        let mut div_n = 0.0;
        let mut smallest_cost = std::f32::INFINITY;

        for d in 0..3 {
            for div in 0..(SAH_BIN_COUNT - 1) {
                let left_cost = sah_bins[d][div].0.surface_area() * sah_bins[d][div].2 as f32;
                let right_cost = sah_bins[d][div].1.surface_area() * sah_bins[d][div].3 as f32;
                let tot_cost = left_cost + right_cost;
                if tot_cost < smallest_cost {
                    dim = d;
                    div_n = sah_divs[d][div];
                    smallest_cost = tot_cost;
                }
            }
        }

        (dim, div_n)
    };

    // Partition
    let mut split_i = partition(&mut objects[..], |obj| {
        let tb = lerp_slice(bounder(obj), 0.5);
        let centroid = (tb.min.get_n(split_axis) + tb.max.get_n(split_axis)) * 0.5;
        centroid < div
    });
    if split_i < 1 {
        split_i = 1;
    } else if split_i >= objects.len() {
        split_i = objects.len() - 1;
    }

    (split_i, split_axis)
}

/// Takes a slice of boundable objects and partitions them based on the bounds mean heuristic.
///
/// Returns the index of the partition boundary and the axis that it split on
/// (0 = x, 1 = y, 2 = z).
pub fn bounds_mean_split<'a, T, F>(objects: &mut [T], bounder: &F) -> (usize, usize)
where
    F: Fn(&T) -> &'a [BBox],
{
    // Get combined object bounds
    let bounds = {
        let mut bb = BBox::new();
        for obj in &objects[..] {
            bb |= lerp_slice(bounder(obj), 0.5);
        }
        bb
    };

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

    let div = (bounds.min.get_n(split_axis) + bounds.max.get_n(split_axis)) * 0.5;

    // Partition
    let mut split_i = partition(&mut objects[..], |obj| {
        let tb = lerp_slice(bounder(obj), 0.5);
        let centroid = (tb.min.get_n(split_axis) + tb.max.get_n(split_axis)) * 0.5;
        centroid < div
    });
    if split_i < 1 {
        split_i = 1;
    } else if split_i >= objects.len() {
        split_i = objects.len() - 1;
    }

    (split_i, split_axis)
}


/// Takes a slice of boundable objects and partitions them based on the median heuristic.
///
/// Returns the index of the partition boundary and the axis that it split on
/// (0 = x, 1 = y, 2 = z).
pub fn median_split<'a, T, F>(objects: &mut [T], bounder: &F) -> (usize, usize)
where
    F: Fn(&T) -> &'a [BBox],
{
    // Get combined object bounds
    let bounds = {
        let mut bb = BBox::new();
        for obj in &objects[..] {
            bb |= lerp_slice(bounder(obj), 0.5);
        }
        bb
    };

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
        if place > 0 { place } else { 1 }
    };
    quick_select(objects, place, |a, b| {
        let tb_a = lerp_slice(bounder(a), 0.5);
        let tb_b = lerp_slice(bounder(b), 0.5);
        let centroid_a = (tb_a.min.get_n(split_axis) + tb_a.max.get_n(split_axis)) * 0.5;
        let centroid_b = (tb_b.min.get_n(split_axis) + tb_b.max.get_n(split_axis)) * 0.5;

        if centroid_a < centroid_b {
            Ordering::Less
        } else if centroid_a == centroid_b {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    });

    (place, split_axis)
}
