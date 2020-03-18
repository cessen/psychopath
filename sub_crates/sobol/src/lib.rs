//! An implementation of the Sobol low discrepancy sequence.
//!
//! Includes variants with random digit scrambling, Cranley-Patterson rotation,
//! and Owen scrambling.

// The following `include` provides `MAX_DIMENSION` and `VECTORS`.
// See the build.rs file for how this included file is generated.
include!(concat!(env!("OUT_DIR"), "/vectors.inc"));

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
pub fn sample_rd(dimension: u32, index: u32, scramble: u32) -> f32 {
    u32_to_0_1_f32(sobol_u32(dimension, index) ^ scramble)
}

/// Same as `sample()` except applies Cranley Patterson rotation using the
/// given scramble parameter.
///
/// To get proper Cranley Patterson rotation, you need to use a different
/// scramble value for each dimension, and those values should be generated
/// more-or-less randomly.  For example, using a 32-bit hash of the dimension
/// parameter works well.
#[inline]
pub fn sample_cranley(dimension: u32, index: u32, scramble: u32) -> f32 {
    u32_to_0_1_f32(sobol_u32(dimension, index).wrapping_add(scramble))
}

/// Same as `sample()` except applies Owen scrambling using the given scramble
/// parameter.
///
/// To get proper Owen scrambling, you need to use a different scramble
/// value for each dimension, and those values should be generated more-or-less
/// randomly.  For example, using a 32-bit hash of the dimension parameter
/// works well.
#[inline]
pub fn sample_owen(dimension: u32, index: u32, scramble: u32) -> f32 {
    u32_to_0_1_f32(owen_scramble_u32(sobol_u32(dimension, index), scramble))
}

/// Same as `sample()` except applies both Owen scrambling and
/// Cranley-Patterson rotation using the given scramble parameter.
///
/// For the technically curious: this first does Owen scrambling, and then
/// Cranley-Patterson.  If it were done the other way around it would
/// invalidate the Owen scrambling.
///
/// To get proper scrambling and rotation, you need to use a different scramble
/// value for each dimension, and those values should be generated more-or-less
/// randomly.  For example, using a 32-bit hash of the dimension parameter
/// works well.
#[inline]
pub fn sample_owen_cranley(dimension: u32, index: u32, scramble: u32) -> f32 {
    // Reusing the same scramble parameter for both the Owen scrambling and
    // the Cranely-Patterson rotation actually works fine, because the Owen
    // scrambling is implemented as a sort of hash, so they don't end up being
    // correlated.
    u32_to_0_1_f32(owen_scramble_u32(sobol_u32(dimension, index), scramble).wrapping_add(scramble))
}

//----------------------------------------------------------------------

/// The actual core Sobol samplng code.  Used by the other functions.
#[inline(always)]
fn sobol_u32(dimension: u32, mut index: u32) -> u32 {
    assert!(dimension < MAX_DIMENSION);
    let vecs = &VECTORS[dimension as usize];

    let mut result = 0;
    let mut i = 0;
    while index != 0 {
        let j = index.trailing_zeros();
        result ^= vecs[(i + j) as usize];
        i += j + 1;
        index >>= j + 1;
    }

    result
}

/// Scrambles `n` using Owen scrambling and the given scramble parameter.
#[inline(always)]
fn owen_scramble_u32(mut n: u32, scramble: u32) -> u32 {
    // We don't need the lowest 8 bits because we're converting to an f32 at
    // the end which only has 24 bits of precision anyway.  And doing this
    // allows the seed to affect the mixing of the higher bits to make them
    // more random in the Owen scrambling below.
    n >>= 8;

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
    // The permutation constants here were selected through an optimization
    // process to maximize low-bias avalanche between bits.

    n = n.reverse_bits();
    n ^= scramble;
    let perms = [0xa56bb1c6, 0xef577134, 0xd0e5e808, 0x200bd50a];
    for p in perms.iter() {
        n ^= n.wrapping_mul(*p);
    }
    n = n.reverse_bits();

    // Return the scrambled value, shifted back into place.
    n << 8
}

#[inline(always)]
fn u32_to_0_1_f32(n: u32) -> f32 {
    n as f32 * (1.0 / (1u64 << 32) as f32)
}
