// ── nebu/test_vectors ────────────────────────────────────────────
//
// Compute shader that validates field operations against known
// test vectors from reference/vectors.md.
// Writes 1 (pass) or 0 (fail) per test to the output buffer.
//
// Include order: field.wgsl, extension.wgsl (concatenate before compile)

@group(0) @binding(0)
var<storage, read_write> results: array<u32>;

fn check(idx: u32, got_lo: u32, got_hi: u32, exp_lo: u32, exp_hi: u32) {
    let got = gl_canon(got_lo, got_hi);
    let exp = gl_canon(exp_lo, exp_hi);
    results[idx] = select(0u, 1u, got.x == exp.x && got.y == exp.y);
}

@compute @workgroup_size(1)
fn main() {
    var t = 0u;

    // ── Canonical reduction ──────────────────────────────────────
    // g(0) = 0
    check(t, 0u, 0u, 0u, 0u); t++;
    // g(1) = 1
    check(t, 1u, 0u, 1u, 0u); t++;
    // g(P) = 0
    check(t, P_LO, P_HI, 0u, 0u); t++;
    // g(P+1) = 1
    let pp1 = add64(P_LO, P_HI, 1u, 0u);
    check(t, gl_canon(pp1.x, pp1.y).x, gl_canon(pp1.x, pp1.y).y, 1u, 0u); t++;

    // ── Addition ─────────────────────────────────────────────────
    // 1 + 2 = 3
    let a0 = gl_add(1u, 0u, 2u, 0u);
    check(t, a0.x, a0.y, 3u, 0u); t++;
    // (p-1) + 1 = 0   [p-1 = (0x00000000, 0xFFFFFFFF)]
    let a1 = gl_add(0x00000000u, 0xFFFFFFFFu, 1u, 0u);
    check(t, a1.x, a1.y, 0u, 0u); t++;

    // ── Subtraction ──────────────────────────────────────────────
    // 5 - 3 = 2
    let s0 = gl_sub(5u, 0u, 3u, 0u);
    check(t, s0.x, s0.y, 2u, 0u); t++;
    // 0 - 1 = p-1 = 0xFFFFFFFF00000000
    let s1 = gl_sub(0u, 0u, 1u, 0u);
    check(t, s1.x, s1.y, 0x00000000u, 0xFFFFFFFFu); t++;

    // ── Multiplication ───────────────────────────────────────────
    // 3 * 7 = 21
    let m0 = gl_mul(3u, 0u, 7u, 0u);
    check(t, m0.x, m0.y, 0x15u, 0u); t++;
    // (p-1)^2 = 1
    let m1 = gl_mul(0x00000000u, 0xFFFFFFFFu, 0x00000000u, 0xFFFFFFFFu);
    check(t, m1.x, m1.y, 1u, 0u); t++;

    // ── Negation ─────────────────────────────────────────────────
    // -0 = 0
    let n0 = gl_neg(0u, 0u);
    check(t, n0.x, n0.y, 0u, 0u); t++;
    // -1 = p-1
    let n1 = gl_neg(1u, 0u);
    check(t, n1.x, n1.y, 0x00000000u, 0xFFFFFFFFu); t++;

    // ── pow7 ─────────────────────────────────────────────────────
    // pow7(0) = 0
    let p0 = gl_pow7(0u, 0u);
    check(t, p0.x, p0.y, 0u, 0u); t++;
    // pow7(1) = 1
    let p1 = gl_pow7(1u, 0u);
    check(t, p1.x, p1.y, 1u, 0u); t++;
    // pow7(2) = 128
    let p2 = gl_pow7(2u, 0u);
    check(t, p2.x, p2.y, 0x80u, 0u); t++;

    // ── Inversion ────────────────────────────────────────────────
    // inv(1) = 1
    let i0 = gl_inv(1u, 0u);
    check(t, i0.x, i0.y, 1u, 0u); t++;
    // inv(2) = 0x7FFFFFFF80000001 = (lo=0x80000001, hi=0x7FFFFFFF)
    let i1 = gl_inv(2u, 0u);
    check(t, i1.x, i1.y, 0x80000001u, 0x7FFFFFFFu); t++;
    // 2 * inv(2) = 1
    let i2 = gl_mul(2u, 0u, i1.x, i1.y);
    check(t, i2.x, i2.y, 1u, 0u); t++;

    // ── Legendre ─────────────────────────────────────────────────
    // legendre(0) = 0
    let l0 = gl_legendre(0u, 0u);
    check(t, l0.x, l0.y, 0u, 0u); t++;
    // legendre(4) = 1  (QR)
    let l1 = gl_legendre(4u, 0u);
    check(t, l1.x, l1.y, 1u, 0u); t++;
    // legendre(7) = p-1  (QNR)
    let l2 = gl_legendre(7u, 0u);
    check(t, l2.x, l2.y, 0x00000000u, 0xFFFFFFFFu); t++;

    // ── Fp2 multiplication ───────────────────────────────────────
    // (2 + 3u)(4 + 5u) = (113, 22)
    let fx = fp2_new(2u, 0u, 3u, 0u);
    let fy = fp2_new(4u, 0u, 5u, 0u);
    let fz = fp2_mul(fx, fy);
    check(t, fz.x, fz.y, 0x71u, 0u); t++;   // re = 113
    check(t, fz.z, fz.w, 0x16u, 0u); t++;   // im = 22

    // ── Fp2 inversion roundtrip ──────────────────────────────────
    let fxi = fp2_inv(fx);
    let fone = fp2_mul(fx, fxi);
    check(t, fone.x, fone.y, 1u, 0u); t++;   // re = 1
    check(t, fone.z, fone.w, 0u, 0u); t++;   // im = 0

    // ── Fp2 conjugate ────────────────────────────────────────────
    let fc = fp2_conj(fp2_new(1u, 0u, 1u, 0u));
    check(t, fc.x, fc.y, 1u, 0u); t++;       // re = 1
    // im = -1 = p-1
    check(t, fc.z, fc.w, 0x00000000u, 0xFFFFFFFFu); t++;

    // Store total test count in last slot
    results[64u] = t;
}
