pub fn hash_u32(n: u32, seed: u32) -> u32 {
    let mut hash = n;

    for _ in 0..3 {
        hash = hash.wrapping_mul(1936502639);
        hash ^= hash.wrapping_shr(16);
        hash = hash.wrapping_add(seed);
    }

    return hash;
}

pub fn hash_u64(n: u64, seed: u64) -> u64 {
    let mut hash = n;

    for _ in 0..4 {
        hash = hash.wrapping_mul(32416190071 * 314604959);
        hash ^= hash.wrapping_shr(32);
        hash = hash.wrapping_add(seed);
    }

    return hash;
}
