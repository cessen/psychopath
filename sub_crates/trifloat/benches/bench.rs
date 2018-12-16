use bencher::{benchmark_group, benchmark_main, black_box, Bencher};
use rand::{rngs::SmallRng, FromEntropy, Rng};
use trifloat::{decode, encode};

//----

fn encode_100_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let x = rng.gen::<f32>() - 0.5;
        let y = rng.gen::<f32>() - 0.5;
        let z = rng.gen::<f32>() - 0.5;
        for _ in 0..100 {
            black_box(encode(black_box((x, y, z))));
        }
    });
}

fn decode_100_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let v = rng.gen::<u32>();
        for _ in 0..100 {
            black_box(decode(black_box(v)));
        }
    });
}

//----

benchmark_group!(benches, encode_100_values, decode_100_values,);
benchmark_main!(benches);
