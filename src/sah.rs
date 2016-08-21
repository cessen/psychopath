use std;

use algorithm::partition;
use bbox::BBox;
use lerp::lerp_slice;


const SAH_BIN_COUNT: usize = 13; // Prime numbers work best, for some reason

/// Takes a slice of boundable objects and partitions them based on the Surface
/// Area Heuristic.
///
/// Returns the index of the partition boundary and the axis that it split on
/// (0 = x, 1 = y, 2 = z).
pub fn sah_split<'a, T, F>(objects: &mut [T], bounder: &F) -> (usize, usize)
    where F: Fn(&T) -> &'a [BBox]
{
    // Get combined object bounds
    let bounds = {
        let mut bb = BBox::new();
        for obj in &objects[..] {
            bb |= lerp_slice(bounder(obj), 0.5);
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
