use bencher::{benchmark_group, benchmark_main, black_box, Bencher};
use rand::{rngs::SmallRng, FromEntropy, Rng};
use trifloat::{fluv32, signed48, unsigned32, unsigned40};

//----

fn unsigned32_encode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let x = rng.gen::<f32>();
        let y = rng.gen::<f32>();
        let z = rng.gen::<f32>();
        for _ in 0..1000 {
            black_box(unsigned32::encode(black_box((x, y, z))));
        }
    });
}

fn unsigned32_decode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let v = rng.gen::<u32>();
        for _ in 0..1000 {
            black_box(unsigned32::decode(black_box(v)));
        }
    });
}

fn unsigned40_encode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let x = rng.gen::<f32>();
        let y = rng.gen::<f32>();
        let z = rng.gen::<f32>();
        for _ in 0..1000 {
            black_box(unsigned40::encode(black_box((x, y, z))));
        }
    });
}

fn unsigned40_decode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let v = [
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
        ];
        for _ in 0..1000 {
            black_box(unsigned40::decode(black_box(v)));
        }
    });
}

fn signed48_encode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let x = rng.gen::<f32>() - 0.5;
        let y = rng.gen::<f32>() - 0.5;
        let z = rng.gen::<f32>() - 0.5;
        for _ in 0..1000 {
            black_box(signed48::encode(black_box((x, y, z))));
        }
    });
}

fn signed48_decode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let v = [
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
        ];
        for _ in 0..1000 {
            black_box(signed48::decode(black_box(v)));
        }
    });
}

fn fluv32_encode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let x = rng.gen::<f32>();
        let y = rng.gen::<f32>();
        let z = rng.gen::<f32>();
        for _ in 0..1000 {
            black_box(fluv32::encode(black_box((x, y, z))));
        }
    });
}

fn fluv32_decode_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let v = rng.gen::<u32>();
        for _ in 0..1000 {
            black_box(fluv32::decode(black_box(v)));
        }
    });
}

fn fluv32_decode_yuv_1000_values(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let v = rng.gen::<u32>();
        for _ in 0..1000 {
            black_box(fluv32::decode_yuv(black_box(v)));
        }
    });
}

//----

benchmark_group!(
    benches,
    unsigned32_encode_1000_values,
    unsigned32_decode_1000_values,
    unsigned40_encode_1000_values,
    unsigned40_decode_1000_values,
    signed48_encode_1000_values,
    signed48_decode_1000_values,
    fluv32_encode_1000_values,
    fluv32_decode_1000_values,
    fluv32_decode_yuv_1000_values,
);
benchmark_main!(benches);
