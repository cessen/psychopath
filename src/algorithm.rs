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
