// ── nebu/extension ───────────────────────────────────────────────
//
// Quadratic extension F_{p²} = F_p[u] / (u² - 7).
// Elements are (re, im) as vec4<u32>(re_lo, re_hi, im_lo, im_hi).
// Requires: field.wgsl

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
