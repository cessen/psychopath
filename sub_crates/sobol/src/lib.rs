//! An implementation of the Sobol sequence with Owen scrambling.

// The following `include` provides `MAX_DIMENSION` and `VECTORS`.
// See the build.rs file for how this included file is generated.
include!(concat!(env!("OUT_DIR"), "/vectors.inc"));

/// Compute one component of one sample from the Sobol'-sequence, where
/// `dimension` specifies the component and `index` specifies the sample
/// within the sequence.
///
/// A different `seed` parameter results in a statistically independent Sobol
/// sequence, uncorrelated to others with different seeds.  However, seed
/// itself needs to be sufficiently random: you can't just pass 1, 2, 3, etc.
///
/// Note: generates a maximum of 2^16 samples per dimension.  If the `index`
/// parameter exceeds 2^16-1, the sample set will start repeating.
#[inline]
pub fn sample(dimension: u32, index: u32, seed: u32) -> f32 {
    let scramble = hash(dimension ^ seed);
    let shuffled_index = owen_scramble(index, seed);
    u32_to_0_1_f32(owen_scramble(
        sobol_u32(dimension, shuffled_index),
        scramble,
    ))
}

//----------------------------------------------------------------------

/// The actual core Sobol samplng code.  Used by the other functions.
///
/// Note: if the `index` parameter exceeds 2^16-1, the sample set will start
/// repeating.
#[inline(always)]
fn sobol_u32(dimension: u32, index: u32) -> u32 {
    assert!(dimension < MAX_DIMENSION);
    let vecs = &VECTORS[dimension as usize];
    let mut index = index as u16;

    let mut result = 0;
    let mut i = 0;
    while index != 0 {
        let j = index.trailing_zeros();
        result ^= vecs[(i + j) as usize];
        i += j + 1;
        index >>= j;
        index >>= 1;
    }

    (result as u32) << 16
}

/// Scrambles `n` using Owen scrambling and the given scramble parameter.
#[inline(always)]
fn owen_scramble(mut n: u32, scramble: u32) -> u32 {
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
    // Note that the application of the scramble parameter here via addition
    // does not invalidate the Owen scramble as long as it is done after the
    // bit the reversal.
    //
    // The permutation constants here were selected through an optimization
    // process to maximize low-bias avalanche between bits.

    const PERMS: [u32; 3] = [0x97b756bc, 0x4b0a8a12, 0x75c77e36];
    n = n.reverse_bits();
    n = n.wrapping_add(scramble);
    for &p in PERMS.iter() {
        n ^= n.wrapping_mul(p);
    }
    n = n.reverse_bits();

    // Return the scrambled value.
    n
}

/// Same as `owen_scramble()` except uses a slower more full version of
/// Owen scrambling.
///
/// This is mainly intended to help validate the faster Owen scrambling,
/// and likely shouldn't be used for real things.  It is significantly
/// slower.
#[allow(dead_code)]
#[inline]
fn owen_scramble_slow(mut n: u32, scramble: u32) -> u32 {
    n = n.reverse_bits().wrapping_add(scramble).reverse_bits();
    for i in 0..31 {
        let mask = (1 << (31 - i)) - 1;
        let high_bits_hash = hash((n & (!mask)) ^ hash(i));
        n ^= high_bits_hash & mask;
    }
    n
}

#[inline(always)]
fn hash(n: u32) -> u32 {
    let mut hash = n;
    for _ in 0..3 {
        hash = hash.wrapping_mul(0x736caf6f);
        hash ^= hash.wrapping_shr(16);
    }
    hash
}

#[inline(always)]
fn u32_to_0_1_f32(n: u32) -> f32 {
    const ONE_OVER_32BITS: f32 = 1.0 / (1u64 << 32) as f32;
    n as f32 * ONE_OVER_32BITS
}
