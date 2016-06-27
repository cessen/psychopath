#![allow(dead_code)]

use std;

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
