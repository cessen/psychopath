use bencher::{benchmark_group, benchmark_main, black_box, Bencher};
use oct32norm::{decode, encode, encode_precise};
use rand::{rngs::SmallRng, FromEntropy, Rng};

//----

fn encode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let x = rng.gen::<f32>() - 0.5;
        let y = rng.gen::<f32>() - 0.5;
        let z = rng.gen::<f32>() - 0.5;
        for _ in 0..1000 {
            black_box(encode(black_box((x, y, z))));
        }
    });
}

fn encode_precise_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let x = rng.gen::<f32>() - 0.5;
        let y = rng.gen::<f32>() - 0.5;
        let z = rng.gen::<f32>() - 0.5;
        for _ in 0..1000 {
            black_box(encode_precise(black_box((x, y, z))));
        }
    });
}

fn decode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let v = rng.gen::<u32>();
        for _ in 0..1000 {
            black_box(decode(black_box(v)));
        }
    });
}

//----

benchmark_group!(
    benches,
    encode_1000_values,
    encode_precise_1000_values,
    decode_1000_values,
);
benchmark_main!(benches);
