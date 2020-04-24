//! An implementation of the Sobol sequence with Owen scrambling.

mod wide;
use wide::Int4;

// The following `include` provides `MAX_DIMENSION` and `REV_VECTORS`.
// See the build.rs file for how this included file is generated.
include!(concat!(env!("OUT_DIR"), "/vectors.inc"));

/// Compute one component of one sample from the Sobol sequence, where
/// `dimension` specifies the component and `index` specifies the sample
/// within the sequence.
///
/// Passing a different `seed` parameter results in a statistically
/// independent Sobol sequence, uncorrelated to others with different seeds.
///
/// Note: generates a maximum of 2^16 samples per dimension.  If the `index`
/// parameter exceeds 2^16-1, the sample set will start repeating.
#[inline]
pub fn sample(dimension: u32, index: u32, seed: u32) -> f32 {
    sample_4d(dimension >> 2, index, seed)[(dimension & 0b11) as usize]
}

/// Same as `sample()` but calculates a set of 4 dimensions all at once
/// using SIMD.
#[inline]
pub fn sample_4d(dimension_set: u32, index: u32, seed: u32) -> [f32; 4] {
    // This index shuffling approach is due to Brent Burley, and is
    // what allows us to create statistically independent Sobol sequences.
    let shuffled_rev_index = lk_scramble(index.reverse_bits(), seed);

    let sobol = lk_int4_scramble(
        sobol_int4_rev(dimension_set, shuffled_rev_index),
        dimension_set ^ seed,
    )
    .reverse_bits();

    sobol.to_norm_floats()
}

//----------------------------------------------------------------------

/// The core Sobol samplng code.  Used by the other functions.
///
/// This actually produces the Sobol sequence with reversed bits, and takes
/// the index with reversed bits.  This is because the related scrambling
/// code works on reversed bits, so this avoids repeated reversing/unreversing,
/// keeping everything in reversed bits until the final step.
///
/// Note: if the `index` parameter exceeds 2^16-1, the sample set will start
/// repeating.
#[inline(always)]
fn sobol_int4_rev(dimension_set: u32, index: u32) -> Int4 {
    assert!(dimension_set < (MAX_DIMENSION / 4));
    let vecs = &REV_VECTORS[dimension_set as usize];
    let mut index = (index >> 16) as u16;

    let mut result = Int4::zero();
    let mut i = 0;
    while index != 0 {
        let j = index.leading_zeros();
        result ^= vecs[(i + j) as usize].into();
        i += j + 1;
        index <<= j;
        index <<= 1;
    }

    result
}

/// Scrambles `n` using the Laine Karras hash.  This is equivalent to Owen
/// scrambling, but on reversed bits.
#[inline]
fn lk_scramble(mut n: u32, scramble: u32) -> u32 {
    // This uses the technique presented in the paper "Stratified Sampling for
    // Stochastic Transparency" by Laine and Karras to scramble the bits.
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

/// Same as `lk_scramble()`, except does it on 4 integers at a time
/// with SIMD.
#[inline(always)]
fn lk_int4_scramble(mut n: Int4, scramble: u32) -> Int4 {
    n += {
        let a = hash(scramble, 2);
        let b = a ^ 0x174f18ab;
        let c = a ^ 0x691e72ca;
        let d = a ^ 0xb40cc1b8;
        [a, b, c, d].into()
    };

    n ^= [0xdc967795; 4].into();
    n *= [0x97b754b7; 4].into();
    n ^= [0x866350b1; 4].into();
    n *= [0x9e3779cd; 4].into();

    n
}

/// Same as `lk_scramble()` except uses a slower more full version of
/// hashing.
///
/// This is mainly intended to help validate the faster scrambling function,
/// and likely shouldn't be used for real things.  It is significantly
/// slower.
#[allow(dead_code)]
#[inline]
fn lk_scramble_slow(mut n: u32, scramble: u32) -> u32 {
    n = n.wrapping_add(hash(scramble, 3));
    for i in 0..31 {
        let low_mask = (1u32 << i).wrapping_sub(1);
        let low_bits_hash = hash((n & low_mask) ^ hash(i, 3), 3);
        n ^= low_bits_hash & !low_mask;
    }
    n
}

/// A simple 32-bit hash function.  Its quality can be tuned with
/// the number of rounds used.
#[inline(always)]
fn hash(n: u32, rounds: u32) -> u32 {
    let mut hash = n ^ 0x912f69ba;
    for _ in 0..rounds {
        hash = hash.wrapping_mul(0x736caf6f);
        hash ^= hash.wrapping_shr(16);
    }
    hash
}
