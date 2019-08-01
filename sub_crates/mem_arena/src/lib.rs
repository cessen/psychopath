#![allow(clippy::redundant_field_names)]
#![allow(clippy::needless_return)]
#![allow(clippy::mut_from_ref)]
#![allow(clippy::transmute_ptr_to_ptr)]

use std::{
    cell::{Cell, RefCell},
    cmp::max,
    fmt,
    mem::{align_of, size_of, transmute, MaybeUninit},
    slice,
};

const GROWTH_FRACTION: usize = 8; // 1/N  (smaller number leads to bigger allocations)
const DEFAULT_MIN_BLOCK_SIZE: usize = 1 << 10; // 1 KiB
const DEFAULT_MAX_WASTE_PERCENTAGE: usize = 10;

fn alignment_offset(addr: usize, alignment: usize) -> usize {
    (alignment - (addr % alignment)) % alignment
}

/// A growable memory arena for Copy types.
///
/// The arena works by allocating memory in blocks of slowly increasing size.  It
/// doles out memory from the current block until an amount of memory is requested
/// that doesn't fit in the remainder of the current block, and then allocates a new
/// block.
///
/// Additionally, it attempts to minimize wasted space through some heuristics.  By
/// default, it tries to keep memory waste within the arena below 10%.
#[derive(Default)]
pub struct MemArena {
    blocks: RefCell<Vec<Vec<MaybeUninit<u8>>>>,
    min_block_size: usize,
    max_waste_percentage: usize,
    stat_space_occupied: Cell<usize>,
    stat_space_allocated: Cell<usize>,
}

impl fmt::Debug for MemArena {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MemArena")
            .field("blocks.len():", &self.blocks.borrow().len())
            .field("min_block_size", &self.min_block_size)
            .field("max_waste_percentage", &self.max_waste_percentage)
            .field("stat_space_occupied", &self.stat_space_occupied)
            .field("stat_space_allocated", &self.stat_space_allocated)
            .finish()
    }
}

impl MemArena {
    /// Create a new arena, with default minimum block size.
    pub fn new() -> MemArena {
        MemArena {
            blocks: RefCell::new(vec![Vec::with_capacity(DEFAULT_MIN_BLOCK_SIZE)]),
            min_block_size: DEFAULT_MIN_BLOCK_SIZE,
            max_waste_percentage: DEFAULT_MAX_WASTE_PERCENTAGE,
            stat_space_occupied: Cell::new(DEFAULT_MIN_BLOCK_SIZE),
            stat_space_allocated: Cell::new(0),
        }
    }

    /// Create a new arena, with a specified minimum block size.
    pub fn with_min_block_size(min_block_size: usize) -> MemArena {
        assert!(min_block_size > 0);

        MemArena {
            blocks: RefCell::new(vec![Vec::with_capacity(min_block_size)]),
            min_block_size: min_block_size,
            max_waste_percentage: DEFAULT_MAX_WASTE_PERCENTAGE,
            stat_space_occupied: Cell::new(min_block_size),
            stat_space_allocated: Cell::new(0),
        }
    }

    /// Create a new arena, with a specified minimum block size and maximum waste percentage.
    pub fn with_settings(min_block_size: usize, max_waste_percentage: usize) -> MemArena {
        assert!(min_block_size > 0);
        assert!(max_waste_percentage > 0 && max_waste_percentage <= 100);

        MemArena {
            blocks: RefCell::new(vec![Vec::with_capacity(min_block_size)]),
            min_block_size: min_block_size,
            max_waste_percentage: max_waste_percentage,
            stat_space_occupied: Cell::new(min_block_size),
            stat_space_allocated: Cell::new(0),
        }
    }

    /// Returns statistics about the current usage as a tuple:
    /// (space occupied, space allocated, block count, large block count)
    ///
    /// Space occupied is the amount of real memory that the MemArena
    /// is taking up (not counting book keeping).
    ///
    /// Space allocated is the amount of the occupied space that is
    /// actually used.  In other words, it is the sum of the all the
    /// allocation requests made to the arena by client code.
    ///
    /// Block count is the number of blocks that have been allocated.
    pub fn stats(&self) -> (usize, usize, usize) {
        let occupied = self.stat_space_occupied.get();
        let allocated = self.stat_space_allocated.get();
        let blocks = self.blocks.borrow().len();

        (occupied, allocated, blocks)
    }

    /// Frees all memory currently allocated by the arena, resetting itself to start
    /// fresh.
    ///
    /// CAUTION: this is unsafe because it does NOT ensure that all references to the data are
    /// gone, so this can potentially lead to dangling references.
    pub unsafe fn free_all_and_reset(&self) {
        let mut blocks = self.blocks.borrow_mut();

        blocks.clear();
        blocks.shrink_to_fit();
        blocks.push(Vec::with_capacity(self.min_block_size));

        self.stat_space_occupied.set(self.min_block_size);
        self.stat_space_allocated.set(0);
    }

    /// Allocates memory for and initializes a type T, returning a mutable reference to it.
    pub fn alloc<T: Copy>(&self, value: T) -> &mut T {
        let memory = self.alloc_uninitialized();
        unsafe {
            *memory.as_mut_ptr() = value;
        }
        unsafe { transmute(memory) }
    }

    /// Allocates memory for and initializes a type T, returning a mutable reference to it.
    ///
    /// Additionally, the allocation will be made with the given byte alignment or
    /// the type's inherent alignment, whichever is greater.
    pub fn alloc_with_alignment<T: Copy>(&self, value: T, align: usize) -> &mut T {
        let memory = self.alloc_uninitialized_with_alignment(align);
        unsafe {
            *memory.as_mut_ptr() = value;
        }
        unsafe { transmute(memory) }
    }

    /// Allocates memory for a type `T`, returning a mutable reference to it.
    ///
    /// CAUTION: the memory returned is uninitialized.  Make sure to initalize before using!
    pub fn alloc_uninitialized<T: Copy>(&self) -> &mut MaybeUninit<T> {
        assert!(size_of::<T>() > 0);

        let memory = self.alloc_raw(size_of::<T>(), align_of::<T>()) as *mut MaybeUninit<T>;

        unsafe { memory.as_mut().unwrap() }
    }

    /// Allocates memory for a type `T`, returning a mutable reference to it.
    ///
    /// Additionally, the allocation will be made with the given byte alignment or
    /// the type's inherent alignment, whichever is greater.
    ///
    /// CAUTION: the memory returned is uninitialized.  Make sure to initalize before using!
    pub fn alloc_uninitialized_with_alignment<T: Copy>(&self, align: usize) -> &mut MaybeUninit<T> {
        assert!(size_of::<T>() > 0);

        let memory =
            self.alloc_raw(size_of::<T>(), max(align, align_of::<T>())) as *mut MaybeUninit<T>;

        unsafe { memory.as_mut().unwrap() }
    }

    /// Allocates memory for `len` values of type `T`, returning a mutable slice to it.
    /// All elements are initialized to the given `value`.
    pub fn alloc_array<T: Copy>(&self, len: usize, value: T) -> &mut [T] {
        let memory = self.alloc_array_uninitialized(len);

        for v in memory.iter_mut() {
            unsafe {
                *v.as_mut_ptr() = value;
            }
        }

        unsafe { transmute(memory) }
    }

    /// Allocates memory for `len` values of type `T`, returning a mutable slice to it.
    /// All elements are initialized to the given `value`.
    ///
    /// Additionally, the allocation will be made with the given byte alignment or
    /// the type's inherent alignment, whichever is greater.
    pub fn alloc_array_with_alignment<T: Copy>(
        &self,
        len: usize,
        value: T,
        align: usize,
    ) -> &mut [T] {
        let memory = self.alloc_array_uninitialized_with_alignment(len, align);

        for v in memory.iter_mut() {
            unsafe {
                *v.as_mut_ptr() = value;
            }
        }

        unsafe { transmute(memory) }
    }

    /// Allocates and initializes memory to duplicate the given slice, returning a mutable slice
    /// to the new copy.
    pub fn copy_slice<T: Copy>(&self, other: &[T]) -> &mut [T] {
        let memory = self.alloc_array_uninitialized(other.len());

        for (v, other) in memory.iter_mut().zip(other.iter()) {
            unsafe {
                *v.as_mut_ptr() = *other;
            }
        }

        unsafe { transmute(memory) }
    }

    /// Allocates and initializes memory to duplicate the given slice, returning a mutable slice
    /// to the new copy.
    ///
    /// Additionally, the allocation will be made with the given byte alignment or
    /// the type's inherent alignment, whichever is greater.
    pub fn copy_slice_with_alignment<T: Copy>(&self, other: &[T], align: usize) -> &mut [T] {
        let memory = self.alloc_array_uninitialized_with_alignment(other.len(), align);

        for (v, other) in memory.iter_mut().zip(other.iter()) {
            unsafe {
                *v.as_mut_ptr() = *other;
            }
        }

        unsafe { transmute(memory) }
    }

    /// Allocates memory for `len` values of type `T`, returning a mutable slice to it.
    ///
    /// CAUTION: the memory returned is uninitialized.  Make sure to initalize before using!
    pub fn alloc_array_uninitialized<T: Copy>(&self, len: usize) -> &mut [MaybeUninit<T>] {
        assert!(size_of::<T>() > 0);

        let array_mem_size = {
            let alignment_padding = alignment_offset(size_of::<T>(), align_of::<T>());
            let aligned_type_size = size_of::<T>() + alignment_padding;
            aligned_type_size * len
        };

        let memory = self.alloc_raw(array_mem_size, align_of::<T>()) as *mut MaybeUninit<T>;

        unsafe { slice::from_raw_parts_mut(memory, len) }
    }

    /// Allocates memory for `len` values of type `T`, returning a mutable slice to it.
    ///
    /// Additionally, the allocation will be made with the given byte alignment or
    /// the type's inherent alignment, whichever is greater.
    ///
    /// CAUTION: the memory returned is uninitialized.  Make sure to initalize before using!
    pub fn alloc_array_uninitialized_with_alignment<T: Copy>(
        &self,
        len: usize,
        align: usize,
    ) -> &mut [MaybeUninit<T>] {
        assert!(size_of::<T>() > 0);

        let array_mem_size = {
            let alignment_padding = alignment_offset(size_of::<T>(), align_of::<T>());
            let aligned_type_size = size_of::<T>() + alignment_padding;
            aligned_type_size * len
        };

        let memory =
            self.alloc_raw(array_mem_size, max(align, align_of::<T>())) as *mut MaybeUninit<T>;

        unsafe { slice::from_raw_parts_mut(memory, len) }
    }

    /// Allocates space with a given size and alignment.
    ///
    /// This is the work-horse code of the MemArena.
    ///
    /// CAUTION: this returns uninitialized memory.  Make sure to initialize the
    /// memory after calling.
    fn alloc_raw(&self, size: usize, alignment: usize) -> *mut MaybeUninit<u8> {
        assert!(alignment > 0);

        self.stat_space_allocated
            .set(self.stat_space_allocated.get() + size); // Update stats

        let mut blocks = self.blocks.borrow_mut();

        // If it's a zero-size allocation, just point to the beginning of the current block.
        if size == 0 {
            return blocks.first_mut().unwrap().as_mut_ptr();
        }
        // If it's non-zero-size.
        else {
            let start_index = {
                let block_addr = blocks.first().unwrap().as_ptr() as usize;
                let block_filled = blocks.first().unwrap().len();
                block_filled + alignment_offset(block_addr + block_filled, alignment)
            };

            // If it will fit in the current block, use the current block.
            if (start_index + size) <= blocks.first().unwrap().capacity() {
                unsafe {
                    blocks.first_mut().unwrap().set_len(start_index + size);
                }

                let block_ptr = blocks.first_mut().unwrap().as_mut_ptr();
                return unsafe { block_ptr.add(start_index) };
            }
            // If it won't fit in the current block, create a new block and use that.
            else {
                let next_size = if blocks.len() >= GROWTH_FRACTION {
                    let a = self.stat_space_occupied.get() / GROWTH_FRACTION;
                    let b = a % self.min_block_size;
                    if b > 0 {
                        a - b + self.min_block_size
                    } else {
                        a
                    }
                } else {
                    self.min_block_size
                };

                let waste_percentage = {
                    let w1 =
                        ((blocks[0].capacity() - blocks[0].len()) * 100) / blocks[0].capacity();
                    let w2 = ((self.stat_space_occupied.get() - self.stat_space_allocated.get())
                        * 100)
                        / self.stat_space_occupied.get();
                    if w1 < w2 {
                        w1
                    } else {
                        w2
                    }
                };

                // If it's a "large allocation", give it its own memory block.
                if (size + alignment) > next_size || waste_percentage > self.max_waste_percentage {
                    // Update stats
                    self.stat_space_occupied
                        .set(self.stat_space_occupied.get() + size + alignment - 1);

                    blocks.push(Vec::with_capacity(size + alignment - 1));
                    unsafe {
                        blocks.last_mut().unwrap().set_len(size + alignment - 1);
                    }

                    let start_index =
                        alignment_offset(blocks.last().unwrap().as_ptr() as usize, alignment);

                    let block_ptr = blocks.last_mut().unwrap().as_mut_ptr();
                    return unsafe { block_ptr.add(start_index) };
                }
                // Otherwise create a new shared block.
                else {
                    // Update stats
                    self.stat_space_occupied
                        .set(self.stat_space_occupied.get() + next_size);

                    blocks.push(Vec::with_capacity(next_size));
                    let block_count = blocks.len();
                    blocks.swap(0, block_count - 1);

                    let start_index =
                        alignment_offset(blocks.first().unwrap().as_ptr() as usize, alignment);

                    unsafe {
                        blocks.first_mut().unwrap().set_len(start_index + size);
                    }

                    let block_ptr = blocks.first_mut().unwrap().as_mut_ptr();
                    return unsafe { block_ptr.add(start_index) };
                }
            }
        }
    }
}
