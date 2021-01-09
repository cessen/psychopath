use bencher::{benchmark_group, benchmark_main, black_box, Bencher};
use rand::{rngs::SmallRng, FromEntropy, Rng};
use sobol::sample_4d;

//----

fn gen_1000_samples(bench: &mut Bencher) {
    bench.iter(|| {
        for i in 0..1000u32 {
            black_box(sample_4d(i, 0, 1234567890));
        }
    });
}

fn gen_1000_samples_incoherent(bench: &mut Bencher) {
    let mut rng = SmallRng::from_entropy();
    bench.iter(|| {
        let s = rng.gen::<u32>();
        let d = rng.gen::<u32>();
        let seed = rng.gen::<u32>();
        for i in 0..1000u32 {
            black_box(sample_4d(
                s.wrapping_add(i).wrapping_mul(512),
                d.wrapping_add(i).wrapping_mul(97) % 32,
                seed,
            ));
        }
    });
}

//----

benchmark_group!(benches, gen_1000_samples, gen_1000_samples_incoherent,);
benchmark_main!(benches);
