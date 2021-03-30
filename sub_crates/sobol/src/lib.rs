//! A seedable, Owen-scrambled Sobol sequence.
//!
//! This is based on the paper "Practical Hash-based Owen Scrambling"
//! by Brent Burley, but with a novel scramble function in place of the
//! Laine-Karras function used in the paper, and with a larger set of direction
//! numbers due to Kuo et al.
//!
//! This implementation is limited to `2^16` samples, and will loop back
//! to the start of the sequence after that limit.

#![allow(clippy::unreadable_literal)]

mod wide;
use wide::Int4;

// This `include` provides `MAX_DIMENSION` and `REV_VECTORS`.
// See the build.rs file for how this included file is generated.
include!(concat!(env!("OUT_DIR"), "/vectors.inc"));

pub const MAX_DIMENSION_SET: u32 = MAX_DIMENSION / 4;

/// Compute four dimensions of a single sample in the Sobol sequence.
///
/// `sample_index` specifies which sample in the Sobol sequence to compute.
///
/// `dimension_set` specifies which four dimensions to compute. `0` yields the
/// first four dimensions, `1` the second four dimensions, and so on.
///
/// `seed` produces statistically independent Sobol sequences.  Passing two
/// different seeds will produce two different sequences that are only randomly
/// associated, with no stratification or correlation between them.
#[inline]
pub fn sample_4d(sample_index: u32, dimension_set: u32, seed: u32) -> [f32; 4] {
    assert!(dimension_set < MAX_DIMENSION_SET);
    let vecs = &REV_VECTORS[dimension_set as usize];

    // Shuffle the index using the given seed to produce a unique statistically
    // independent Sobol sequence.
    let shuffled_rev_index = lk_scramble(sample_index.reverse_bits(), seed);

    // Compute the Sobol sample with reversed bits.
    let mut sobol_rev = Int4::zero();
    let mut index = shuffled_rev_index & 0xffff0000; // Only use the top 16 bits.
    let mut i = 0;
    while index != 0 {
        let j = index.leading_zeros();
        sobol_rev ^= vecs[(i + j) as usize].into();
        i += j + 1;
        index <<= j;
        index <<= 1;
    }

    // Do Owen scrambling on the reversed-bits Sobol sample.
    let sobol_owen_rev = lk_scramble_int4(sobol_rev, dimension_set ^ seed);

    // Un-reverse the bits and convert to floating point in [0, 1).
    sobol_owen_rev.reverse_bits().to_norm_floats()
}

//----------------------------------------------------------------------

/// Scrambles `n` using a novel variation on the Laine-Karras hash.
///
/// This is equivalent to Owen scrambling, but on reversed bits.
#[inline(always)]
fn lk_scramble(mut n: u32, scramble: u32) -> u32 {
    let scramble = hash(scramble);

    n = n.wrapping_add(n << 2);
    n ^= n.wrapping_mul(0xfe9b5742);
    n = n.wrapping_add(scramble);
    n = n.wrapping_mul((scramble >> 16) | 1);

    n
}

/// Same as `lk_scramble()`, except does it on 4 integers at a time.
#[inline(always)]
fn lk_scramble_int4(mut n: Int4, scramble: u32) -> Int4 {
    let scramble = hash_int4([scramble; 4].into());

    n += n << 2;
    n ^= n * [0xfe9b5742; 4].into();
    n += scramble;
    n *= (scramble >> 16) | [1; 4].into();

    n
}

/// A good 32-bit hash function.
/// From https://github.com/skeeto/hash-prospector
#[inline(always)]
fn hash(n: u32) -> u32 {
    let mut hash = n ^ 0x79c68e4a;

    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb352d);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846ca68b);
    hash ^= hash >> 16;

    hash
}

/// Same as `hash()` except performs hashing on four numbers at once.
///
/// Each of the four numbers gets a different hash, so even if all input
/// numbers are the same, the outputs will still be different for each of them.
#[inline(always)]
fn hash_int4(n: Int4) -> Int4 {
    let mut hash = n ^ [0x912f69ba, 0x174f18ab, 0x691e72ca, 0xb40cc1b8].into();

    hash ^= hash >> 16;
    hash *= [0x7feb352d; 4].into();
    hash ^= hash >> 15;
    hash *= [0x846ca68b; 4].into();
    hash ^= hash >> 16;

    hash
}
