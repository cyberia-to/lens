// ---
// tags: trop, bench
// crystal-type: source
// crystal-domain: comp
// ---

//! Benchmarks for tropical semiring operations.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use trop::determinant::determinant;
use trop::element::Tropical;
use trop::kleene::kleene_star;
use trop::matrix::TropMatrix;

fn bench_matmul(c: &mut Criterion) {
    for &n in &[4, 8, 16, 32] {
        c.bench_function(&format!("matmul_{n}x{n}"), |b| {
            let mut a = TropMatrix::new(n);
            let mut bm = TropMatrix::new(n);
            for i in 0..n {
                for j in 0..n {
                    a.set(i, j, Tropical::from_u64((i * n + j) as u64));
                    bm.set(i, j, Tropical::from_u64(((n - i) * n + j) as u64));
                }
            }
            b.iter(|| black_box(a.mul(&bm)));
        });
    }
}

fn bench_kleene_star(c: &mut Criterion) {
    for &n in &[4, 8, 16, 32] {
        c.bench_function(&format!("kleene_star_{n}"), |b| {
            let mut m = TropMatrix::new(n);
            for i in 0..n {
                if i + 1 < n {
                    m.set(i, i + 1, Tropical::from_u64(1));
                }
                if i > 0 {
                    m.set(i, i - 1, Tropical::from_u64(2));
                }
            }
            b.iter(|| black_box(kleene_star(&m)));
        });
    }
}

fn bench_determinant(c: &mut Criterion) {
    for &n in &[4, 6, 8] {
        c.bench_function(&format!("determinant_{n}"), |b| {
            let mut m = TropMatrix::new(n);
            for i in 0..n {
                for j in 0..n {
                    m.set(i, j, Tropical::from_u64(((i + j) % n) as u64));
                }
            }
            b.iter(|| black_box(determinant(&m)));
        });
    }
}

fn bench_power(c: &mut Criterion) {
    for &n in &[4, 8, 16] {
        c.bench_function(&format!("power_{n}_exp64"), |b| {
            let mut m = TropMatrix::new(n);
            for i in 0..n {
                if i + 1 < n {
                    m.set(i, i + 1, Tropical::from_u64(1));
                }
            }
            m.set(n - 1, 0, Tropical::from_u64(1));
            b.iter(|| black_box(m.power(64)));
        });
    }
}

criterion_group!(
    benches,
    bench_matmul,
    bench_kleene_star,
    bench_determinant,
    bench_power
);
criterion_main!(benches);
