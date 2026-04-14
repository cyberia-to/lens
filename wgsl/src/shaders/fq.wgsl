// ---
// tags: genies, wgsl
// crystal-type: source
// crystal-domain: comp
// ---

// F_q field arithmetic for CSIDH-512 in WGSL.
// 512-bit integers stored as 16 x u32 limbs, little-endian.
// q = 4 * 3 * 5 * 7 * ... * 587 - 1  (511-bit prime)

struct Fq {
    limbs: array<u32, 16>,
}

// CSIDH-512 prime q as 16 little-endian u32 limbs.
// q = 0x65b48e8f740f89bffc8ab0d15e3e4c4ab42d083aedc88c425afbfcc69322c9cd
//       a7aac6c567f35507516730cc1f0b4f25c2721bf457aca8351b81b90533c6c87b
const Q = array<u32, 16>(
    0x33c6c87bu, 0x1b81b905u, 0x57aca835u, 0xc2721bf4u,
    0x1f0b4f25u, 0x516730ccu, 0x67f35507u, 0xa7aac6c5u,
    0x9322c9cdu, 0x5afbfcc6u, 0xedc88c42u, 0xb42d083au,
    0x5e3e4c4au, 0xfc8ab0d1u, 0x740f89bfu, 0x65b48e8fu
);

// Barrett reduction constant: mu = floor(2^1024 / q).
// mu has 514 bits, stored as 18 u32 limbs (from 9 u64 limbs).
const BARRETT_MU = array<u32, 18>(
    0xe2ffb9d4u, 0x87471983u, 0xeabde765u, 0xabb862a1u,
    0x899eca3bu, 0x48b72f84u, 0xb77624deu, 0xdb7e0542u,
    0xca1bb35au, 0xafaeb264u, 0xff081925u, 0xba24269du,
    0xe0fac030u, 0x5d6cec71u, 0x401fac7fu, 0x845f1c9du,
    0x00000002u, 0x00000000u
);

const FQ_ZERO = Fq(array<u32, 16>(
    0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u
));

const FQ_ONE = Fq(array<u32, 16>(
    1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u
));

// 32x32 -> 64-bit widening multiply. Returns (lo, hi).
fn mul32(a: u32, b: u32) -> vec2<u32> {
    // Split each 32-bit value into two 16-bit halves.
    let a_lo = a & 0xffffu;
    let a_hi = a >> 16u;
    let b_lo = b & 0xffffu;
    let b_hi = b >> 16u;

    let ll = a_lo * b_lo;
    let lh = a_lo * b_hi;
    let hl = a_hi * b_lo;
    let hh = a_hi * b_hi;

    // Combine: result = hh:00 + 0:lh:0 + 0:hl:0 + 00:ll
    let mid = lh + (ll >> 16u);
    let mid2 = (mid & 0xffffu) + hl;

    let lo = (mid2 << 16u) | (ll & 0xffffu);
    let hi = hh + (mid >> 16u) + (mid2 >> 16u);

    return vec2<u32>(lo, hi);
}

// Add with carry: a + b + carry_in -> (sum, carry_out)
fn adc(a: u32, b: u32, carry: u32) -> vec2<u32> {
    let sum = u32(a) + u32(b) + u32(carry);
    // Detect carry: sum < a means overflow, or if sum == a and (b|carry)!=0
    // Simpler: use the fact that if a+b overflows, result < a.
    let ab = a + b;
    var c = u32(ab < a);
    let abc = ab + carry;
    c += u32(abc < ab);
    return vec2<u32>(abc, c);
}

// Subtract with borrow: a - b - borrow_in -> (diff, borrow_out)
fn sbb(a: u32, b: u32, borrow: u32) -> vec2<u32> {
    let d1 = a - b;
    let b1 = u32(d1 > a);
    let d2 = d1 - borrow;
    let b2 = u32(d2 > d1);
    return vec2<u32>(d2, b1 + b2);
}

// fq_add: (a + b) mod q
fn fq_add(a: Fq, b: Fq) -> Fq {
    // Add 16 limbs with carry chain
    var result: array<u32, 16>;
    var carry = 0u;
    for (var i = 0u; i < 16u; i++) {
        let r = adc(a.limbs[i], b.limbs[i], carry);
        result[i] = r.x;
        carry = r.y;
    }

    // Conditional subtract q
    var sub: array<u32, 16>;
    var borrow = 0u;
    for (var i = 0u; i < 16u; i++) {
        let r = sbb(result[i], Q[i], borrow);
        sub[i] = r.x;
        borrow = r.y;
    }

    // If carry >= borrow, subtraction is valid (result >= q)
    if carry >= borrow {
        return Fq(sub);
    }
    return Fq(result);
}

// fq_sub: (a - b) mod q
fn fq_sub(a: Fq, b: Fq) -> Fq {
    var result: array<u32, 16>;
    var borrow = 0u;
    for (var i = 0u; i < 16u; i++) {
        let r = sbb(a.limbs[i], b.limbs[i], borrow);
        result[i] = r.x;
        borrow = r.y;
    }

    // If borrow, add q back
    if borrow != 0u {
        var carry = 0u;
        for (var i = 0u; i < 16u; i++) {
            let r = adc(result[i], Q[i], carry);
            result[i] = r.x;
            carry = r.y;
        }
    }

    return Fq(result);
}

// fq_neg: q - a (or 0 if a == 0)
fn fq_neg(a: Fq) -> Fq {
    var is_zero = true;
    for (var i = 0u; i < 16u; i++) {
        if a.limbs[i] != 0u {
            is_zero = false;
        }
    }
    if is_zero {
        return FQ_ZERO;
    }
    return fq_sub(Fq(Q), a);
}

// Schoolbook 16x16 -> 32-limb multiplication, then Barrett reduction.
fn fq_mul(a: Fq, b: Fq) -> Fq {
    // Schoolbook multiply: 16x16 -> 32 limbs
    var t: array<u32, 32>;
    for (var k = 0u; k < 32u; k++) {
        t[k] = 0u;
    }

    for (var i = 0u; i < 16u; i++) {
        var carry = 0u;
        for (var j = 0u; j < 16u; j++) {
            let m = mul32(a.limbs[i], b.limbs[j]);
            let idx = i + j;
            // Accumulate: t[idx] += m.lo + carry
            let r1 = adc(t[idx], m.x, carry);
            t[idx] = r1.x;
            carry = r1.y + m.y;
        }
        t[i + 16u] = carry;
    }

    // Barrett reduction
    return barrett_reduce(t);
}

// Barrett reduction of 32-limb product to 16-limb result mod q.
// Algorithm (k=16 limbs in u32, base = 2^32):
//   1. q1 = t >> (15*32) = t[15..31]  (17 limbs)
//   2. q2 = q1 * mu                    (up to 35 limbs)
//   3. q3 = q2 >> (17*32) = q2[17..]   (up to 18 limbs)
//   4. r1 = t mod 2^(17*32)            (17 limbs)
//   5. r2 = (q3 * q) mod 2^(17*32)    (17 limbs)
//   6. r = r1 - r2 (mod 2^(17*32))
//   7. while r >= q: r -= q
fn barrett_reduce(t: array<u32, 32>) -> Fq {
    // Step 1: q1 = t[15..31] (17 limbs)
    var q1: array<u32, 17>;
    for (var i = 0u; i < 17u; i++) {
        q1[i] = t[i + 15u];
    }

    // Step 2: q2 = q1 * mu (17 x 18 = up to 35 limbs, we only need from index 17+)
    // Actually we need q3 = q2[17..], so we compute q2 fully.
    var q2: array<u32, 36>;
    for (var k = 0u; k < 36u; k++) {
        q2[k] = 0u;
    }
    for (var i = 0u; i < 17u; i++) {
        var carry = 0u;
        for (var j = 0u; j < 18u; j++) {
            let m = mul32(q1[i], BARRETT_MU[j]);
            let idx = i + j;
            let r1 = adc(q2[idx], m.x, carry);
            q2[idx] = r1.x;
            carry = r1.y + m.y;
        }
        if (i + 18u) < 36u {
            q2[i + 18u] = carry;
        }
    }

    // Step 3: q3 = q2[17..] (up to 19 limbs, but we only need ~17 for q3 * q)
    var q3: array<u32, 18>;
    for (var i = 0u; i < 18u; i++) {
        q3[i] = q2[i + 17u];
    }

    // Step 4: r1 = t mod 2^(17*32) (low 17 limbs of t)
    var r1: array<u32, 17>;
    for (var i = 0u; i < 17u; i++) {
        r1[i] = t[i];
    }

    // Step 5: r2 = (q3 * Q) mod 2^(17*32) (low 17 limbs only)
    var r2: array<u32, 17>;
    for (var k = 0u; k < 17u; k++) {
        r2[k] = 0u;
    }
    for (var i = 0u; i < 18u; i++) {
        var carry = 0u;
        for (var j = 0u; j < 16u; j++) {
            let idx = i + j;
            if idx >= 17u { break; }
            let m = mul32(q3[i], Q[j]);
            let ra = adc(r2[idx], m.x, carry);
            r2[idx] = ra.x;
            carry = ra.y + m.y;
        }
    }

    // Step 6: r = r1 - r2 (mod 2^(17*32))
    var r: array<u32, 17>;
    var borrow = 0u;
    for (var i = 0u; i < 17u; i++) {
        let s = sbb(r1[i], r2[i], borrow);
        r[i] = s.x;
        borrow = s.y;
    }

    // Step 7: while r >= q, subtract q (at most 3 times)
    for (var iter = 0u; iter < 4u; iter++) {
        // Check if r >= q (17-limb vs 16-limb comparison, q[16]=0)
        var ge = true;
        for (var k = 16u; ; ) {
            let qk = select(Q[k], 0u, k >= 16u);
            if r[k] > qk { break; }
            if r[k] < qk { ge = false; break; }
            if k == 0u { break; }
            k--;
        }
        if !ge { break; }

        // Subtract q
        var bw = 0u;
        for (var i = 0u; i < 17u; i++) {
            let qk = select(Q[i], 0u, i >= 16u);
            let s = sbb(r[i], qk, bw);
            r[i] = s.x;
            bw = s.y;
        }
    }

    var out: array<u32, 16>;
    for (var i = 0u; i < 16u; i++) {
        out[i] = r[i];
    }
    return Fq(out);
}
