//! A seedable, Owen-scrambled Sobol sequence.
//!
//! This implementation is limited to `2^16` samples, and will loop back to
//! the start of the sequence after that limit.

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
    // independent Sobol sequence.  This index shuffling approach is due to
    // Brent Burley.
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

/// Scrambles `n` using the Laine Karras hash.  This is equivalent to Owen
/// scrambling, but on reversed bits.
#[inline]
fn lk_scramble(mut n: u32, scramble: u32) -> u32 {
    // This uses essentially the same technique as presented in the paper
    // "Stratified Sampling for Stochastic Transparency" by Laine and Karras,
    // but with a faster, higher quality hash function.
    //
    // The basic idea is that we're running a special kind of hash function
    // that only allows avalanche to happen upwards (i.e. a bit is only
    // affected by the bits lower than it).  This is achieved by only doing
    // mixing via operations that also adhere to that property.
    //
    // Normally this would be considered a poor hash function, because normally
    // you want all bits to have an equal chance of affecting all other bits.
    // But in this case that only-upward behavior is exactly what we want,
    // because it ends up being equivalent to Owen scrambling on
    // reverse-ordered bits.

    n = n.wrapping_add(hash(scramble, 2));

    n ^= 0xdc967795;
    n = n.wrapping_mul(0x97b754b7);
    n ^= 0x866350b1;
    n = n.wrapping_mul(0x9e3779cd);

    n
}

/// Same as `lk_scramble()`, except does it on 4 integers at a time.
#[inline(always)]
fn lk_scramble_int4(mut n: Int4, scramble: u32) -> Int4 {
    n += hash_int4([scramble; 4].into(), 2);

    n ^= [0xdc967795; 4].into();
    n *= [0x97b754b7; 4].into();
    n ^= [0x866350b1; 4].into();
    n *= [0x9e3779cd; 4].into();

    n
}

/// A simple 32-bit hash function.  Its quality can be tuned with
/// the number of rounds used.
#[inline(always)]
fn hash(n: u32, rounds: u32) -> u32 {
    let mut hash = n ^ 0x79c68e4a;
    for _ in 0..rounds {
        hash = hash.wrapping_mul(0x736caf6f);
        hash ^= hash.wrapping_shr(16);
    }
    hash
}

/// Same as `hash()` except performs hashing on four numbers at once.
///
/// Each of the four numbers gets a different hash, so even if all input
/// numbers are the same, the outputs will still be different for each of them.
#[inline(always)]
fn hash_int4(n: Int4, rounds: u32) -> Int4 {
    let mut hash = n;
    hash ^= [0x912f69ba, 0x174f18ab, 0x691e72ca, 0xb40cc1b8].into();
    for _ in 0..rounds {
        hash *= [0x736caf6f; 4].into();
        hash ^= hash.shr16();
    }
    hash
}
