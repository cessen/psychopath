#![allow(dead_code)]

use std::mem::size_of;

#[derive(Copy, Clone, Debug)]
pub struct BitStack128 {
    data: (u64, u64),
}

impl BitStack128 {
    pub fn new() -> BitStack128 {
        BitStack128 { data: (0, 0) }
    }

    pub fn new_with_1() -> BitStack128 {
        BitStack128 { data: (1, 0) }
    }

    /// Push a bit onto the top of the stack.
    pub fn push(&mut self, value: bool) {
        debug_assert!((self.data.1 >> (size_of::<u64>() - 1)) == 0); // Verify no stack overflow
        self.data.1 = (self.data.1 << 1) | (self.data.0 >> (size_of::<u64>() - 1));
        self.data.0 <<= 1;
        self.data.0 |= value as u64;
    }

    /// Push n bits onto the top of the stack.  The input
    /// bits are passed as an integer, with the bit that
    /// will be on top in the least significant digit, and
    /// the rest following in order from there.
    ///
    /// Note that unless you are running a debug build, no
    /// effort is made to verify that only the first n
    /// bits of the passed value are used.  So if other
    /// bits are non-zero this will produce incorrect results.
    pub fn push_n(&mut self, value: u8, count: u8) {
        // Verify no bitstack overflow
        debug_assert!((self.data.1 >> (size_of::<u64>() - count as usize)) == 0);
        // Verify no bits outside of the n-bit range
        debug_assert!(value & (!((1 << count) - 1)) == 0);

        self.data.1 = (self.data.1 << count as usize) |
                      (self.data.0 >> (size_of::<u64>() - count as usize));
        self.data.0 <<= count as u64;
        self.data.0 |= value as u64;
    }

    /// Pop the top bit off the stack.
    pub fn pop(&mut self) -> bool {
        let b = (self.data.0 & 1) != 0;
        self.data.0 = (self.data.0 >> 1) | (self.data.1 << (size_of::<u64>() - 1));
        self.data.1 >>= 1;
        return b;
    }

    /// Pop the top n bits off the stack.  The bits are returned as
    /// an integer, with the top bit in the least significant digit,
    /// and the rest following in order from there.
    pub fn pop_n(&mut self, n: usize) -> u64 {
        debug_assert!(n < size_of::<BitStack128>()); // Can't pop more than we have
        debug_assert!(n < size_of::<u64>()); // Can't pop more than the return type can hold
        let b = self.data.0 & ((1 << n) - 1);
        self.data.0 = (self.data.0 >> n) | (self.data.1 << (size_of::<u64>() - n));
        self.data.1 >>= n;
        return b;
    }

    /// Read the top bit of the stack without popping it.
    pub fn peek(&self) -> bool {
        (self.data.0 & 1) != 0
    }

    /// Read the top n bits of the stack without popping them.  The bits
    /// are returned as an integer, with the top bit in the least
    /// significant digit, and the rest following in order from there.
    pub fn peek_n(&self, n: usize) -> u64 {
        debug_assert!(n < size_of::<BitStack128>()); // Can't return more than we have
        debug_assert!(n < size_of::<u64>()); // Can't return more than the return type can hold
        self.data.0 & ((1 << n) - 1)
    }
}
