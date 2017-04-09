use std::slice;
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
/// size (the large allocation threshold) are given their own block.
///
/// The block size and large allocation threshold size are configurable.
pub struct MemArena {
    blocks: Vec<Vec<u8>>,
    block_size: usize,
    large_alloc_threshold: usize,
}

impl MemArena {
    /// Create a new arena, with default block size and large allocation threshold
    pub fn new() -> MemArena {
        MemArena {
            blocks: vec![Vec::with_capacity(DEFAULT_BLOCK_SIZE)],
            block_size: DEFAULT_BLOCK_SIZE,
            large_alloc_threshold: DEFAULT_LARGE_ALLOCATION_THRESHOLD,
        }
    }

    pub fn new_with_settings(block_size: usize, large_alloc_threshold: usize) -> MemArena {
        assert!(large_alloc_threshold <= block_size);

        MemArena {
            blocks: vec![Vec::with_capacity(block_size)],
            block_size: block_size,
            large_alloc_threshold: large_alloc_threshold,
        }
    }

    /// Frees all memory currently allocated by the arena, resetting itself to start
    /// fresh.
    ///
    /// CAUTION: this is unsafe because it does NOT ensure that all references to the data are
    /// gone, so this can potentially lead to dangling references.
    pub unsafe fn free_all_and_reset(&mut self) {
        self.blocks.clear();
        self.blocks.shrink_to_fit();
        self.blocks.push(Vec::with_capacity(self.block_size));
    }

    /// Allocates memory for and initializes a type T, returning a mutable reference to it.
    pub fn alloc<'a, T: Copy>(&'a mut self, value: T) -> &'a mut T {
        let mut memory = unsafe { self.alloc_uninitialized() };
        *memory = value;
        memory
    }

    /// Allocates memory for a type `T`, returning a mutable reference to it.
    ///
    /// CAUTION: the memory returned is uninitialized.  Make sure to initalize before using!
    pub unsafe fn alloc_uninitialized<'a, T: Copy>(&'a mut self) -> &'a mut T {
        let memory = self.alloc_space(size_of::<T>(), align_of::<T>()) as *mut T;

        memory.as_mut().unwrap()
    }

    /// Allocates memory for `len` values of type `T`, returning a mutable slice to it.
    /// All elements are initialized to the given `value`.
    pub fn alloc_array<'a, T: Copy>(&'a mut self, len: usize, value: T) -> &'a mut [T] {
        let memory = unsafe { self.alloc_array_uninitialized(len) };

        for v in memory.iter_mut() {
            *v = value;
        }

        memory
    }

    /// Allocates memory for `len` values of type `T`, returning a mutable slice to it.
    /// All elements are initialized to the given `value`.
    pub unsafe fn alloc_array_uninitialized<'a, T: Copy>(&'a mut self, len: usize) -> &'a mut [T] {
        let array_mem_size = {
            let alignment_padding = alignment_offset(size_of::<T>(), align_of::<T>());
            let aligned_type_size = size_of::<T>() + alignment_padding;
            aligned_type_size * len
        };

        let memory = self.alloc_space(array_mem_size, align_of::<T>()) as *mut T;

        slice::from_raw_parts_mut(memory, len)
    }

    /// Allocates space with a given size and alignment.
    ///
    /// CAUTION: this is unsafe because it returns uninitialized memory.
    /// Make sure to initialize the memory after calling.
    unsafe fn alloc_space<'a>(&'a mut self, size: usize, alignment: usize) -> *mut u8 {
        assert!(size > 0);
        assert!(alignment > 0);

        // If the desired size is considered a "large allocation", give it its own memory block.
        if size > self.large_alloc_threshold {
            self.blocks.push(Vec::with_capacity(size + alignment - 1));
            self.blocks.last_mut().unwrap().set_len(size + alignment - 1);

            let start_index = alignment_offset(self.blocks.last().unwrap().as_ptr() as usize,
                                               alignment);

            let block_ptr = self.blocks.last_mut().unwrap().as_mut_ptr();
            return block_ptr.offset(start_index as isize);
        }
        // If the desired size is not a "large allocation", try to fit it into the current
        // block, and only create a new block if doesn't fit.
        else {
            let start_index = {
                let block_addr = self.blocks.first().unwrap().as_ptr() as usize;
                let block_filled = self.blocks.first().unwrap().len();
                block_filled + alignment_offset(block_addr + block_filled, alignment)
            };

            // If it will fit in the current block, use the current block.
            if (start_index + size) <= self.blocks.first().unwrap().capacity() {
                self.blocks.first_mut().unwrap().set_len(start_index + size);

                let block_ptr = self.blocks.first_mut().unwrap().as_mut_ptr();
                return block_ptr.offset(start_index as isize);
            }
            // If it won't fit in the current block, create a new block and use that.
            else {
                self.blocks.push(Vec::with_capacity(self.block_size));
                let block_count = self.blocks.len();
                self.blocks.swap(0, block_count - 1);

                let start_index = alignment_offset(self.blocks.first().unwrap().as_ptr() as usize,
                                                   alignment);

                self.blocks.first_mut().unwrap().set_len(start_index + size);

                let block_ptr = self.blocks.first_mut().unwrap().as_mut_ptr();
                return block_ptr.offset(start_index as isize);
            }
        }
    }
}
