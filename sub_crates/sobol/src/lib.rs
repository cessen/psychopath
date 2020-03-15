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

// Adapted to Rust by Nathan Vegdahl (2017).
// Owen scrambling implementation also by Nathan Vegdahl (2020).

mod matrices;

pub use crate::matrices::NUM_DIMENSIONS;
use crate::matrices::{MATRICES, SIZE};

/// Compute one component of one sample from the Sobol'-sequence, where
/// `dimension` specifies the component and `index` specifies the sample
/// within the sequence.
#[inline]
pub fn sample(dimension: u32, index: u32) -> f32 {
    u32_to_0_1_f32(sobol_u32(dimension, index))
}

/// Same as `sample()` except applies random digit scrambling using the
/// scramble parameter.
///
/// To get proper random digit scrambling, you need to use a different scramble
/// value for each dimension, and those values should be generated more-or-less
/// randomly.  For example, using a 32-bit hash of the dimension parameter
/// works well.
#[inline]
pub fn sample_rd_scramble(dimension: u32, index: u32, scramble: u32) -> f32 {
    u32_to_0_1_f32(sobol_u32(dimension, index) ^ scramble)
}

/// Same as `sample()` except applies Owen scrambling using the given scramble
/// parameter.
///
/// To get proper Owen scrambling, you need to use a different scramble
/// value for each dimension, and those values should be generated more-or-less
/// randomly.  For example, using a 32-bit hash of the dimension parameter
/// works well.
#[inline]
pub fn sample_owen_scramble(dimension: u32, index: u32, scramble: u32) -> f32 {
    // Get the sobol point.
    let mut n = sobol_u32(dimension, index);

    // Do Owen scrambling.
    //
    // This uses the technique presented in the paper "Stratified Sampling for
    // Stochastic Transparency" by Laine and Karras.
    // The basic idea is that we're running a special kind of hash function
    // that only allows avalanche to happen downwards (i.e. a bit is only
    // affected by the bits higher than it).  This is achieved by first
    // reversing the bits and then doing mixing via multiplication by even
    // numbers.
    //
    // Normally this would be considered a poor hash function, because normally
    // you want all bits to have an equal chance of affecting all other bits.
    // But in this case that only-downward behavior is exactly what we want,
    // because it ends up being equivalent to Owen scrambling.
    //
    // Note that the application of the scramble parameter here is equivalent
    // to doing random digit scrambling.  This is valid because random digit
    // scrambling is a strict subset of Owen scrambling, and therefore does
    // not invalidate the Owen scrambling itself.
    //
    // The constants here were selected through an optimization process
    // to maximize unidirectional low-bias avalanche between bits.
    n = n.reverse_bits();
    n ^= scramble; // Apply the scramble parameter.
    n ^= n.wrapping_mul(0x08afbbe0);
    n ^= n.wrapping_mul(0xa7389b46);
    n ^= n.wrapping_mul(0x42bf6dbc);
    n = n.reverse_bits();

    u32_to_0_1_f32(n)
}

//----------------------------------------------------------------------

/// The actual core Sobol samplng code.  Used by the other functions.
#[inline(always)]
fn sobol_u32(dimension: u32, mut index: u32) -> u32 {
    assert!((dimension as usize) < NUM_DIMENSIONS);

    let mut result = 0;
    let mut i = (dimension as usize) * SIZE;
    while index != 0 {
        if (index & 1) != 0 {
            result ^= MATRICES[i];
        }

        index >>= 1;
        i += 1;
    }

    result
}

#[inline(always)]
fn u32_to_0_1_f32(n: u32) -> f32 {
    n as f32 * (1.0 / (1u64 << 32) as f32)
}
