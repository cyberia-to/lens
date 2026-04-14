use criterion::{black_box, criterion_group, criterion_main, Criterion};

use genies::fq::Fq;
use genies::curve::{MontCurve, MontPoint};
use genies::action::{Ideal, NUM_PRIMES};

fn bench_fq_mul(c: &mut Criterion) {
    let a = Fq::from_limbs([
        0x1c80317fa3b1799e, 0xbdd640fb06671ad1,
        0x3eb13b9046685257, 0x23b8c1e9392456de,
        0x1a3d1fa7bc8960a9, 0xbd9c66b3ad3c2d6d,
        0x8b9d2434e465e150, 0x4b95423416419f82,
    ]);
    let b = Fq::from_limbs([
        0x0822e8f36c03119a, 0x17fc695a07a0ca6e,
        0x3b8faa1837f8a88b, 0x9a1de644815ef6d1,
        0x8fadc1a606cb0fb3, 0xb74d0fb132e70629,
        0xb38a088ca65ed389, 0x35b2d3528b8148f6,
    ]);
    c.bench_function("fq_mul", |bench| {
        bench.iter(|| black_box(Fq::mul(&a, &b)))
    });
}

fn bench_fq_inv(c: &mut Criterion) {
    let a = Fq::from_limbs([
        0x1c80317fa3b1799e, 0xbdd640fb06671ad1,
        0x3eb13b9046685257, 0x23b8c1e9392456de,
        0x1a3d1fa7bc8960a9, 0xbd9c66b3ad3c2d6d,
        0x8b9d2434e465e150, 0x4b95423416419f82,
    ]);
    c.bench_function("fq_inv", |bench| {
        bench.iter(|| black_box(Fq::inv(&a)))
    });
}

fn bench_ladder(c: &mut Criterion) {
    let e0 = MontCurve::e0();
    let p = MontPoint::from_x(Fq::from_u64(4));
    let scalar: [u64; 8] = [0xdeadbeef, 0xcafebabe, 0x12345678, 0x9abcdef0, 0, 0, 0, 0];
    c.bench_function("ladder", |bench| {
        bench.iter(|| black_box(p.ladder(&scalar, &e0.a)))
    });
}

fn bench_action_single(c: &mut Criterion) {
    let e0 = MontCurve::e0();
    let mut exponents = [0i8; NUM_PRIMES];
    exponents[0] = 1;
    let ideal = Ideal::from_exponents(&exponents);
    c.bench_function("action_single", |bench| {
        bench.iter(|| black_box(genies::action::action(&ideal, &e0)))
    });
}

criterion_group!(benches, bench_fq_mul, bench_fq_inv, bench_ladder, bench_action_single);
criterion_main!(benches);
