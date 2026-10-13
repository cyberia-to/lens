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
template <int KK>
static inline void permute_k(thread ulong (*s)[16]) {
    for (int k = 0; k < KK; k++) {
        external(s[k]);
        for (int r = 0; r < 4; r++) full_round(s[k], r);
    }
    for (int r = 0; r < 16; r++) {
        ulong x[KK];
        ulong pre[KK];
        ulong acc = 1;
        for (int k = 0; k < KK; k++) {
            x[k] = gadd(s[k][0], RC[128 + r]);
            pre[k] = acc;
            acc = gmul(acc, x[k] == 0 ? 1 : x[k]);
        }
        ulong inv = ginv(acc);
        for (int k = KK - 1; k >= 0; k--) {
            ulong xk = x[k] == 0 ? 1 : x[k];
            ulong ik = gmul(inv, pre[k]);
            inv = gmul(inv, xk);
            s[k][0] = x[k] == 0 ? 0 : ik;
        }
        for (int k = 0; k < KK; k++) {
            ulong sum = s[k][0];
            for (int i = 1; i < 16; i++) sum = gadd(sum, s[k][i]);
            for (int i = 0; i < 16; i++) s[k][i] = gadd(gmul(s[k][i], DIAG[i]), sum);
        }
    }
    for (int k = 0; k < KK; k++) {
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
    permute_k<K>(s);
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
    permute_k<K>(s);
    for (int k = 0; k < K; k++) {
        if ((s[k][0] & g.mask) == 0) atomic_fetch_min_explicit(found, first + k, memory_order_relaxed);
    }
}

// ── overwrite sponge per item (Merkle leaves) ──

struct SpongeParams {
    ulong init[16];
    uint rate;
    uint blocks;
    uint items;
    uint pad;
};

kernel void hemera_sponge(device const ulong* lanes [[buffer(0)]],
                          device ulong* out [[buffer(1)]],
                          constant SpongeParams& p [[buffer(2)]],
                          uint gid [[thread_position_in_grid]]) {
    if (gid >= p.items) return;
    ulong s[1][16];
    for (int i = 0; i < 16; i++) s[0][i] = p.init[i];
    device const ulong* src = lanes + (ulong)gid * p.rate * p.blocks;
    for (uint b = 0; b < p.blocks; b++) {
        for (uint i = 0; i < 16; i++) {
            if (i < p.rate) s[0][i] = src[b * p.rate + i];
        }
        permute_k<1>(s);
    }
    for (int i = 0; i < 4; i++) out[4 * (ulong)gid + i] = s[0][i];
}

// ── straight-line Fp3 programs along lines (zerocheck rounds) ──
// Fp3 = F[t]/(t³ − t − 1), limbs c0 + c1·t + c2·t².

struct f3 { ulong a; ulong b; ulong c; };

static inline f3 f3add(f3 x, f3 y) { return f3{gadd(x.a, y.a), gadd(x.b, y.b), gadd(x.c, y.c)}; }
static inline f3 f3sub(f3 x, f3 y) { return f3{gsub(x.a, y.a), gsub(x.b, y.b), gsub(x.c, y.c)}; }
static inline f3 f3neg(f3 x) { return f3{gsub(0, x.a), gsub(0, x.b), gsub(0, x.c)}; }
static inline f3 f3mulb(f3 x, ulong k) { return f3{gmul(x.a, k), gmul(x.b, k), gmul(x.c, k)}; }
static inline f3 f3mul(f3 x, f3 y) {
    ulong d0 = gmul(x.a, y.a);
    ulong d1 = gadd(gmul(x.a, y.b), gmul(x.b, y.a));
    ulong d2 = gadd(gadd(gmul(x.a, y.c), gmul(x.b, y.b)), gmul(x.c, y.a));
    ulong d3 = gadd(gmul(x.b, y.c), gmul(x.c, y.b));
    ulong d4 = gmul(x.c, y.c);
    // t³ = t + 1, t⁴ = t² + t
    return f3{gadd(d0, d3), gadd(gadd(d1, d3), d4), gadd(d2, d4)};
}

struct LineParams {
    uint nops;
    uint ncols;
    uint len;       // rows per column (2 · pairs)
    uint nx;        // points per pair
    uint threads;   // pairs · nx in this dispatch
    uint first;     // first pair of this dispatch
    uint output;    // the output operand
    uint pad;
    ulong xs[16];
};

#define MAXREGS 160

static inline f3 rd(uint o, device const ulong* consts, device const ulong* cols, thread const f3* r,
                    uint len, uint i, ulong x) {
    uint t = o >> 30, k = o & 0x3FFFFFFFu;
    if (t == 2) return r[k];
    if (t == 1) return f3{consts[3 * k], consts[3 * k + 1], consts[3 * k + 2]};
    device const ulong* pa = cols + 3 * ((ulong)k * len + 2 * i);
    f3 a = f3{pa[0], pa[1], pa[2]};
    f3 b = f3{pa[3], pa[4], pa[5]};
    return f3add(a, f3mulb(f3sub(b, a), x));
}

kernel void prog_lines(device const uint4* ops [[buffer(0)]],
                       device const ulong* consts [[buffer(1)]],
                       device const ulong* cols [[buffer(2)]],
                       device const ulong* weights [[buffer(3)]],
                       device ulong* out [[buffer(4)]],
                       constant LineParams& p [[buffer(5)]],
                       uint gid [[thread_position_in_grid]]) {
    if (gid >= p.threads) return;
    uint i = p.first + gid / p.nx;
    ulong x = p.xs[gid % p.nx];
    f3 r[MAXREGS];
    for (uint n = 0; n < p.nops; n++) {
        uint4 op = ops[n];
        f3 a = rd(op.y, consts, cols, r, p.len, i, x);
        f3 v;
        if (op.x == 0) v = f3add(a, rd(op.z, consts, cols, r, p.len, i, x));
        else if (op.x == 1) v = f3sub(a, rd(op.z, consts, cols, r, p.len, i, x));
        else if (op.x == 2) v = f3mul(a, rd(op.z, consts, cols, r, p.len, i, x));
        else if (op.x == 3) v = f3neg(a);
        else v = f3mulb(a, consts[3 * (op.z & 0x3FFFFFFFu)]);
        r[op.w] = v;
    }
    f3 g = f3{weights[3 * i], weights[3 * i + 1], weights[3 * i + 2]};
    f3 res = f3mul(g, rd(p.output, consts, cols, r, p.len, i, x));
    out[3 * (ulong)gid] = res.a;
    out[3 * (ulong)gid + 1] = res.b;
    out[3 * (ulong)gid + 2] = res.c;
}

// ── streamed Reed–Solomon leaves: sponges read straight from coset values ──

struct CosetLeafParams {
    ulong init[16];
    uint k;        // coset length
    uint per;      // leaves per coset
    uint g;        // cosets in this call
    uint w;        // symbols per member and leaf
    uint members;
    uint limbs;    // 1 (base) or 3 (Fp3)
    uint spb;      // symbols per block
    uint blocks;
    uint rate;
    uint pad;
};

// vals[((m·L + l)·G + c)·k + u]; leaf (c, a) hashes, member by member,
// symbols u = a + t·per (t < w) of coset c
kernel void coset_leaves(device const ulong* vals [[buffer(0)]],
                         device ulong* out [[buffer(1)]],
                         constant CosetLeafParams& p [[buffer(2)]],
                         uint gid [[thread_position_in_grid]]) {
    if (gid >= p.g * p.per) return;
    uint c = gid / p.per;
    uint a = gid % p.per;
    ulong s[1][16];
    for (int i = 0; i < 16; i++) s[0][i] = p.init[i];
    uint total = p.members * p.w;
    for (uint blk = 0; blk < p.blocks; blk++) {
        uint lane = 0;
        for (uint j = 0; j < p.spb; j++) {
            uint t = blk * p.spb + j;
            if (t >= total) break;
            uint m = t / p.w, tt = t % p.w;
            ulong u = a + (ulong)tt * p.per;
            for (uint l = 0; l < p.limbs; l++) {
                s[0][lane + l] = vals[(((ulong)(m * p.limbs + l)) * p.g + c) * p.k + u];
            }
            lane += p.limbs;
        }
        for (uint i = lane; i < p.rate; i++) s[0][i] = 0;
        permute_k<1>(s);
    }
    for (int i = 0; i < 4; i++) out[4 * (ulong)gid + i] = s[0][i];
}

// column folds of a line session: dst[c·stride + i] = v + α·(w − v),
// v = src[c·stride + 2i], w = src[c·stride + 2i + 1]
struct FoldParams {
    ulong alpha[3];
    uint ncols;
    uint stride;
    uint halfn;
    uint pad;
};

kernel void fold_cols(device const ulong* src [[buffer(0)]],
                      device ulong* dst [[buffer(1)]],
                      constant FoldParams& p [[buffer(2)]],
                      uint gid [[thread_position_in_grid]]) {
    if (gid >= p.ncols * p.halfn) return;
    uint c = gid / p.halfn, i = gid % p.halfn;
    device const ulong* s = src + 3 * ((ulong)c * p.stride + 2 * i);
    f3 v = f3{s[0], s[1], s[2]};
    f3 w = f3{s[3], s[4], s[5]};
    f3 r = f3add(v, f3mul(f3{p.alpha[0], p.alpha[1], p.alpha[2]}, f3sub(w, v)));
    device ulong* d = dst + 3 * ((ulong)c * p.stride + i);
    d[0] = r.a; d[1] = r.b; d[2] = r.c;
}

// ── coset NTT (nebu's radix-2 DIT: bit-reversed input, natural output) ──

struct NttParams {
    uint log_n;   // coset length 2^log_n
    uint span;    // stage butterfly span (ntt_stage)
    uint cosets;
    uint pad;
};

static inline ulong gpow(ulong b, uint e) {
    ulong r = 1;
    while (e != 0) {
        if (e & 1) r = gmul(r, b);
        b = gsqr(b);
        e >>= 1;
    }
    return r;
}

// data[s·n + bitrev(i)] = coeffs[i] · shifts[s]^i
kernel void ntt_scatter(device const ulong* coeffs [[buffer(0)]],
                        device ulong* data [[buffer(1)]],
                        device const ulong* shifts [[buffer(2)]],
                        constant NttParams& p [[buffer(3)]],
                        uint gid [[thread_position_in_grid]]) {
    uint n = 1u << p.log_n;
    if (gid >= n * p.cosets) return;
    uint s = gid >> p.log_n;
    uint i = gid & (n - 1);
    uint r = reverse_bits(i) >> (32 - p.log_n);
    if (p.log_n == 0) r = 0;
    data[(ulong)s * n + r] = gmul(coeffs[i], gpow(shifts[s], i));
}

// one stage: butterflies of span `half` with twiddles tw[half + i] = ω_{2·half}^i
kernel void ntt_stage(device ulong* data [[buffer(0)]],
                      device const ulong* tw [[buffer(1)]],
                      constant NttParams& p [[buffer(2)]],
                      uint gid [[thread_position_in_grid]]) {
    uint n = 1u << p.log_n;
    uint per = n >> 1;
    if (gid >= per * p.cosets) return;
    uint s = gid / per;
    uint b = gid - s * per;
    uint i = b & (p.span - 1);
    uint j = (b - i) << 1;
    device ulong* a = data + (ulong)s * n + j + i;
    ulong t = gmul(tw[p.span + i], a[p.span]);
    ulong u = a[0];
    a[p.span] = gsub(u, t);
    a[0] = gadd(u, t);
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
