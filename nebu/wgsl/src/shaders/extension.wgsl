// ── nebu/extension ──────────────────────────────────────────────
//
// Composite extension field module: Fp2 + Fp3 + Fp4.
// Requires: field.wgsl (concatenated before this file)

// ================================================================
// Fp2 — Quadratic extension F_{p²} = F_p[u] / (u² - 7)
// ================================================================

// ── Fp2 helpers ──────────────────────────────────────────────────

fn fp2_zero() -> vec4<u32> {
    return vec4<u32>(0u, 0u, 0u, 0u);
}

fn fp2_one() -> vec4<u32> {
    return vec4<u32>(1u, 0u, 0u, 0u);
}

fn fp2_new(re_lo: u32, re_hi: u32, im_lo: u32, im_hi: u32) -> vec4<u32> {
    return vec4<u32>(re_lo, re_hi, im_lo, im_hi);
}

fn fp2_from_base(a_lo: u32, a_hi: u32) -> vec4<u32> {
    return vec4<u32>(a_lo, a_hi, 0u, 0u);
}

// ── Fp2 arithmetic ───────────────────────────────────────────────

fn fp2_add(x: vec4<u32>, y: vec4<u32>) -> vec4<u32> {
    let re = gl_add(x.x, x.y, y.x, y.y);
    let im = gl_add(x.z, x.w, y.z, y.w);
    return vec4<u32>(re.x, re.y, im.x, im.y);
}

fn fp2_sub(x: vec4<u32>, y: vec4<u32>) -> vec4<u32> {
    let re = gl_sub(x.x, x.y, y.x, y.y);
    let im = gl_sub(x.z, x.w, y.z, y.w);
    return vec4<u32>(re.x, re.y, im.x, im.y);
}

fn fp2_neg(x: vec4<u32>) -> vec4<u32> {
    let re = gl_neg(x.x, x.y);
    let im = gl_neg(x.z, x.w);
    return vec4<u32>(re.x, re.y, im.x, im.y);
}

// Karatsuba: 3 base muls + 1 mul-by-7 + 5 add/subs.
fn fp2_mul(x: vec4<u32>, y: vec4<u32>) -> vec4<u32> {
    let v0 = gl_mul(x.x, x.y, y.x, y.y);          // a * c
    let v1 = gl_mul(x.z, x.w, y.z, y.w);          // b * d
    let seven_v1 = gl_mul_small(v1.x, v1.y, 7u);  // 7 * b * d
    let re = gl_add(v0.x, v0.y, seven_v1.x, seven_v1.y);  // ac + 7bd

    let a_plus_b = gl_add(x.x, x.y, x.z, x.w);   // a + b
    let c_plus_d = gl_add(y.x, y.y, y.z, y.w);   // c + d
    let cross = gl_mul(a_plus_b.x, a_plus_b.y, c_plus_d.x, c_plus_d.y);
    let im_t = gl_sub(cross.x, cross.y, v0.x, v0.y);
    let im = gl_sub(im_t.x, im_t.y, v1.x, v1.y);  // (a+b)(c+d) - ac - bd

    return vec4<u32>(re.x, re.y, im.x, im.y);
}

// Optimized squaring: 2 muls + small-constant muls.
fn fp2_sqr(x: vec4<u32>) -> vec4<u32> {
    let ab = gl_mul(x.x, x.y, x.z, x.w);             // a * b
    let a_plus_b = gl_add(x.x, x.y, x.z, x.w);       // a + b
    let seven_b = gl_mul_small(x.z, x.w, 7u);         // 7 * b
    let a_plus_7b = gl_add(x.x, x.y, seven_b.x, seven_b.y);  // a + 7b
    let prod = gl_mul(a_plus_b.x, a_plus_b.y, a_plus_7b.x, a_plus_7b.y);
    let eight_ab = gl_mul_small(ab.x, ab.y, 8u);      // 8 * ab
    let re = gl_sub(prod.x, prod.y, eight_ab.x, eight_ab.y);
    let im = gl_double(ab.x, ab.y);                   // 2 * ab

    return vec4<u32>(re.x, re.y, im.x, im.y);
}

// Conjugate: (a, b) → (a, -b)
fn fp2_conj(x: vec4<u32>) -> vec4<u32> {
    let neg_im = gl_neg(x.z, x.w);
    return vec4<u32>(x.x, x.y, neg_im.x, neg_im.y);
}

// Norm: a² - 7b² (in F_p)
fn fp2_norm(x: vec4<u32>) -> vec2<u32> {
    let a2 = gl_square(x.x, x.y);
    let b2 = gl_square(x.z, x.w);
    let seven_b2 = gl_mul_small(b2.x, b2.y, 7u);
    return gl_sub(a2.x, a2.y, seven_b2.x, seven_b2.y);
}

// Inversion: (a + bu)^(-1) = (a - bu) / (a² - 7b²)
fn fp2_inv(x: vec4<u32>) -> vec4<u32> {
    let n = fp2_norm(x);
    let n_inv = gl_inv(n.x, n.y);
    let re = gl_mul(x.x, x.y, n_inv.x, n_inv.y);
    let neg_b = gl_neg(x.z, x.w);
    let im = gl_mul(neg_b.x, neg_b.y, n_inv.x, n_inv.y);
    return vec4<u32>(re.x, re.y, im.x, im.y);
}

// ================================================================
// Fp3 — Cubic extension F_{p³} = F_p[t] / (t³ - t - 1)
// ================================================================

// ── Fp3 helpers ─────────────────────────────────────────────────

struct Fp3 {
    c0: vec2<u32>,
    c1: vec2<u32>,
    c2: vec2<u32>,
}

fn fp3_zero() -> Fp3 {
    return Fp3(vec2<u32>(0u, 0u), vec2<u32>(0u, 0u), vec2<u32>(0u, 0u));
}

fn fp3_one() -> Fp3 {
    return Fp3(vec2<u32>(1u, 0u), vec2<u32>(0u, 0u), vec2<u32>(0u, 0u));
}

fn fp3_from_base(a_lo: u32, a_hi: u32) -> Fp3 {
    return Fp3(vec2<u32>(a_lo, a_hi), vec2<u32>(0u, 0u), vec2<u32>(0u, 0u));
}

// ── Fp3 arithmetic ──────────────────────────────────────────────

fn fp3_add(x: Fp3, y: Fp3) -> Fp3 {
    return Fp3(
        gl_add(x.c0.x, x.c0.y, y.c0.x, y.c0.y),
        gl_add(x.c1.x, x.c1.y, y.c1.x, y.c1.y),
        gl_add(x.c2.x, x.c2.y, y.c2.x, y.c2.y),
    );
}

fn fp3_sub(x: Fp3, y: Fp3) -> Fp3 {
    return Fp3(
        gl_sub(x.c0.x, x.c0.y, y.c0.x, y.c0.y),
        gl_sub(x.c1.x, x.c1.y, y.c1.x, y.c1.y),
        gl_sub(x.c2.x, x.c2.y, y.c2.x, y.c2.y),
    );
}

fn fp3_neg(x: Fp3) -> Fp3 {
    return Fp3(
        gl_neg(x.c0.x, x.c0.y),
        gl_neg(x.c1.x, x.c1.y),
        gl_neg(x.c2.x, x.c2.y),
    );
}

// Schoolbook multiplication: 9 base muls + reduction via t³ = t + 1.
fn fp3_mul(x: Fp3, y: Fp3) -> Fp3 {
    let a0b0 = gl_mul(x.c0.x, x.c0.y, y.c0.x, y.c0.y);
    let a0b1 = gl_mul(x.c0.x, x.c0.y, y.c1.x, y.c1.y);
    let a0b2 = gl_mul(x.c0.x, x.c0.y, y.c2.x, y.c2.y);
    let a1b0 = gl_mul(x.c1.x, x.c1.y, y.c0.x, y.c0.y);
    let a1b1 = gl_mul(x.c1.x, x.c1.y, y.c1.x, y.c1.y);
    let a1b2 = gl_mul(x.c1.x, x.c1.y, y.c2.x, y.c2.y);
    let a2b0 = gl_mul(x.c2.x, x.c2.y, y.c0.x, y.c0.y);
    let a2b1 = gl_mul(x.c2.x, x.c2.y, y.c1.x, y.c1.y);
    let a2b2 = gl_mul(x.c2.x, x.c2.y, y.c2.x, y.c2.y);

    let d0 = a0b0;
    let d1 = gl_add(a0b1.x, a0b1.y, a1b0.x, a1b0.y);
    let d2_t = gl_add(a0b2.x, a0b2.y, a1b1.x, a1b1.y);
    let d2 = gl_add(d2_t.x, d2_t.y, a2b0.x, a2b0.y);
    let d3 = gl_add(a1b2.x, a1b2.y, a2b1.x, a2b1.y);
    let d4 = a2b2;

    // Reduce: t³ = t + 1, t⁴ = t² + t
    let c0 = gl_add(d0.x, d0.y, d3.x, d3.y);
    let c1_t = gl_add(d1.x, d1.y, d3.x, d3.y);
    let c1 = gl_add(c1_t.x, c1_t.y, d4.x, d4.y);
    let c2 = gl_add(d2.x, d2.y, d4.x, d4.y);

    return Fp3(c0, c1, c2);
}

// Norm: F_{p³} → F_p.
fn fp3_norm(x: Fp3) -> vec2<u32> {
    let c0_2 = gl_square(x.c0.x, x.c0.y);
    let c1_2 = gl_square(x.c1.x, x.c1.y);
    let c2_2 = gl_square(x.c2.x, x.c2.y);
    let c0_3 = gl_mul(c0_2.x, c0_2.y, x.c0.x, x.c0.y);
    let c1_3 = gl_mul(c1_2.x, c1_2.y, x.c1.x, x.c1.y);
    let c2_3 = gl_mul(c2_2.x, c2_2.y, x.c2.x, x.c2.y);
    let c0c1 = gl_mul(x.c0.x, x.c0.y, x.c1.x, x.c1.y);
    let c0c2 = gl_mul(x.c0.x, x.c0.y, x.c2.x, x.c2.y);
    let c1c2 = gl_mul(x.c1.x, x.c1.y, x.c2.x, x.c2.y);
    let c0c1c2 = gl_mul(c0c1.x, c0c1.y, x.c2.x, x.c2.y);
    let three_c0c1c2 = gl_mul_small(c0c1c2.x, c0c1c2.y, 3u);
    let two_c02_c2 = gl_double(gl_mul(c0_2.x, c0_2.y, x.c2.x, x.c2.y));
    let c0_c22 = gl_mul(x.c0.x, x.c0.y, c2_2.x, c2_2.y);
    let c1_c22 = gl_mul(x.c1.x, x.c1.y, c2_2.x, c2_2.y);
    let c0_c12 = gl_mul(x.c0.x, x.c0.y, c1_2.x, c1_2.y);

    // c0³ + c1³ + c2³ - 3·c0c1c2 + 2·c0²c2 + c0·c2² - c1·c2² - c0·c1²
    var r = gl_add(c0_3.x, c0_3.y, c1_3.x, c1_3.y);
    r = gl_add(r.x, r.y, c2_3.x, c2_3.y);
    r = gl_sub(r.x, r.y, three_c0c1c2.x, three_c0c1c2.y);
    r = gl_add(r.x, r.y, two_c02_c2.x, two_c02_c2.y);
    r = gl_add(r.x, r.y, c0_c22.x, c0_c22.y);
    r = gl_sub(r.x, r.y, c1_c22.x, c1_c22.y);
    r = gl_sub(r.x, r.y, c0_c12.x, c0_c12.y);
    return r;
}

// Inversion via norm and adjugate.
fn fp3_inv(x: Fp3) -> Fp3 {
    let n = fp3_norm(x);
    let n_inv = gl_inv(n.x, n.y);

    let c0_2 = gl_square(x.c0.x, x.c0.y);
    let c1_2 = gl_square(x.c1.x, x.c1.y);
    let c2_2 = gl_square(x.c2.x, x.c2.y);
    let c0c1 = gl_mul(x.c0.x, x.c0.y, x.c1.x, x.c1.y);
    let c0c2 = gl_mul(x.c0.x, x.c0.y, x.c2.x, x.c2.y);
    let c1c2 = gl_mul(x.c1.x, x.c1.y, x.c2.x, x.c2.y);

    // r0 = c0² + 2·c0·c2 - c1² - c1·c2 + c2²
    var r0 = gl_add(c0_2.x, c0_2.y, gl_double(c0c2).x, gl_double(c0c2).y);
    r0 = gl_sub(r0.x, r0.y, c1_2.x, c1_2.y);
    r0 = gl_sub(r0.x, r0.y, c1c2.x, c1c2.y);
    r0 = gl_add(r0.x, r0.y, c2_2.x, c2_2.y);
    r0 = gl_mul(r0.x, r0.y, n_inv.x, n_inv.y);

    // r1 = c2² - c0·c1
    var r1 = gl_sub(c2_2.x, c2_2.y, c0c1.x, c0c1.y);
    r1 = gl_mul(r1.x, r1.y, n_inv.x, n_inv.y);

    // r2 = c1² - c0·c2 - c2²
    var r2 = gl_sub(c1_2.x, c1_2.y, c0c2.x, c0c2.y);
    r2 = gl_sub(r2.x, r2.y, c2_2.x, c2_2.y);
    r2 = gl_mul(r2.x, r2.y, n_inv.x, n_inv.y);

    return Fp3(r0, r1, r2);
}

// ================================================================
// Fp4 — Quartic extension F_{p⁴} = F_p[w] / (w⁴ - 7)
// ================================================================

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
