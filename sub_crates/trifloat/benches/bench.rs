use bencher::{benchmark_group, benchmark_main, black_box, Bencher};
use rand::{rngs::SmallRng, FromEntropy, Rng};
use trifloat::{signed48, unsigned32};

//----

fn unsigned32_encode_100_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let x = rng.gen::<f32>() - 0.5;
        let y = rng.gen::<f32>() - 0.5;
        let z = rng.gen::<f32>() - 0.5;
        for _ in 0..100 {
            black_box(unsigned32::encode(black_box((x, y, z))));
        }
    });
}

fn unsigned32_decode_100_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let v = rng.gen::<u32>();
        for _ in 0..100 {
            black_box(unsigned32::decode(black_box(v)));
        }
    });
}

fn signed48_encode_100_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let x = rng.gen::<f32>() - 0.5;
        let y = rng.gen::<f32>() - 0.5;
        let z = rng.gen::<f32>() - 0.5;
        for _ in 0..100 {
            black_box(signed48::encode(black_box((x, y, z))));
        }
    });
}

fn signed48_decode_100_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let v = rng.gen::<u64>() & 0x0000_FFFF_FFFF_FFFF;
        for _ in 0..100 {
            black_box(signed48::decode(black_box(v)));
        }
    });
}

//----

benchmark_group!(
    benches,
    unsigned32_encode_100_values,
    unsigned32_decode_100_values,
    signed48_encode_100_values,
    signed48_decode_100_values,
);
benchmark_main!(benches);
