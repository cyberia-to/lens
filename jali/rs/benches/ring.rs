// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Benchmarks for ring arithmetic.

use criterion::{criterion_group, criterion_main, Criterion};
use jali::ring::RingElement;
use jali::ntt;
use jali::sample;

fn bench_ring_add(c: &mut Criterion) {
    let n = 1024;
    let a = sample::sample_uniform(1, n);
    let b = sample::sample_uniform(2, n);
    c.bench_function("ring_add_n1024", |bench| {
        bench.iter(|| a.add(&b));
    });
}

fn bench_ring_mul(c: &mut Criterion) {
    let n = 1024;
    let a = sample::sample_uniform(3, n);
    let b = sample::sample_uniform(4, n);
    c.bench_function("ring_mul_n1024", |bench| {
        bench.iter(|| a.mul(&b));
    });
}

fn bench_ntt_forward(c: &mut Criterion) {
    let n = 1024;
    let a = sample::sample_uniform(5, n);
    c.bench_function("ntt_forward_n1024", |bench| {
        bench.iter(|| {
            let mut elem = a.clone();
            ntt::to_ntt(&mut elem);
            elem
        });
    });
}

fn bench_ntt_inverse(c: &mut Criterion) {
    let n = 1024;
    let mut a = sample::sample_uniform(6, n);
    ntt::to_ntt(&mut a);
    c.bench_function("ntt_inverse_n1024", |bench| {
        bench.iter(|| {
            let mut elem = a.clone();
            ntt::from_ntt(&mut elem);
            elem
        });
    });
}

fn bench_ring_scalar_mul(c: &mut Criterion) {
    let n = 1024;
    let a = sample::sample_uniform(7, n);
    let s = nebu::Goldilocks::new(42);
    c.bench_function("ring_scalar_mul_n1024", |bench| {
        bench.iter(|| a.scalar_mul(s));
    });
}

criterion_group!(benches, bench_ring_add, bench_ring_mul, bench_ntt_forward, bench_ntt_inverse, bench_ring_scalar_mul);
criterion_main!(benches);
