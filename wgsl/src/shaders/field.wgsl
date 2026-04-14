// ── nebu/field ───────────────────────────────────────────────────
//
// Goldilocks field arithmetic (p = 2^64 - 2^32 + 1).
// All u64 values emulated as vec2<u32>(lo, hi).

const P_LO: u32 = 0x00000001u;
const P_HI: u32 = 0xFFFFFFFFu;
const EPSILON: u32 = 0xFFFFFFFFu;  // 2^32 - 1

// ── 64-bit helpers ───────────────────────────────────────────────

fn add64(a_lo: u32, a_hi: u32, b_lo: u32, b_hi: u32) -> vec2<u32> {
    let lo = a_lo + b_lo;
    let carry = select(0u, 1u, lo < a_lo);
    let hi = a_hi + b_hi + carry;
    return vec2<u32>(lo, hi);
}

fn sub64(a_lo: u32, a_hi: u32, b_lo: u32, b_hi: u32) -> vec2<u32> {
    let borrow = select(0u, 1u, a_lo < b_lo);
    let lo = a_lo - b_lo;
    let hi = a_hi - b_hi - borrow;
    return vec2<u32>(lo, hi);
}

fn gte64(a_lo: u32, a_hi: u32, b_lo: u32, b_hi: u32) -> bool {
    if a_hi > b_hi { return true; }
    if a_hi < b_hi { return false; }
    return a_lo >= b_lo;
}

fn is_zero64(lo: u32, hi: u32) -> bool {
    return lo == 0u && hi == 0u;
}

fn eq64(a_lo: u32, a_hi: u32, b_lo: u32, b_hi: u32) -> bool {
    return a_lo == b_lo && a_hi == b_hi;
}

// ── 32x32 → 64-bit multiply ─────────────────────────────────────

fn mul32(a: u32, b: u32) -> vec2<u32> {
    let a_lo = a & 0xFFFFu;
    let a_hi = a >> 16u;
    let b_lo = b & 0xFFFFu;
    let b_hi = b >> 16u;

    let ll = a_lo * b_lo;
    let lh = a_lo * b_hi;
    let hl = a_hi * b_lo;
    let hh = a_hi * b_hi;

    let mid = lh + (ll >> 16u);
    let mid2 = (mid & 0xFFFFu) + hl;

    let lo = (mid2 << 16u) | (ll & 0xFFFFu);
    let hi = hh + (mid >> 16u) + (mid2 >> 16u);
    return vec2<u32>(lo, hi);
}

// ── Goldilocks reduction ─────────────────────────────────────────

fn gl_reduce(lo: u32, hi: u32) -> vec2<u32> {
    if gte64(lo, hi, P_LO, P_HI) {
        return sub64(lo, hi, P_LO, P_HI);
    }
    return vec2<u32>(lo, hi);
}

fn gl_canon(lo: u32, hi: u32) -> vec2<u32> {
    return gl_reduce(lo, hi);
}

// ── Field arithmetic ─────────────────────────────────────────────

fn gl_add(a_lo: u32, a_hi: u32, b_lo: u32, b_hi: u32) -> vec2<u32> {
    let sum = add64(a_lo, a_hi, b_lo, b_hi);
    let overflow = (sum.y < a_hi) || (sum.y == a_hi && sum.x < a_lo);
    if overflow {
        let adj = add64(sum.x, sum.y, EPSILON, 0u);
        let overflow2 = (adj.y < sum.y) || (adj.y == sum.y && adj.x < sum.x);
        if overflow2 {
            let adj2 = add64(adj.x, adj.y, EPSILON, 0u);
            return gl_reduce(adj2.x, adj2.y);
        }
        return gl_reduce(adj.x, adj.y);
    }
    return gl_reduce(sum.x, sum.y);
}

fn gl_sub(a_lo: u32, a_hi: u32, b_lo: u32, b_hi: u32) -> vec2<u32> {
    let diff = sub64(a_lo, a_hi, b_lo, b_hi);
    let underflow = (a_hi < b_hi) || (a_hi == b_hi && a_lo < b_lo);
    if underflow {
        let adj = sub64(diff.x, diff.y, EPSILON, 0u);
        let underflow2 = (diff.y == 0u && diff.x < EPSILON);
        if underflow2 {
            let adj2 = sub64(adj.x, adj.y, EPSILON, 0u);
            return adj2;
        }
        return adj;
    }
    return diff;
}

fn gl_neg(a_lo: u32, a_hi: u32) -> vec2<u32> {
    if is_zero64(a_lo, a_hi) {
        return vec2<u32>(0u, 0u);
    }
    return sub64(P_LO, P_HI, a_lo, a_hi);
}

fn gl_mul(a_lo: u32, a_hi: u32, b_lo: u32, b_hi: u32) -> vec2<u32> {
    // 64x64 → 128 bit product as (r0, r1, r2, r3)
    let ll = mul32(a_lo, b_lo);
    let lh = mul32(a_lo, b_hi);
    let hl = mul32(a_hi, b_lo);
    let hh = mul32(a_hi, b_hi);

    let r0 = ll.x;
    let t1 = add64(ll.y, 0u, lh.x, 0u);
    let t2 = add64(t1.x, t1.y, hl.x, 0u);
    let r1 = t2.x;
    let carry1 = t2.y;
    let t3 = add64(lh.y, 0u, hl.y, 0u);
    let t4 = add64(t3.x, t3.y, hh.x, 0u);
    let t5 = add64(t4.x, t4.y, carry1, 0u);
    let r2 = t5.x;
    let carry2 = t5.y;
    let r3 = hh.y + carry2;

    // reduce128: x_lo=(r0,r1), x_hi=(r2,r3)
    // x_hi_hi = r3, x_hi_lo = r2
    // t0 = x_lo - x_hi_hi (with borrow correction via -ε)
    let sub_borrow = select(0u, 1u, r0 < r3);
    var t0_lo = r0 - r3;
    var t0_hi = r1 - sub_borrow;
    let real_borrow = (r1 == 0u && r0 < r3) || (r1 < sub_borrow);
    if real_borrow {
        let sub2_borrow = select(0u, 1u, t0_lo < EPSILON);
        t0_lo = t0_lo - EPSILON;
        t0_hi = t0_hi - sub2_borrow;
    }

    // t1 = x_hi_lo * ε
    let t1_val = mul32(r2, EPSILON);
    let res = add64(t0_lo, t0_hi, t1_val.x, t1_val.y);
    let add_carry = (res.y < t0_hi) || (res.y == t0_hi && res.x < t0_lo);

    if add_carry {
        let final_val = add64(res.x, res.y, EPSILON, 0u);
        return final_val;
    }
    return vec2<u32>(res.x, res.y);
}

fn gl_square(x_lo: u32, x_hi: u32) -> vec2<u32> {
    return gl_mul(x_lo, x_hi, x_lo, x_hi);
}

fn gl_double(x_lo: u32, x_hi: u32) -> vec2<u32> {
    return gl_add(x_lo, x_hi, x_lo, x_hi);
}

fn gl_pow7(x_lo: u32, x_hi: u32) -> vec2<u32> {
    let x2 = gl_square(x_lo, x_hi);
    let x3 = gl_mul(x2.x, x2.y, x_lo, x_hi);
    let x4 = gl_mul(x2.x, x2.y, x2.x, x2.y);
    let x7 = gl_mul(x3.x, x3.y, x4.x, x4.y);
    return x7;
}

// ── Exponentiation ───────────────────────────────────────────────
// Square-and-multiply for arbitrary 64-bit exponent (lo, hi).

fn gl_exp(base_lo: u32, base_hi: u32, exp_lo: u32, exp_hi: u32) -> vec2<u32> {
    var result = vec2<u32>(1u, 0u);  // ONE
    var b = vec2<u32>(base_lo, base_hi);

    // Process low 32 bits
    var e = exp_lo;
    for (var i = 0u; i < 32u; i++) {
        if (e & 1u) != 0u {
            result = gl_mul(result.x, result.y, b.x, b.y);
        }
        b = gl_square(b.x, b.y);
        e >>= 1u;
    }

    // Process high 32 bits
    e = exp_hi;
    for (var i = 0u; i < 32u; i++) {
        if (e & 1u) != 0u {
            result = gl_mul(result.x, result.y, b.x, b.y);
        }
        b = gl_square(b.x, b.y);
        e >>= 1u;
    }

    return result;
}

// ── Inversion ────────────────────────────────────────────────────
// a^(p-2) via square-and-multiply.
// p-2 = 0xFFFFFFFEFFFFFFFF: all bits set except bit 32.

fn gl_inv(a_lo: u32, a_hi: u32) -> vec2<u32> {
    // p - 2 = (lo=0xFFFFFFFF, hi=0xFFFFFFFE)
    return gl_exp(a_lo, a_hi, 0xFFFFFFFFu, 0xFFFFFFFEu);
}

// ── Legendre symbol ──────────────────────────────────────────────
// a^((p-1)/2). Returns 0, 1, or p-1.

fn gl_legendre(a_lo: u32, a_hi: u32) -> vec2<u32> {
    // (p-1)/2 = 0x7FFFFFFF80000000 = (lo=0x80000000, hi=0x7FFFFFFF)
    return gl_exp(a_lo, a_hi, 0x80000000u, 0x7FFFFFFFu);
}

// ── Multiply by small constant ───────────────────────────────────

fn gl_mul_small(a_lo: u32, a_hi: u32, c: u32) -> vec2<u32> {
    return gl_mul(a_lo, a_hi, c, 0u);
}

// ── Square root (Tonelli-Shanks) ─────────────────────────────────
// p - 1 = 2^32 × ε, z = 7 (QNR), M = 32.
// Returns (root_lo, root_hi, found, _) as vec4.
// found = 1u if n is QR (root valid), 0u if QNR.

fn gl_sqrt(n_lo: u32, n_hi: u32) -> vec4<u32> {
    if is_zero64(n_lo, n_hi) {
        return vec4<u32>(0u, 0u, 1u, 0u);
    }

    // Legendre test
    let leg = gl_legendre(n_lo, n_hi);
    // p-1 = (lo=0x00000000, hi=0xFFFFFFFF)
    if eq64(leg.x, leg.y, 0x00000000u, 0xFFFFFFFFu) {
        return vec4<u32>(0u, 0u, 0u, 0u);  // QNR
    }

    // s = ε = 0xFFFFFFFF (odd part of p-1)
    let s_lo = 0xFFFFFFFFu;
    let s_hi = 0u;

    var big_m = 32u;
    var c = gl_exp(7u, 0u, s_lo, s_hi);                    // 7^s
    var t = gl_exp(n_lo, n_hi, s_lo, s_hi);                // n^s
    // r = n^((s+1)/2) = n^(2^31)  since s = 2^32-1, (s+1)/2 = 2^31
    // (s+1)/2 = (0xFFFFFFFF + 1) / 2 = 0x80000000
    var r = gl_exp(n_lo, n_hi, 0x80000000u, 0x00000000u);  // n^(2^31)

    loop {
        if eq64(t.x, t.y, 1u, 0u) {
            // Canonical sign: r <= (p-1)/2
            // (p-1)/2 = (lo=0x80000000, hi=0x7FFFFFFF)
            if gte64(r.x, r.y, 0x80000001u, 0x7FFFFFFFu) {
                r = gl_neg(r.x, r.y);
            }
            return vec4<u32>(r.x, r.y, 1u, 0u);
        }

        // Find least i > 0 such that t^(2^i) = 1
        var i = 1u;
        var tmp = gl_square(t.x, t.y);
        loop {
            if eq64(tmp.x, tmp.y, 1u, 0u) { break; }
            tmp = gl_square(tmp.x, tmp.y);
            i++;
        }

        // b = c^(2^(M-i-1))
        var b = c;
        for (var j = 0u; j < (big_m - i - 1u); j++) {
            b = gl_square(b.x, b.y);
        }

        big_m = i;
        c = gl_square(b.x, b.y);
        t = gl_mul(t.x, t.y, c.x, c.y);
        r = gl_mul(r.x, r.y, b.x, b.y);
    }
    // unreachable
    return vec4<u32>(0u, 0u, 0u, 0u);
}
