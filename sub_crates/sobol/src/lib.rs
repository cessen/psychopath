// Copyright (c) 2012 Leonhard Gruenschloss (leonhard@gruenschloss.org)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is furnished to do
// so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// Adapted to Rust by Nathan Vegdahl (2017)

mod matrices;

pub use matrices::NUM_DIMENSIONS;
use matrices::{SIZE, MATRICES};

/// Compute one component of the Sobol'-sequence, where the component
/// corresponds to the dimension parameter, and the index specifies
/// the point inside the sequence. The scramble parameter can be used
/// to permute elementary intervals, and might be chosen randomly to
/// generate a randomized QMC sequence.
#[inline]
pub fn sample_with_scramble(dimension: u32, mut index: u32, scramble: u32) -> f32 {
    assert!((dimension as usize) < NUM_DIMENSIONS);

    let mut result = scramble;
    let mut i = (dimension as usize) * SIZE;
    while index != 0 {
        if (index & 1) != 0 {
            result ^= MATRICES[i];
        }

        index >>= 1;
        i += 1;
    }

    return result as f32 * (1.0 / (1u64 << 32) as f32);
}

#[inline]
pub fn sample(dimension: u32, index: u32) -> f32 {
    sample_with_scramble(dimension, index, 0)
}
