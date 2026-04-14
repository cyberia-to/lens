// ── nebu/fp3 ────────────────────────────────────────────────────
//
// Cubic extension F_{p³} = F_p[t] / (t³ - t - 1).
// Elements are (c0, c1, c2) as array<vec2<u32>, 3>.
// Each vec2<u32> is (lo, hi) limbs of a Goldilocks element.
// Reduction: t³ = t + 1.
// Requires: field.wgsl

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
