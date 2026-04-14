// ── nebu/fp4 ────────────────────────────────────────────────────
//
// Quartic extension F_{p⁴} = F_p[w] / (w⁴ - 7).
// Elements are (c0, c1, c2, c3) as array<vec2<u32>, 4>.
// Each vec2<u32> is (lo, hi) limbs of a Goldilocks element.
// Reduction: w⁴ = 7.
// Tower: Fp4 = Fp2[v] / (v² - u) where u² = 7, v = w.
// Requires: field.wgsl, extension.wgsl

// ── Fp4 helpers ─────────────────────────────────────────────────

struct Fp4 {
    c0: vec2<u32>,
    c1: vec2<u32>,
    c2: vec2<u32>,
    c3: vec2<u32>,
}

fn fp4_zero() -> Fp4 {
    return Fp4(
        vec2<u32>(0u, 0u), vec2<u32>(0u, 0u),
        vec2<u32>(0u, 0u), vec2<u32>(0u, 0u),
    );
}

fn fp4_one() -> Fp4 {
    return Fp4(
        vec2<u32>(1u, 0u), vec2<u32>(0u, 0u),
        vec2<u32>(0u, 0u), vec2<u32>(0u, 0u),
    );
}

fn fp4_from_base(a_lo: u32, a_hi: u32) -> Fp4 {
    return Fp4(
        vec2<u32>(a_lo, a_hi), vec2<u32>(0u, 0u),
        vec2<u32>(0u, 0u), vec2<u32>(0u, 0u),
    );
}

// Embed Fp2: (re, im) → (re, 0, im, 0)
fn fp4_from_fp2(x: vec4<u32>) -> Fp4 {
    return Fp4(
        vec2<u32>(x.x, x.y), vec2<u32>(0u, 0u),
        vec2<u32>(x.z, x.w), vec2<u32>(0u, 0u),
    );
}

// ── Fp4 arithmetic ──────────────────────────────────────────────

fn fp4_add(x: Fp4, y: Fp4) -> Fp4 {
    return Fp4(
        gl_add(x.c0.x, x.c0.y, y.c0.x, y.c0.y),
        gl_add(x.c1.x, x.c1.y, y.c1.x, y.c1.y),
        gl_add(x.c2.x, x.c2.y, y.c2.x, y.c2.y),
        gl_add(x.c3.x, x.c3.y, y.c3.x, y.c3.y),
    );
}

fn fp4_sub(x: Fp4, y: Fp4) -> Fp4 {
    return Fp4(
        gl_sub(x.c0.x, x.c0.y, y.c0.x, y.c0.y),
        gl_sub(x.c1.x, x.c1.y, y.c1.x, y.c1.y),
        gl_sub(x.c2.x, x.c2.y, y.c2.x, y.c2.y),
        gl_sub(x.c3.x, x.c3.y, y.c3.x, y.c3.y),
    );
}

fn fp4_neg(x: Fp4) -> Fp4 {
    return Fp4(
        gl_neg(x.c0.x, x.c0.y),
        gl_neg(x.c1.x, x.c1.y),
        gl_neg(x.c2.x, x.c2.y),
        gl_neg(x.c3.x, x.c3.y),
    );
}

// Tower conjugate: (c0, -c1, c2, -c3)
fn fp4_conj(x: Fp4) -> Fp4 {
    return Fp4(
        x.c0,
        gl_neg(x.c1.x, x.c1.y),
        x.c2,
        gl_neg(x.c3.x, x.c3.y),
    );
}

// Schoolbook multiplication: 16 base muls + w⁴ = 7 reduction.
fn fp4_mul(x: Fp4, y: Fp4) -> Fp4 {
    // All 16 products
    let a0b0 = gl_mul(x.c0.x, x.c0.y, y.c0.x, y.c0.y);
    let a0b1 = gl_mul(x.c0.x, x.c0.y, y.c1.x, y.c1.y);
    let a0b2 = gl_mul(x.c0.x, x.c0.y, y.c2.x, y.c2.y);
    let a0b3 = gl_mul(x.c0.x, x.c0.y, y.c3.x, y.c3.y);
    let a1b0 = gl_mul(x.c1.x, x.c1.y, y.c0.x, y.c0.y);
    let a1b1 = gl_mul(x.c1.x, x.c1.y, y.c1.x, y.c1.y);
    let a1b2 = gl_mul(x.c1.x, x.c1.y, y.c2.x, y.c2.y);
    let a1b3 = gl_mul(x.c1.x, x.c1.y, y.c3.x, y.c3.y);
    let a2b0 = gl_mul(x.c2.x, x.c2.y, y.c0.x, y.c0.y);
    let a2b1 = gl_mul(x.c2.x, x.c2.y, y.c1.x, y.c1.y);
    let a2b2 = gl_mul(x.c2.x, x.c2.y, y.c2.x, y.c2.y);
    let a2b3 = gl_mul(x.c2.x, x.c2.y, y.c3.x, y.c3.y);
    let a3b0 = gl_mul(x.c3.x, x.c3.y, y.c0.x, y.c0.y);
    let a3b1 = gl_mul(x.c3.x, x.c3.y, y.c1.x, y.c1.y);
    let a3b2 = gl_mul(x.c3.x, x.c3.y, y.c2.x, y.c2.y);
    let a3b3 = gl_mul(x.c3.x, x.c3.y, y.c3.x, y.c3.y);

    // d0 through d6
    let d0 = a0b0;
    var d1 = gl_add(a0b1.x, a0b1.y, a1b0.x, a1b0.y);
    var d2 = gl_add(a0b2.x, a0b2.y, a1b1.x, a1b1.y);
    d2 = gl_add(d2.x, d2.y, a2b0.x, a2b0.y);
    var d3 = gl_add(a0b3.x, a0b3.y, a1b2.x, a1b2.y);
    d3 = gl_add(d3.x, d3.y, a2b1.x, a2b1.y);
    d3 = gl_add(d3.x, d3.y, a3b0.x, a3b0.y);
    var d4 = gl_add(a1b3.x, a1b3.y, a2b2.x, a2b2.y);
    d4 = gl_add(d4.x, d4.y, a3b1.x, a3b1.y);
    let d5 = gl_add(a2b3.x, a2b3.y, a3b2.x, a3b2.y);
    let d6 = a3b3;

    // Reduce: w⁴ = 7
    let seven_d4 = gl_mul_small(d4.x, d4.y, 7u);
    let seven_d5 = gl_mul_small(d5.x, d5.y, 7u);
    let seven_d6 = gl_mul_small(d6.x, d6.y, 7u);

    return Fp4(
        gl_add(d0.x, d0.y, seven_d4.x, seven_d4.y),
        gl_add(d1.x, d1.y, seven_d5.x, seven_d5.y),
        gl_add(d2.x, d2.y, seven_d6.x, seven_d6.y),
        d3,
    );
}

// Tower-based inversion via Fp2 norm.
fn fp4_inv(x: Fp4) -> Fp4 {
    // Tower components: A = (c0, c2), B = (c1, c3) in Fp2
    let a = vec4<u32>(x.c0.x, x.c0.y, x.c2.x, x.c2.y);
    let b = vec4<u32>(x.c1.x, x.c1.y, x.c3.x, x.c3.y);

    // N = A² - u·B² in Fp2
    let a_sq = fp2_sqr(a);
    let b_sq = fp2_sqr(b);
    // u·B²: multiply Fp2 by u=(0,1) → (re,im) → (7·im, re)
    let seven_im = gl_mul_small(b_sq.z, b_sq.w, 7u);
    let u_b_sq = vec4<u32>(seven_im.x, seven_im.y, b_sq.x, b_sq.y);
    let n = fp2_sub(a_sq, u_b_sq);
    let n_inv = fp2_inv(n);

    // result = (A·n_inv, -B·n_inv)
    let r_a = fp2_mul(a, n_inv);
    let neg_b = fp2_neg(b);
    let r_b = fp2_mul(neg_b, n_inv);

    return Fp4(
        vec2<u32>(r_a.x, r_a.y),
        vec2<u32>(r_b.x, r_b.y),
        vec2<u32>(r_a.z, r_a.w),
        vec2<u32>(r_b.z, r_b.w),
    );
}

// Frobenius: σ(w) = 2⁴⁸·w
// σ(c0 + c1·w + c2·w² + c3·w³) = (c0, 2⁴⁸·c1, -c2, -2⁴⁸·c3)
const W_FROB_LO: u32 = 0x00000000u;  // 2^48 low 32 bits
const W_FROB_HI: u32 = 0x00010000u;  // 2^48 high 32 bits

fn fp4_frobenius(x: Fp4) -> Fp4 {
    let fc1 = gl_mul(x.c1.x, x.c1.y, W_FROB_LO, W_FROB_HI);
    let neg_c2 = gl_neg(x.c2.x, x.c2.y);
    let fc3 = gl_mul(x.c3.x, x.c3.y, W_FROB_LO, W_FROB_HI);
    let neg_fc3 = gl_neg(fc3.x, fc3.y);
    return Fp4(x.c0, fc1, neg_c2, neg_fc3);
}
