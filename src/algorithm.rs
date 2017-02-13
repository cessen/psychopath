#![allow(dead_code)]

use std;
use std::cmp;
use std::cmp::Ordering;

use hash::hash_u64;
use lerp::{Lerp, lerp_slice};


/// Selects an item from a slice based on a weighting function and a
/// number (n) between 0.0 and 1.0.  Returns the index of the selected
/// item and the probability that it would have been selected with a
/// random n.
pub fn weighted_choice<T, F>(slc: &[T], n: f32, weight: F) -> (usize, f32)
    where F: Fn(&T) -> f32
{
    assert!(slc.len() > 0);

    let total_weight = slc.iter().fold(0.0, |sum, v| sum + weight(v));
    let n = n * total_weight;

    let mut x = 0.0;
    for (i, v) in slc.iter().enumerate() {
        let w = weight(v);
        x += w;
        if x > n || i == slc.len() {
            return (i, w / total_weight);
        }
    }

    unreachable!()
}


/// Partitions a slice in-place with the given unary predicate, returning
/// the index of the first element for which the predicate evaluates
/// false.
///
/// The predicate is executed precisely once on every element in
/// the slice, and is allowed to modify the elements.
pub fn partition<T, F>(slc: &mut [T], mut pred: F) -> usize
    where F: FnMut(&mut T) -> bool
{
    // This version uses raw pointers and pointer arithmetic to squeeze more
    // performance out of the code.
    unsafe {
        let mut a = slc.as_mut_ptr();
        let mut b = a.offset(slc.len() as isize);
        let start = a as usize;

        loop {
            loop {
                if a == b {
                    return ((a as usize) - start) / std::mem::size_of::<T>();
                }
                if !pred(&mut *a) {
                    break;
                }
                a = a.offset(1);
            }

            loop {
                b = b.offset(-1);
                if a == b {
                    return ((a as usize) - start) / std::mem::size_of::<T>();
                }
                if pred(&mut *b) {
                    break;
                }
            }

            std::ptr::swap(a, b);

            a = a.offset(1);
        }
    }
}


/// Partitions two slices in-place in concert based on the given unary
/// predicate, returning the index of the first element for which the
/// predicate evaluates false.
///
/// Because this runs on two slices at once, they must both be the same
/// length.
///
/// The predicate takes a usize (which will receive the index of the elments
/// being tested), a mutable reference to an element of the first slice's type,
/// and a mutable reference to an element of the last slice's type.
///
/// The predicate is executed precisely once on every element in
/// the slices, and is allowed to modify the elements.
pub fn partition_pair<A, B, F>(slc1: &mut [A], slc2: &mut [B], mut pred: F) -> usize
    where F: FnMut(usize, &mut A, &mut B) -> bool
{
    assert!(slc1.len() == slc2.len());

    // This version uses raw pointers and pointer arithmetic to squeeze more
    // performance out of the code.
    unsafe {
        let mut a1 = slc1.as_mut_ptr();
        let mut a2 = slc2.as_mut_ptr();
        let mut b1 = a1.offset(slc1.len() as isize);
        let mut b2 = a2.offset(slc2.len() as isize);
        let start = a1 as usize;

        loop {
            loop {
                if a1 == b1 {
                    return ((a1 as usize) - start) / std::mem::size_of::<A>();
                }
                if !pred(((a1 as usize) - start) / std::mem::size_of::<A>(),
                         &mut *a1,
                         &mut *a2) {
                    break;
                }
                a1 = a1.offset(1);
                a2 = a2.offset(1);
            }

            loop {
                b1 = b1.offset(-1);
                b2 = b2.offset(-1);
                if a1 == b1 {
                    return ((a1 as usize) - start) / std::mem::size_of::<A>();
                }
                if pred(((b1 as usize) - start) / std::mem::size_of::<A>(),
                        &mut *b1,
                        &mut *b2) {
                    break;
                }
            }

            std::ptr::swap(a1, b1);
            std::ptr::swap(a2, b2);

            a1 = a1.offset(1);
            a2 = a2.offset(1);
        }
    }
}

/// Partitions the slice of items to place the nth-ordered item in the nth place,
/// and the items less than it before and the items more than it after.
pub fn quick_select<T, F>(slc: &mut [T], n: usize, mut order: F)
    where F: FnMut(&T, &T) -> Ordering
{
    let mut left = 0;
    let mut right = slc.len();
    let mut seed = n as u64;

    loop {
        let i = left + (hash_u64(right as u64, seed) as usize % (right - left));

        slc.swap(i, right - 1);
        let ii = left +
                 {
            let (val, list) = (&mut slc[left..right]).split_last_mut().unwrap();
            partition(list, |n| order(n, val) == Ordering::Less)
        };
        slc.swap(ii, right - 1);

        if ii == n {
            return;
        } else if ii > n {
            right = ii;
        } else {
            left = ii + 1;
        }

        seed += 1;
    }
}

/// Merges two slices of things, appending the result to vec_out
pub fn merge_slices_append<T: Lerp + Copy, F>(slice1: &[T],
                                              slice2: &[T],
                                              vec_out: &mut Vec<T>,
                                              merge: F)
    where F: Fn(&T, &T) -> T
{
    // Transform the bounding boxes
    if slice1.len() == 0 || slice2.len() == 0 {
        return;
    } else if slice1.len() == slice2.len() {
        for (xf1, xf2) in Iterator::zip(slice1.iter(), slice2.iter()) {
            vec_out.push(merge(xf1, xf2));
        }
    } else if slice1.len() > slice2.len() {
        let s = (slice1.len() - 1) as f32;
        for (i, xf1) in slice1.iter().enumerate() {
            let xf2 = lerp_slice(slice2, i as f32 / s);
            vec_out.push(merge(xf1, &xf2));
        }
    } else if slice1.len() < slice2.len() {
        let s = (slice2.len() - 1) as f32;
        for (i, xf2) in slice2.iter().enumerate() {
            let xf1 = lerp_slice(slice1, i as f32 / s);
            vec_out.push(merge(&xf1, xf2));
        }
    }
}

/// Merges two slices of things, storing the result in slice_out.
/// Panics if slice_out is not the right size.
pub fn merge_slices_to<T: Lerp + Copy, F>(slice1: &[T],
                                          slice2: &[T],
                                          slice_out: &mut [T],
                                          merge: F)
    where F: Fn(&T, &T) -> T
{
    assert!(slice_out.len() == cmp::max(slice1.len(), slice2.len()));

    // Transform the bounding boxes
    if slice1.len() == 0 || slice2.len() == 0 {
        return;
    } else if slice1.len() == slice2.len() {
        for (xfo, (xf1, xf2)) in
            Iterator::zip(slice_out.iter_mut(),
                          Iterator::zip(slice1.iter(), slice2.iter())) {
            *xfo = merge(xf1, xf2);
        }
    } else if slice1.len() > slice2.len() {
        let s = (slice1.len() - 1) as f32;
        for (i, (xfo, xf1)) in Iterator::zip(slice_out.iter_mut(), slice1.iter()).enumerate() {
            let xf2 = lerp_slice(slice2, i as f32 / s);
            *xfo = merge(xf1, &xf2);
        }
    } else if slice1.len() < slice2.len() {
        let s = (slice2.len() - 1) as f32;
        for (i, (xfo, xf2)) in Iterator::zip(slice_out.iter_mut(), slice2.iter()).enumerate() {
            let xf1 = lerp_slice(slice1, i as f32 / s);
            *xfo = merge(&xf1, xf2);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use super::*;

    fn quick_select_ints(list: &mut [i32], i: usize) {
        quick_select(list, i, |a, b| if a < b {
            Ordering::Less
        } else if a == b {
            Ordering::Equal
        } else {
            Ordering::Greater
        });
    }

    #[test]
    fn quick_select_1() {
        let mut list = [8, 9, 7, 4, 6, 1, 0, 5, 3, 2];
        quick_select_ints(&mut list, 5);
        assert_eq!(list[5], 5);
    }

    #[test]
    fn quick_select_2() {
        let mut list = [8, 9, 7, 4, 6, 1, 0, 5, 3, 2];
        quick_select_ints(&mut list, 3);
        assert_eq!(list[3], 3);
    }

    #[test]
    fn quick_select_3() {
        let mut list = [8, 9, 7, 4, 6, 1, 0, 5, 3, 2];
        quick_select_ints(&mut list, 0);
        assert_eq!(list[0], 0);
    }

    #[test]
    fn quick_select_4() {
        let mut list = [8, 9, 7, 4, 6, 1, 0, 5, 3, 2];
        quick_select_ints(&mut list, 9);
        assert_eq!(list[9], 9);
    }
}
