pub fn hash_u32(n: u32, seed: u32) -> u32 {
    let mut hash = n;
    for _ in 0..3 {
        hash = hash.wrapping_mul(0x736caf6f);
        hash ^= hash.wrapping_shr(16);
        hash ^= seed;
    }

    hash
}

pub fn hash_u64(n: u64, seed: u64) -> u64 {
    let mut hash = n;
    for _ in 0..4 {
        hash = hash.wrapping_mul(32_416_190_071 * 314_604_959);
        hash ^= hash.wrapping_shr(32);
        hash ^= seed;
    }

    hash
}

/// Returns a random float in [0, 1] based on 'n' and a seed.
/// Generally use n for getting a bunch of different random
/// numbers, and use seed to vary between runs.
pub fn hash_u32_to_f32(n: u32, seed: u32) -> f32 {
    const INV_MAX: f32 = 1.0 / std::u32::MAX as f32;
    hash_u32(n, seed) as f32 * INV_MAX
}
