// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! GPU cross-validation — validates WGSL implementation against CPU (Rust).
//!
//! Each test runs the same operation on both CPU and GPU, then
//! compares results. This is the bridge between the two independent
//! implementations: rs/ (CPU) and wgsl/ (GPU).

use nebu::field::Goldilocks;
use nebu::Fp2;
use nebu_wgsl::GpuContext;

// ── Helpers ────────────────────────────────────────────────────────

fn gpu() -> GpuContext {
    GpuContext::new().expect("no GPU adapter available")
}

fn to_lohi(g: Goldilocks) -> (u32, u32) {
    let v = g.as_u64();
    (v as u32, (v >> 32) as u32)
}

// ── Test vector shader ─────────────────────────────────────────────

#[test]
fn gpu_test_vectors_shader() {
    let ctx = gpu();
    let results = ctx.run_test_vectors();
    let test_count = results[64] as usize;
    assert!(test_count > 0, "no tests ran");

    let mut failures = Vec::new();
    for i in 0..test_count {
        if results[i] != 1 {
            failures.push(i);
        }
    }
    assert!(
        failures.is_empty(),
        "GPU test vector failures at indices: {:?} ({}/{} passed)",
        failures,
        test_count - failures.len(),
        test_count
    );
}

// ── Cross-validation: GPU vs CPU ───────────────────────────────────

#[test]
fn gpu_add_matches_cpu() {
    let ctx = gpu();
    let cases = [
        (0u64, 0u64),
        (1, 2),
        (0xFFFFFFFF00000000, 1),
        (0x8000000000000000, 0x8000000000000000),
    ];
    for (a, b) in cases {
        let cpu = Goldilocks::new(a) + Goldilocks::new(b);
        let (lo, hi) = ctx.eval_field_op(
            &format!("gl_add({}u, {}u, {}u, {}u)", a as u32, (a >> 32) as u32, b as u32, (b >> 32) as u32)
        );
        assert_eq!(to_lohi(cpu), (lo, hi), "add({a:#x}, {b:#x})");
    }
}

#[test]
fn gpu_sub_matches_cpu() {
    let ctx = gpu();
    let cases = [(5u64, 3u64), (0, 1), (1, 0xFFFFFFFF00000000)];
    for (a, b) in cases {
        let cpu = Goldilocks::new(a) - Goldilocks::new(b);
        let (lo, hi) = ctx.eval_field_op(
            &format!("gl_sub({}u, {}u, {}u, {}u)", a as u32, (a >> 32) as u32, b as u32, (b >> 32) as u32)
        );
        assert_eq!(to_lohi(cpu), (lo, hi), "sub({a:#x}, {b:#x})");
    }
}

#[test]
fn gpu_mul_matches_cpu() {
    let ctx = gpu();
    let cases = [
        (3u64, 7u64),
        (0xFFFFFFFF00000000, 0xFFFFFFFF00000000),
        (0x12345678, 0x9ABCDEF0),
        (0xFFFFFFFF00000000, 2),
    ];
    for (a, b) in cases {
        let cpu = Goldilocks::new(a) * Goldilocks::new(b);
        let (lo, hi) = ctx.eval_field_op(
            &format!("gl_mul({}u, {}u, {}u, {}u)", a as u32, (a >> 32) as u32, b as u32, (b >> 32) as u32)
        );
        assert_eq!(to_lohi(cpu), (lo, hi), "mul({a:#x}, {b:#x})");
    }
}

#[test]
fn gpu_neg_matches_cpu() {
    let ctx = gpu();
    let cases = [0u64, 1, 0xFFFFFFFF00000000, 0x2A];
    for a in cases {
        let cpu = -Goldilocks::new(a);
        let (lo, hi) = ctx.eval_field_op(
            &format!("gl_neg({}u, {}u)", a as u32, (a >> 32) as u32)
        );
        assert_eq!(to_lohi(cpu), (lo, hi), "neg({a:#x})");
    }
}

#[test]
fn gpu_pow7_matches_cpu() {
    let ctx = gpu();
    let cases = [0u64, 1, 2, 7, 0xDEADBEEF];
    for a in cases {
        let cpu = Goldilocks::new(a).pow7();
        let (lo, hi) = ctx.eval_field_op(
            &format!("gl_pow7({}u, {}u)", a as u32, (a >> 32) as u32)
        );
        assert_eq!(to_lohi(cpu), (lo, hi), "pow7({a:#x})");
    }
}

#[test]
fn gpu_inv_matches_cpu() {
    let ctx = gpu();
    let cases = [1u64, 2, 0xFFFFFFFF00000000];
    for a in cases {
        let cpu = Goldilocks::new(a).inv();
        let (lo, hi) = ctx.eval_field_op(
            &format!("gl_inv({}u, {}u)", a as u32, (a >> 32) as u32)
        );
        assert_eq!(to_lohi(cpu), (lo, hi), "inv({a:#x})");
    }
}

#[test]
fn gpu_inv_roundtrip() {
    let ctx = gpu();
    let (lo, hi) = ctx.eval_field_op(
        "gl_mul(2u, 0u, gl_inv(2u, 0u).x, gl_inv(2u, 0u).y)"
    );
    assert_eq!((lo, hi), (1, 0), "2 * inv(2) should be 1");
}

#[test]
#[ignore = "Tonelli-Shanks calls gl_exp 4x with 64-bit exponents — exceeds GPU timeout"]
fn gpu_sqrt_four() {
    let ctx = gpu();
    let r = ctx.run_custom(
        "let r = gl_sqrt(4u, 0u);\n\
         out[0] = r.x; out[1] = r.y; out[2] = r.z;",
        3,
    );
    assert_eq!(r[2], 1, "sqrt(4) found");
    assert_eq!((r[0], r[1]), to_lohi(Goldilocks::new(2)), "sqrt(4) = 2");
}

#[test]
fn gpu_sqrt_qnr() {
    let ctx = gpu();
    let r = ctx.run_custom(
        "let r = gl_sqrt(7u, 0u);\n\
         out[0] = r.z;",
        1,
    );
    assert_eq!(r[0], 0, "sqrt(7) should be QNR");
}

#[test]
fn gpu_fp2_mul_matches_cpu() {
    let ctx = gpu();

    let x = Fp2::new(Goldilocks::new(2), Goldilocks::new(3));
    let y = Fp2::new(Goldilocks::new(4), Goldilocks::new(5));
    let cpu = x * y;

    let r = ctx.run_custom(
        "let z = fp2_mul(fp2_new(2u, 0u, 3u, 0u), fp2_new(4u, 0u, 5u, 0u));\n\
         let re = gl_canon(z.x, z.y);\n\
         let im = gl_canon(z.z, z.w);\n\
         out[0] = re.x; out[1] = re.y; out[2] = im.x; out[3] = im.y;",
        4,
    );

    assert_eq!((r[0], r[1]), to_lohi(cpu.re), "fp2_mul re");
    assert_eq!((r[2], r[3]), to_lohi(cpu.im), "fp2_mul im");
}

#[test]
fn gpu_ntt_roundtrip() {
    let ctx = gpu();

    let input = [
        Goldilocks::new(1),
        Goldilocks::new(2),
        Goldilocks::new(3),
        Goldilocks::new(4),
    ];
    let mut cpu_data = input;
    nebu::ntt::ntt(&mut cpu_data);

    let mut gpu_data: Vec<(u32, u32)> = input.iter().map(|g| to_lohi(*g)).collect();
    ctx.run_ntt(&mut gpu_data);

    for i in 0..4 {
        assert_eq!(
            gpu_data[i], to_lohi(cpu_data[i]),
            "NTT mismatch at index {i}"
        );
    }

    ctx.run_intt(&mut gpu_data);
    for i in 0..4 {
        assert_eq!(
            gpu_data[i], to_lohi(input[i]),
            "INTT roundtrip mismatch at index {i}"
        );
    }
}
