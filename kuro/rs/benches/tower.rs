use criterion::{Criterion, black_box, criterion_group, criterion_main};
use kuro::{F2_8, F2_16, F2_32, F2_64, F2_128, Packed128};

fn bench_add(c: &mut Criterion) {
    let a = F2_128(0xDEADBEEFCAFEBABE_1234567890ABCDEF);
    let b = F2_128(0x0123456789ABCDEF_FEDCBA9876543210);
    c.bench_function("F2_128::add", |bench| {
        bench.iter(|| black_box(a).add(black_box(b)))
    });
}

fn bench_mul_128(c: &mut Criterion) {
    let a = F2_128(0xDEADBEEFCAFEBABE_1234567890ABCDEF);
    let b = F2_128(0x0123456789ABCDEF_FEDCBA9876543210);
    c.bench_function("F2_128::mul", |bench| {
        bench.iter(|| black_box(a).mul(black_box(b)))
    });
}

fn bench_mul_64(c: &mut Criterion) {
    let a = F2_64(0xDEADBEEFCAFEBABE);
    let b = F2_64(0x0123456789ABCDEF);
    c.bench_function("F2_64::mul", |bench| {
        bench.iter(|| black_box(a).mul(black_box(b)))
    });
}

fn bench_mul_32(c: &mut Criterion) {
    let a = F2_32(0xDEADBEEF);
    let b = F2_32(0xCAFEBABE);
    c.bench_function("F2_32::mul", |bench| {
        bench.iter(|| black_box(a).mul(black_box(b)))
    });
}

fn bench_inv_128(c: &mut Criterion) {
    let a = F2_128(0xDEADBEEFCAFEBABE_1234567890ABCDEF);
    c.bench_function("F2_128::inv", |bench| bench.iter(|| black_box(a).inv()));
}

fn bench_inv_64(c: &mut Criterion) {
    let a = F2_64(0xDEADBEEFCAFEBABE);
    c.bench_function("F2_64::inv", |bench| bench.iter(|| black_box(a).inv()));
}

fn bench_square_128(c: &mut Criterion) {
    let a = F2_128(0xDEADBEEFCAFEBABE_1234567890ABCDEF);
    c.bench_function("F2_128::square", |bench| {
        bench.iter(|| black_box(a).square())
    });
}

fn bench_packed_inner_product(c: &mut Criterion) {
    let a = Packed128(0xDEADBEEFCAFEBABE_1234567890ABCDEF);
    let b = Packed128(0x0123456789ABCDEF_FEDCBA9876543210);
    c.bench_function("Packed128::inner_product", |bench| {
        bench.iter(|| black_box(a).inner_product(black_box(b)))
    });
}

fn bench_packed_popcount(c: &mut Criterion) {
    let a = Packed128(0xDEADBEEFCAFEBABE_1234567890ABCDEF);
    c.bench_function("Packed128::popcount", |bench| {
        bench.iter(|| black_box(a).popcount())
    });
}

criterion_group!(
    benches,
    bench_add,
    bench_mul_128,
    bench_mul_64,
    bench_mul_32,
    bench_inv_128,
    bench_inv_64,
    bench_square_128,
    bench_packed_inner_product,
    bench_packed_popcount,
);
criterion_main!(benches);
