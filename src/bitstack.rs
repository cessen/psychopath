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
        // Verify no stack overflow
        debug_assert!((self.data.1 >> ((size_of::<u64>() * 8) - 1)) == 0);

        self.data.1 = (self.data.1 << 1) | (self.data.0 >> ((size_of::<u64>() * 8) - 1));
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
        debug_assert!((self.data.1 >> ((size_of::<u64>() * 8) - count as usize)) == 0);
        // Verify no bits outside of the n-bit range
        debug_assert!(if count < (size_of::<u8>() * 8) as u8 {
            value & (!((1 << count) - 1)) == 0
        } else {
            true
        });
        debug_assert!(count <= (size_of::<u8>() * 8) as u8);

        self.data.1 = (self.data.1 << count as usize) |
                      (self.data.0 >> ((size_of::<u64>() * 8) - count as usize));
        self.data.0 <<= count as u64;
        self.data.0 |= value as u64;
    }

    /// Pop the top bit off the stack.
    pub fn pop(&mut self) -> bool {
        let b = (self.data.0 & 1) != 0;
        self.data.0 = (self.data.0 >> 1) | (self.data.1 << ((size_of::<u64>() * 8) - 1));
        self.data.1 >>= 1;
        return b;
    }

    /// Pop the top n bits off the stack.  The bits are returned as
    /// an integer, with the top bit in the least significant digit,
    /// and the rest following in order from there.
    pub fn pop_n(&mut self, n: usize) -> u64 {
        debug_assert!(n < (size_of::<BitStack128>() * 8)); // Can't pop more than we have
        debug_assert!(n < (size_of::<u64>() * 8)); // Can't pop more than the return type can hold
        let b = self.data.0 & ((1 << n) - 1);
        self.data.0 = (self.data.0 >> n) | (self.data.1 << ((size_of::<u64>() * 8) - n));
        self.data.1 >>= n;
        return b;
    }

    /// Pop the top n bits off the stack, but return only the nth bit.
    pub fn pop_to_nth(&mut self, n: usize) -> bool {
        debug_assert!(n > 0);
        debug_assert!(n < (size_of::<BitStack128>() * 8)); // Can't pop more than we have
        debug_assert!(n < (size_of::<u64>() * 8)); // Can't pop more than the return type can hold
        let b = (self.data.0 & (1 << (n - 1))) != 0;
        self.data.0 = (self.data.0 >> n) | (self.data.1 << ((size_of::<u64>() * 8) - n));
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
        // Can't return more than we have
        debug_assert!(n < (size_of::<BitStack128>() * 8));
        // Can't return more than the return type can hold
        debug_assert!(n < (size_of::<u64>() * 8));

        self.data.0 & ((1 << n) - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push() {
        let mut bs = BitStack128::new();
        bs.push(true);
        bs.push(false);
        bs.push(true);
        bs.push(true);
        bs.push(false);
        bs.push(true);
        bs.push(true);
        bs.push(true);

        assert!(bs.data.0 == 0b10110111);
        assert!(bs.data.1 == 0);
    }

    #[test]
    fn push_overflow() {
        let mut bs = BitStack128::new();
        for _ in 0..9 {
            bs.push(true);
            bs.push(false);
            bs.push(true);
            bs.push(true);
            bs.push(false);
            bs.push(true);
            bs.push(true);
            bs.push(true);
        }

        assert!(bs.data.0 == 0b1011011110110111101101111011011110110111101101111011011110110111);
        assert!(bs.data.1 == 0b10110111);
    }

    #[test]
    fn pop() {
        let mut bs = BitStack128::new();
        bs.data.0 = 0b10110111;

        assert!(bs.pop() == true);
        assert!(bs.pop() == true);
        assert!(bs.pop() == true);
        assert!(bs.pop() == false);
        assert!(bs.pop() == true);
        assert!(bs.pop() == true);
        assert!(bs.pop() == false);
        assert!(bs.pop() == true);
    }

    #[test]
    fn pop_overflow() {
        let mut bs = BitStack128::new();
        bs.data.0 = 0b1011011110110111101101111011011110110111101101111011011110110111;
        bs.data.1 = 0b10110111;
        for _ in 0..9 {
            assert!(bs.pop() == true);
            assert!(bs.pop() == true);
            assert!(bs.pop() == true);
            assert!(bs.pop() == false);
            assert!(bs.pop() == true);
            assert!(bs.pop() == true);
            assert!(bs.pop() == false);
            assert!(bs.pop() == true);
        }
    }

    #[test]
    fn push_n() {
        let mut bs = BitStack128::new();
        bs.push_n(0b10110, 5);
        bs.push_n(0b10110111, 8);

        assert!(bs.data.0 == 0b1011010110111);
    }

    #[test]
    fn push_n_overflow() {
        let mut bs = BitStack128::new();
        for _ in 0..9 {
            bs.push_n(0b10110111, 8);
        }

        assert!(bs.data.0 == 0b1011011110110111101101111011011110110111101101111011011110110111);
        assert!(bs.data.1 == 0b10110111);
    }

    #[test]
    fn pop_n() {
        let mut bs = BitStack128::new();
        bs.data.0 = 0b0010_1000_1100_1110_0101_0111;

        assert!(bs.pop_n(4) == 0b0111);
        assert!(bs.data.0 == 0b0010_1000_1100_1110_0101);

        assert!(bs.pop_n(4) == 0b0101);
        assert!(bs.data.0 == 0b0010_1000_1100_1110);

        assert!(bs.pop_n(4) == 0b1110);
        assert!(bs.data.0 == 0b0010_1000_1100);

        assert!(bs.pop_n(4) == 0b1100);
        assert!(bs.data.0 == 0b0010_1000);

        assert!(bs.pop_n(4) == 0b1000);
        assert!(bs.data.0 == 0b0010);

        assert!(bs.pop_n(4) == 0b0010);
        assert!(bs.data.0 == 0);
    }

    #[test]
    fn pop_n_overflow() {
        let mut bs = BitStack128::new();
        bs.data.0 = 0b1011011110110111101101111011011110110111101101111011011110110111;
        bs.data.1 = 0b10110111;
        for _ in 0..9 {
            assert!(bs.pop_n(8) == 0b10110111);
        }
    }

    #[test]
    fn pop_to_nth() {
        let mut bs = BitStack128::new();
        bs.data.0 = 0b0010_1000_1100_1110_0101_0111;

        assert!(bs.pop_to_nth(4) == false);
        assert!(bs.data.0 == 0b0010_1000_1100_1110_0101);

        assert!(bs.pop_to_nth(4) == false);
        assert!(bs.data.0 == 0b0010_1000_1100_1110);

        assert!(bs.pop_to_nth(4) == true);
        assert!(bs.data.0 == 0b0010_1000_1100);

        assert!(bs.pop_to_nth(4) == true);
        assert!(bs.data.0 == 0b0010_1000);

        assert!(bs.pop_to_nth(4) == true);
        assert!(bs.data.0 == 0b0010);

        assert!(bs.pop_to_nth(4) == false);
        assert!(bs.data.0 == 0);
    }

    #[test]
    fn pop_to_nth_overflow() {
        let mut bs = BitStack128::new();
        bs.data.0 = 0b00110111_10110111_00110111_10110111_00110111_10110111_00110111_10110111;
        bs.data.1 = 0b00110111_10110111;
        for _ in 0..5 {
            assert!(bs.pop_to_nth(8) == true);
            assert!(bs.pop_to_nth(8) == false);
        }
    }

    #[test]
    fn peek() {
        let mut bs = BitStack128::new();
        bs.data.0 = 0b10110111;

        assert!(bs.peek() == true);
        bs.pop();

        assert!(bs.peek() == true);
        bs.pop();

        assert!(bs.peek() == true);
        bs.pop();

        assert!(bs.peek() == false);
        bs.pop();

        assert!(bs.peek() == true);
        bs.pop();

        assert!(bs.peek() == true);
        bs.pop();

        assert!(bs.peek() == false);
        bs.pop();

        assert!(bs.peek() == true);
    }

    #[test]
    fn peek_n() {
        let mut bs = BitStack128::new();
        bs.data.0 = 0b10110111;

        assert!(bs.peek_n(4) == 0b0111);
        bs.pop_n(4);

        assert!(bs.peek_n(4) == 0b1011);
        bs.pop_n(4);
    }
}
