//! Metal Shading Language source: Goldilocks over native `ulong` (`mulhi`
//! for the high product), hemera's permutation with its constants inlined
//! (the compiler sees them as literals), and the kernels.
//!
//! Every value a kernel stores is canonical (`< p`), matching the CPU.

use std::fmt::Write;

use crate::consts;

const FIELD: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant ulong P = 0xFFFFFFFF00000001UL;
constant ulong EPS = 0xFFFFFFFFUL; // 2^64 mod p

// canonical inputs, canonical output
static inline ulong gadd(ulong a, ulong b) {
    ulong s = a + b;
    if (s < a) s += EPS;
    if (s >= P) s -= P;
    return s;
}

static inline ulong gsub(ulong a, ulong b) {
    ulong d = a - b;
    if (a < b) d += P;
    return d;
}

// the 128-bit value hi·2^64 + lo, reduced: 2^64 ≡ 2^32 − 1, 2^96 ≡ −1
static inline ulong red(ulong lo, ulong hi) {
    ulong hh = hi >> 32;
    ulong hl = hi & EPS;
    ulong t0 = lo - hh;
    if (lo < hh) t0 -= EPS;
    ulong t1 = (hl << 32) - hl;
    ulong r = t0 + t1;
    if (r < t1) r += EPS;
    if (r >= P) r -= P;
    return r;
}

// 32-bit limb products (one mul + mulhi each); squaring shares the cross term
static inline ulong gmul(ulong a, ulong b) {
    uint a0 = (uint)a, a1 = (uint)(a >> 32), b0 = (uint)b, b1 = (uint)(b >> 32);
    ulong p00 = (ulong)a0 * b0;
    ulong p01 = (ulong)a0 * b1;
    ulong p10 = (ulong)a1 * b0;
    ulong p11 = (ulong)a1 * b1;
    ulong mid = p01 + p10;
    ulong mc = mid < p01 ? (1UL << 32) : 0;
    ulong lo = p00 + (mid << 32);
    ulong hi = p11 + (mid >> 32) + mc + (lo < p00 ? 1 : 0);
    return red(lo, hi);
}
static inline ulong gsqr(ulong a) {
    uint a0 = (uint)a, a1 = (uint)(a >> 32);
    ulong p00 = (ulong)a0 * a0;
    ulong p01 = (ulong)a0 * a1;
    ulong p11 = (ulong)a1 * a1;
    ulong lo = p00 + (p01 << 33);
    ulong hi = p11 + (p01 >> 31) + (lo < p00 ? 1 : 0);
    return red(lo, hi);
}

static inline ulong gsqn(ulong x, int n) {
    for (int i = 0; i < n; i++) x = gsqr(x);
    return x;
}

// x^(p-2) by an addition chain (72 multiplications); 0 maps to 0
static inline ulong ginv(ulong x) {
    ulong t2 = gmul(gsqr(x), x);           // 2^2-1
    ulong t3 = gmul(gsqr(t2), x);         // 2^3-1
    ulong t6 = gmul(gsqn(t3, 3), t3);         // 2^6-1
    ulong t12 = gmul(gsqn(t6, 6), t6);        // 2^12-1
    ulong t24 = gmul(gsqn(t12, 12), t12);     // 2^24-1
    ulong t30 = gmul(gsqn(t24, 6), t6);       // 2^30-1
    ulong t31 = gmul(gsqr(t30), x);      // 2^31-1
    ulong u = gsqr(t31);                 // 2^32-2
    ulong c = gmul(u, x);                     // 2^32-1
    return gmul(gsqn(u, 32), c);              // (2^32-2)·2^32 + 2^32-1 = p-2
}

static inline ulong pow7(ulong x) {
    ulong x2 = gsqr(x);
    ulong x3 = gmul(x2, x);
    ulong x4 = gsqr(x2);
    return gmul(x3, x4);
}

static inline void mat4(thread ulong* x) {
    ulong t01 = gadd(x[0], x[1]);
    ulong t23 = gadd(x[2], x[3]);
    ulong t0123 = gadd(t01, t23);
    ulong t01123 = gadd(t0123, x[1]);
    ulong t01233 = gadd(t0123, x[3]);
    ulong x0 = x[0], x2 = x[2];
    x[3] = gadd(t01233, gadd(x0, x0));
    x[1] = gadd(t01123, gadd(x2, x2));
    x[0] = gadd(t01123, t01);
    x[2] = gadd(t01233, t23);
}

static inline void external(thread ulong* s) {
    for (int c = 0; c < 4; c++) mat4(s + 4 * c);
    for (int k = 0; k < 4; k++) {
        ulong sum = gadd(gadd(s[k], s[4 + k]), gadd(s[8 + k], s[12 + k]));
        s[k] = gadd(s[k], sum);
        s[4 + k] = gadd(s[4 + k], sum);
        s[8 + k] = gadd(s[8 + k], sum);
        s[12 + k] = gadd(s[12 + k], sum);
    }
}

static inline void full_round(thread ulong* s, int r) {
    for (int i = 0; i < 16; i++) s[i] = pow7(gadd(s[i], RC[r * 16 + i]));
    external(s);
}

// K states at once: the partial rounds' inversions share one field
// inversion (Montgomery's trick; a zero input inverts to zero).
static inline void permute_k(thread ulong (*s)[16]) {
    for (int k = 0; k < K; k++) {
        external(s[k]);
        for (int r = 0; r < 4; r++) full_round(s[k], r);
    }
    for (int r = 0; r < 16; r++) {
        ulong x[K];
        ulong pre[K];
        ulong acc = 1;
        for (int k = 0; k < K; k++) {
            x[k] = gadd(s[k][0], RC[128 + r]);
            pre[k] = acc;
            acc = gmul(acc, x[k] == 0 ? 1 : x[k]);
        }
        ulong inv = ginv(acc);
        for (int k = K - 1; k >= 0; k--) {
            ulong xk = x[k] == 0 ? 1 : x[k];
            ulong ik = gmul(inv, pre[k]);
            inv = gmul(inv, xk);
            s[k][0] = x[k] == 0 ? 0 : ik;
        }
        for (int k = 0; k < K; k++) {
            ulong sum = s[k][0];
            for (int i = 1; i < 16; i++) sum = gadd(sum, s[k][i]);
            for (int i = 0; i < 16; i++) s[k][i] = gadd(gmul(s[k][i], DIAG[i]), sum);
        }
    }
    for (int k = 0; k < K; k++) {
        for (int r = 4; r < 8; r++) full_round(s[k], r);
    }
}

struct GrindParams {
    ulong base;      // first nonce of the dispatch
    ulong mask;      // low bits that must be zero
    ulong lo_mask;   // nonce bits added to lane lo
    uint lo;
    uint lo_shift;
    uint hi;
    uint hi_shift;   // 64: no high part
    uint count;      // nonces in the dispatch (a multiple of K)
    uint pad;
};

// thread t permutes states K·t .. K·t+K−1 (those below n)
kernel void hemera_permute(device ulong* st [[buffer(0)]],
                           constant uint& n [[buffer(1)]],
                           uint gid [[thread_position_in_grid]]) {
    uint first = gid * K;
    if (first >= n) return;
    ulong s[K][16];
    for (int k = 0; k < K; k++) {
        uint j = min(first + k, n - 1);
        device ulong* p = st + 16 * (ulong)j;
        for (int i = 0; i < 16; i++) s[k][i] = p[i];
    }
    permute_k(s);
    for (int k = 0; k < K; k++) {
        uint j = first + k;
        if (j < n) {
            device ulong* p = st + 16 * (ulong)j;
            for (int i = 0; i < 16; i++) p[i] = s[k][i];
        }
    }
}

// thread t tries nonces base + K·t + k
kernel void hemera_grind(constant ulong* tpl [[buffer(0)]],
                         device atomic_uint* found [[buffer(1)]],
                         constant GrindParams& g [[buffer(2)]],
                         uint gid [[thread_position_in_grid]]) {
    uint first = gid * K;
    if (first >= g.count) return;
    ulong s[K][16];
    for (int k = 0; k < K; k++) {
        ulong n = g.base + first + k;
        ulong lo = (n & g.lo_mask) << g.lo_shift;
        ulong hi = g.hi_shift < 64 ? n >> g.hi_shift : 0;
        for (int i = 0; i < 16; i++) {
            ulong v = tpl[i];
            if (i == (int)g.lo) v = gadd(v, lo);
            if (i == (int)g.hi) v = gadd(v, hi);
            s[k][i] = v;
        }
    }
    permute_k(s);
    for (int k = 0; k < K; k++) {
        if ((s[k][0] & g.mask) == 0) atomic_fetch_min_explicit(found, first + k, memory_order_relaxed);
    }
}
"#;

/// The whole library source, `k` states per thread.
pub fn source(k: usize) -> String {
    let mut s = String::new();
    s.push_str("#include <metal_stdlib>\nusing namespace metal;\n");
    writeln!(s, "#define K {k}").expect("fmt");
    write!(s, "constant ulong RC[144] = {{").expect("fmt");
    for (i, c) in consts::round_constants().iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        write!(s, "0x{c:016x}UL").expect("fmt");
    }
    s.push_str("};\nconstant ulong DIAG[16] = {");
    for (i, c) in consts::diag().iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        write!(s, "0x{c:016x}UL").expect("fmt");
    }
    s.push_str("};\n");
    s.push_str(FIELD);
    s
}
