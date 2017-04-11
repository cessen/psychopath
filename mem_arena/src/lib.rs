use std::slice;
use std::cell::{Cell, RefCell};
use std::mem::{size_of, align_of};

const DEFAULT_BLOCK_SIZE: usize = (1 << 20) * 32; // 32 MiB
const DEFAULT_LARGE_ALLOCATION_THRESHOLD: usize = 1 << 20;

fn alignment_offset(addr: usize, alignment: usize) -> usize {
    (alignment - (addr % alignment)) % alignment
}

/// A growable memory arena for Copy types.
///
/// The arena works by allocating memory in blocks of a fixed size.  It doles
/// out memory from the current block until an amount of memory is requested that
/// doesn't fit in the remainder of the current block, and then allocates a new
/// block.
///
/// Additionally, to minimize unused space in blocks, allocations above a specified
/// size (the large allocation threshold) are given their own block if they won't
/// fit in the remainder of the current block.
///
/// The block size and large allocation threshold are configurable.
#[derive(Debug)]
pub struct MemArena {
    blocks: RefCell<Vec<Vec<u8>>>,
    block_size: usize,
    large_alloc_threshold: usize,
    stat_space_occupied: Cell<usize>,
    stat_space_allocated: Cell<usize>,
    stat_large_blocks: Cell<usize>,
}

impl MemArena {
    /// Create a new arena, with default block size and large allocation threshold.
    pub fn new() -> MemArena {
        MemArena {
            blocks: RefCell::new(vec![Vec::with_capacity(DEFAULT_BLOCK_SIZE)]),
            block_size: DEFAULT_BLOCK_SIZE,
            large_alloc_threshold: DEFAULT_LARGE_ALLOCATION_THRESHOLD,
            stat_space_occupied: Cell::new(DEFAULT_BLOCK_SIZE),
            stat_space_allocated: Cell::new(0),
            stat_large_blocks: Cell::new(0),
        }
    }

    /// Create a new arena, with a custom block size and large allocation threshold.
    pub fn new_with_settings(block_size: usize, large_alloc_threshold: usize) -> MemArena {
        assert!(large_alloc_threshold <= block_size);

        MemArena {
            blocks: RefCell::new(vec![Vec::with_capacity(block_size)]),
            block_size: block_size,
            large_alloc_threshold: large_alloc_threshold,
            stat_space_occupied: Cell::new(block_size),
            stat_space_allocated: Cell::new(0),
            stat_large_blocks: Cell::new(0),
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
    ///
    /// Large block count is the number of blocks that have beem specifically
    /// allocated for allocation requests that were above the large
    /// allocation threshold.
    pub fn stats(&self) -> (usize, usize, usize, usize) {
        let occupied = self.stat_space_occupied.get();
        let allocated = self.stat_space_allocated.get();
        let blocks = self.blocks.borrow().len();
        let large_blocks = self.stat_large_blocks.get();

        (occupied, allocated, blocks, large_blocks)
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
        blocks.push(Vec::with_capacity(self.block_size));

        self.stat_space_occupied.set(self.block_size);
        self.stat_space_allocated.set(0);
        self.stat_large_blocks.set(0);
    }

    /// Allocates memory for and initializes a type T, returning a mutable reference to it.
    pub fn alloc<'a, T: Copy>(&'a self, value: T) -> &'a mut T {
        let mut memory = unsafe { self.alloc_uninitialized() };
        *memory = value;
        memory
    }

    /// Allocates memory for a type `T`, returning a mutable reference to it.
    ///
    /// CAUTION: the memory returned is uninitialized.  Make sure to initalize before using!
    pub unsafe fn alloc_uninitialized<'a, T: Copy>(&'a self) -> &'a mut T {
        assert!(size_of::<T>() > 0);

        let memory = self.alloc_raw(size_of::<T>(), align_of::<T>()) as *mut T;

        memory.as_mut().unwrap()
    }

    /// Allocates memory for `len` values of type `T`, returning a mutable slice to it.
    /// All elements are initialized to the given `value`.
    pub fn alloc_array<'a, T: Copy>(&'a self, len: usize, value: T) -> &'a mut [T] {
        let memory = unsafe { self.alloc_array_uninitialized(len) };

        for v in memory.iter_mut() {
            *v = value;
        }

        memory
    }

    /// Allocates and initializes memory to duplicate the given slice, returning a mutable slice
    /// to the new copy.
    pub fn copy_slice<'a, T: Copy>(&'a self, other: &[T]) -> &'a mut [T] {
        let memory = unsafe { self.alloc_array_uninitialized(other.len()) };

        for (v, other) in memory.iter_mut().zip(other.iter()) {
            *v = *other;
        }

        memory
    }

    /// Allocates memory for `len` values of type `T`, returning a mutable slice to it.
    /// All elements are initialized to the given `value`.
    pub unsafe fn alloc_array_uninitialized<'a, T: Copy>(&'a self, len: usize) -> &'a mut [T] {
        assert!(size_of::<T>() > 0);

        let array_mem_size = {
            let alignment_padding = alignment_offset(size_of::<T>(), align_of::<T>());
            let aligned_type_size = size_of::<T>() + alignment_padding;
            aligned_type_size * len
        };

        let memory = self.alloc_raw(array_mem_size, align_of::<T>()) as *mut T;

        slice::from_raw_parts_mut(memory, len)
    }

    /// Allocates space with a given size and alignment.
    ///
    /// CAUTION: this returns uninitialized memory.  Make sure to initialize the
    /// memory after calling.
    unsafe fn alloc_raw(&self, size: usize, alignment: usize) -> *mut u8 {
        assert!(alignment > 0);

        self.stat_space_allocated.set(self.stat_space_allocated.get() + size); // Update stats

        let mut blocks = self.blocks.borrow_mut();

        // If it's a zero-size allocation, just point to the beginning of the curent block.
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
                blocks.first_mut().unwrap().set_len(start_index + size);

                let block_ptr = blocks.first_mut().unwrap().as_mut_ptr();
                return block_ptr.offset(start_index as isize);
            }
            // If it won't fit in the current block, create a new block and use that.
            else {
                // If it's a "large allocation", give it its own memory block.
                if size > self.large_alloc_threshold {
                    // Update stats
                    self.stat_space_occupied
                        .set(self.stat_space_occupied.get() + size + alignment - 1);
                    self.stat_large_blocks.set(self.stat_large_blocks.get() + 1);

                    blocks.push(Vec::with_capacity(size + alignment - 1));
                    blocks.last_mut().unwrap().set_len(size + alignment - 1);

                    let start_index = alignment_offset(blocks.last().unwrap().as_ptr() as usize,
                                                       alignment);

                    let block_ptr = blocks.last_mut().unwrap().as_mut_ptr();
                    return block_ptr.offset(start_index as isize);
                }
                // Otherwise create a new shared block.
                else {
                    // Update stats
                    self.stat_space_occupied.set(self.stat_space_occupied.get() + self.block_size);

                    blocks.push(Vec::with_capacity(self.block_size));
                    let block_count = blocks.len();
                    blocks.swap(0, block_count - 1);

                    let start_index = alignment_offset(blocks.first().unwrap().as_ptr() as usize,
                                                       alignment);

                    blocks.first_mut().unwrap().set_len(start_index + size);

                    let block_ptr = blocks.first_mut().unwrap().as_mut_ptr();
                    return block_ptr.offset(start_index as isize);
                }
            }
        }
    }
}
