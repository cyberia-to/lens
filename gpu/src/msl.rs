//! Metal Shading Language source: Goldilocks over native `ulong` (`mulhi`
//! for the high product), hemera's permutation (0.4: x⁷ in all 60 rounds,
//! the Poseidon2 reference 4×4 block) with its 240 constants inlined (the
//! compiler sees them as literals), and the kernels.
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

static inline ulong pow7(ulong x) {
    ulong x2 = gsqr(x);
    ulong x3 = gmul(x2, x);
    ulong x4 = gsqr(x2);
    return gmul(x3, x4);
}

// the Poseidon2 reference block [[5,7,1,3],[4,6,1,1],[1,3,5,7],[1,1,4,6]]
// (hemera `apply_mat4`: eight additions, four doublings)
static inline void mat4(thread ulong* x) {
    ulong t0 = gadd(x[0], x[1]);
    ulong t1 = gadd(x[2], x[3]);
    ulong t2 = gadd(gadd(x[1], x[1]), t1);
    ulong t3 = gadd(gadd(x[3], x[3]), t0);
    ulong t1d = gadd(t1, t1);
    ulong t0d = gadd(t0, t0);
    ulong t4 = gadd(gadd(t1d, t1d), t3);
    ulong t5 = gadd(gadd(t0d, t0d), t2);
    x[0] = gadd(t3, t5);
    x[1] = t5;
    x[2] = gadd(t2, t4);
    x[3] = t4;
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

// the definitional permutation, one state: M_E, 6 full rounds, 48 partial
// rounds (x⁷ on lane 0, M_I = 1 + diag(d)), 6 full rounds; canonical
// throughout. The kernels run `permute_k`; this is its readable twin.
static inline void permute_ref(thread ulong* s) {
    external(s);
    for (int r = 0; r < 6; r++) full_round(s, r);
    for (int r = 0; r < 48; r++) {
        s[0] = pow7(gadd(s[0], RC[192 + r]));
        ulong sum = s[0];
        for (int i = 1; i < 16; i++) sum = gadd(sum, s[i]);
        for (int i = 0; i < 16; i++) s[i] = gadd(gmul(s[i], DIAG[i]), sum);
    }
    for (int r = 6; r < 12; r++) full_round(s, r);
}


// ── the fast permutation: hemera's CPU kernel structure on the GPU ──
// Representatives stay below 2^64 (not canonical) between operations:
// one reduction per product, sums accumulated wide (64-bit low word and a
// small high count) and reduced once, the partial rounds' next S-box
// input carried as x' = (d0+1)·y + (Σ_{i≥1} s_i + c), K independent
// states interleaved round by round (instruction-level parallelism only:
// x⁷ shares nothing across states). Field-equal to `permute_ref`; outputs
// canonical.

static inline ulong rwide(ulong lo, ulong hi) {
    ulong hh = hi >> 32;
    ulong hl = hi & EPS;
    ulong t0 = lo - hh;
    if (lo < hh) t0 -= EPS;
    ulong t1 = (hl << 32) - hl;
    ulong r = t0 + t1;
    if (r < t1) r += EPS;
    return r;
}
static inline void prod(ulong a, ulong b, thread ulong& lo, thread ulong& hi) {
    uint a0 = (uint)a, a1 = (uint)(a >> 32), b0 = (uint)b, b1 = (uint)(b >> 32);
    ulong p00 = (ulong)a0 * b0;
    ulong p01 = (ulong)a0 * b1;
    ulong p10 = (ulong)a1 * b0;
    ulong p11 = (ulong)a1 * b1;
    ulong mid = p01 + p10;
    ulong mc = mid < p01 ? (1UL << 32) : 0;
    lo = p00 + (mid << 32);
    hi = p11 + (mid >> 32) + mc + (lo < p00 ? 1 : 0);
}
static inline ulong fmul(ulong a, ulong b) { ulong lo, hi; prod(a, b, lo, hi); return rwide(lo, hi); }
static inline ulong fsqr(ulong a) {
    uint a0 = (uint)a, a1 = (uint)(a >> 32);
    ulong p00 = (ulong)a0 * a0;
    ulong p01 = (ulong)a0 * a1;
    ulong p11 = (ulong)a1 * a1;
    ulong lo = p00 + (p01 << 33);
    ulong hi = p11 + (p01 >> 31) + (lo < p00 ? 1 : 0);
    return rwide(lo, hi);
}
static inline ulong fsqn(ulong x, int n) { for (int i = 0; i < n; i++) x = fsqr(x); return x; }
static inline ulong fmuladd(ulong a, ulong b, ulong c) {
    ulong lo, hi;
    prod(a, b, lo, hi);
    ulong l2 = lo + c;
    hi += (l2 < lo) ? 1 : 0;
    return rwide(l2, hi);
}
// any representatives
static inline ulong fadd(ulong a, ulong b) {
    ulong s = a + b;
    ulong s2 = s + ((s < a) ? EPS : 0);
    return s2 + ((s2 < s) ? EPS : 0);
}
// c < p
static inline ulong faddc(ulong a, ulong c) {
    ulong s = a + c;
    return s + ((s < a) ? EPS : 0);
}
static inline ulong fcanon(ulong x) { return x >= P ? x - P : x; }
// a wide sum: lo + hi·2^64, hi ≤ 2^32 − 1
struct w2 { ulong l; uint h; };
static inline w2 wadd(w2 a, w2 b) { ulong l = a.l + b.l; return w2{l, a.h + b.h + (l < a.l ? 1u : 0u)}; }
static inline w2 wof(ulong x) { return w2{x, 0u}; }
static inline ulong wred(w2 a) { ulong h = a.h; return faddc(a.l, (h << 32) - h); }

static inline ulong fpow7(ulong x) {
    ulong x2 = fsqr(x);
    ulong x3 = fmul(x2, x);
    ulong x4 = fsqr(x2);
    return fmul(x3, x4);
}
static inline w2 wdbl(w2 a) { return wadd(a, a); }
// M_E with wide sums: a block output is at most 16 inputs, a lane of M_E at
// most 80 (< 2^7), so every sum fits w2 and is reduced once
static inline void fexternal(thread ulong* s) {
    w2 y[16];
    for (int c = 0; c < 4; c++) {
        thread ulong* x = s + 4 * c;
        w2 t0 = wadd(wof(x[0]), wof(x[1]));
        w2 t1 = wadd(wof(x[2]), wof(x[3]));
        w2 t2 = wadd(wdbl(wof(x[1])), t1);
        w2 t3 = wadd(wdbl(wof(x[3])), t0);
        w2 t4 = wadd(wdbl(wdbl(t1)), t3);
        w2 t5 = wadd(wdbl(wdbl(t0)), t2);
        y[4 * c] = wadd(t3, t5);
        y[4 * c + 1] = t5;
        y[4 * c + 2] = wadd(t2, t4);
        y[4 * c + 3] = t4;
    }
    for (int k = 0; k < 4; k++) {
        w2 sum = wadd(wadd(y[k], y[4 + k]), wadd(y[8 + k], y[12 + k]));
        for (int c = 0; c < 4; c++) s[4 * c + k] = wred(wadd(y[4 * c + k], sum));
    }
}
static inline void ffull(thread ulong* s, int r) {
    for (int i = 0; i < 16; i++) s[i] = fpow7(faddc(s[i], RC[r * 16 + i]));
    fexternal(s);
}
static inline ulong frest(thread const ulong* s) {
    w2 a = wof(s[1]);
    for (int i = 2; i < 16; i++) a = wadd(a, wof(s[i]));
    return wred(a);
}

template <int KK>
static inline void permute_k(thread ulong (*s)[16]) {
    for (int k = 0; k < KK; k++) {
        fexternal(s[k]);
        for (int r = 0; r < 6; r++) ffull(s[k], r);
    }
    // x: the S-box input of lane 0; rest: Σ_{i≥1} s_i
    ulong x[KK], rest[KK];
    for (int k = 0; k < KK; k++) {
        x[k] = faddc(s[k][0], RC[192]);
        rest[k] = frest(s[k]);
    }
    for (int r = 0; r < 48; r++) {
        for (int k = 0; k < KK; k++) {
            ulong y = fpow7(x[k]);
            ulong sum = fadd(y, rest[k]);
            for (int i = 1; i < 16; i++) s[k][i] = fmuladd(s[k][i], DIAG[i], sum);
            // s_0' = d_0·y + sum = (d_0 + 1)·y + rest
            if (r + 1 < 48) {
                x[k] = fmuladd(y, D0P1, faddc(rest[k], RC[192 + r + 1]));
                rest[k] = frest(s[k]);
            } else {
                s[k][0] = fmuladd(y, D0P1, rest[k]);
            }
        }
    }
    for (int k = 0; k < KK; k++) {
        for (int r = 6; r < 12; r++) ffull(s[k], r);
        for (int i = 0; i < 16; i++) s[k][i] = fcanon(s[k][i]);
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

// ── header mining: the flat sponge over 168 bytes, nonce at bytes 24..40 ──
// (`mine.rs`: four permutations, blocks added lane-wise, the length lane set
// before the last one; a digest passes when its four limbs read as a
// little-endian 256-bit integer below the target)

struct MineParams {
    ulong init[16];
    ulong blocks[32];    // four rate blocks of eight limbs, nonce bits zero
    ulong target[4];
    ulong base_lo;       // first nonce of the dispatch, low word
    ulong base_hi;       // high word, constant over the dispatch
    ulong tail;          // written to lane tail_lane before the last permutation
    uint tail_lane;
    uint count;          // nonces in the dispatch
};

static inline bool below256(thread const ulong* d, constant const ulong* t) {
    for (int i = 3; i >= 0; i--) {
        if (d[i] != t[i]) return d[i] < t[i];
    }
    return false;
}

// thread t tries nonces base + K·t + k
kernel void hemera_mine(constant MineParams& p [[buffer(0)]],
                        device atomic_uint* found [[buffer(1)]],
                        uint gid [[thread_position_in_grid]]) {
    uint first = gid * K;
    if (first >= p.count) return;
    ulong s[K][16];
    for (int k = 0; k < K; k++) {
        ulong lo = p.base_lo + first + k;
        ulong hi = p.base_hi;
        ulong a3 = (lo & 0xffffffffUL) << 24;
        ulong a4 = ((lo >> 32) | (hi << 32)) & 0x00ffffffffffffffUL;
        ulong a5 = (hi >> 24) & 0xffffffffffUL;
        for (int i = 0; i < 16; i++) {
            ulong v = p.init[i];
            if (i < 8) v = gadd(v, p.blocks[i]);
            if (i == 3) v = gadd(v, a3);
            if (i == 4) v = gadd(v, a4);
            if (i == 5) v = gadd(v, a5);
            s[k][i] = v;
        }
    }
    permute_k<K>(s);
    for (int b = 1; b < 4; b++) {
        for (int k = 0; k < K; k++) {
            for (int i = 0; i < 8; i++) s[k][i] = gadd(s[k][i], p.blocks[8 * b + i]);
            if (b == 3) s[k][p.tail_lane] = p.tail;
        }
        permute_k<K>(s);
    }
    for (int k = 0; k < K; k++) {
        if (first + k < p.count && below256(s[k], p.target))
            atomic_fetch_min_explicit(found, first + k, memory_order_relaxed);
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
    // K items a thread, item gid + k·T (T threads)
    uint T = (p.items + K - 1) / K;
    if (gid >= T) return;
    ulong s[K][16];
    for (int k = 0; k < K; k++) for (int i = 0; i < 16; i++) s[k][i] = p.init[i];
    for (uint b = 0; b < p.blocks; b++) {
        for (int k = 0; k < K; k++) {
            uint it = min(gid + (uint)k * T, p.items - 1);
            device const ulong* src = lanes + (ulong)it * p.rate * p.blocks;
            for (uint i = 0; i < 16; i++) {
                if (i < p.rate) s[k][i] = src[b * p.rate + i];
            }
        }
        permute_k<K>(s);
    }
    for (int k = 0; k < K; k++) {
        uint it = gid + (uint)k * T;
        if (it < p.items) for (int i = 0; i < 4; i++) out[4 * (ulong)it + i] = s[k][i];
    }
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
    // K leaves a thread, leaf gid + k·T (T threads)
    uint n = p.g * p.per;
    uint T = (n + K - 1) / K;
    if (gid >= T) return;
    ulong s[K][16];
    for (int k = 0; k < K; k++) for (int i = 0; i < 16; i++) s[k][i] = p.init[i];
    uint total = p.members * p.w;
    for (uint blk = 0; blk < p.blocks; blk++) {
        for (int k = 0; k < K; k++) {
            uint it = min(gid + (uint)k * T, n - 1);
            uint c = it / p.per;
            uint a = it % p.per;
            uint lane = 0;
            for (uint j = 0; j < p.spb; j++) {
                uint t = blk * p.spb + j;
                if (t >= total) break;
                uint m = t / p.w, tt = t % p.w;
                ulong u = a + (ulong)tt * p.per;
                for (uint l = 0; l < p.limbs; l++) {
                    s[k][lane + l] = vals[(((ulong)(m * p.limbs + l)) * p.g + c) * p.k + u];
                }
                lane += p.limbs;
            }
            for (uint i = lane; i < p.rate; i++) s[k][i] = 0;
        }
        permute_k<K>(s);
    }
    for (int k = 0; k < K; k++) {
        uint it = gid + (uint)k * T;
        if (it < n) for (int i = 0; i < 4; i++) out[4 * (ulong)it + i] = s[k][i];
    }
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

// ── Merkle levels: a level's nodes from the previous level's digests ──
struct LevelParams {
    uint count;   // nodes of this level
    uint arity;   // 2: (l ‖ r ‖ tag at lane 8), 4: four children, all lanes
    ulong tag;
};

kernel void merkle_level(device const ulong* src [[buffer(0)]],
                         device ulong* dst [[buffer(1)]],
                         constant LevelParams& p [[buffer(2)]],
                         uint gid [[thread_position_in_grid]]) {
    // K nodes a thread, node gid + k·T (T threads)
    uint T = (p.count + K - 1) / K;
    if (gid >= T) return;
    ulong s[K][16];
    for (int k = 0; k < K; k++) {
        uint it = min(gid + (uint)k * T, p.count - 1);
        for (int i = 0; i < 16; i++) s[k][i] = 0;
        device const ulong* c = src + 4 * (ulong)p.arity * it;
        for (uint i = 0; i < 4 * p.arity; i++) s[k][i] = c[i];
        if (p.arity == 2) s[k][8] = p.tag;
    }
    permute_k<K>(s);
    for (int k = 0; k < K; k++) {
        uint it = gid + (uint)k * T;
        if (it < p.count) for (int i = 0; i < 4; i++) dst[4 * (ulong)it + i] = s[k][i];
    }
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

// the first `stages` radix-2 stages (spans 1 … 2^(stages−1)) of every
// block of 2^stages elements in threadgroup memory: one dispatch, one
// threadgroup of 2^(stages−1) threads a block
kernel void ntt_block(device ulong* data [[buffer(0)]],
                      device const ulong* tw [[buffer(1)]],
                      constant NttParams& p [[buffer(2)]],
                      uint tid [[thread_index_in_threadgroup]],
                      uint blk [[threadgroup_position_in_grid]]) {
    threadgroup ulong a[2048];
    uint size = 1u << p.span;          // block length (span field reused: log of it)
    uint halfb = size >> 1;
    device ulong* d = data + (ulong)blk * size;
    a[tid] = d[tid];
    a[tid + halfb] = d[tid + halfb];
    for (uint st = 0; st < p.span; st++) {
        threadgroup_barrier(mem_flags::mem_threadgroup);
        uint h = 1u << st;
        uint i = tid & (h - 1);
        uint j = ((tid - i) << 1) + i;
        ulong t = gmul(tw[h + i], a[j + h]);
        ulong u = a[j];
        a[j + h] = gsub(u, t);
        a[j] = gadd(u, t);
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);
    d[tid] = a[tid];
    d[tid + halfb] = a[tid + halfb];
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
    write!(s, "constant ulong RC[240] = {{").expect("fmt");
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
    let d0p1 = (u128::from(consts::diag()[0]) + 1) % 0xFFFF_FFFF_0000_0001u128;
    writeln!(s, "constant ulong D0P1 = 0x{d0p1:016x}UL;").expect("fmt");
    s.push_str(FIELD);
    s.push_str(crate::msl_tree::BYTE_TREE);
    s
}

/// `source` with a diagnostic nonce: every kernel gains an unused
/// `[[thread_index_in_simdgroup]]` argument named after the nonce, so both
/// the OS front-end cache (source text) and back-end cache (function
/// bitcode) have never seen it. Cold-start measurements only; outputs are
/// unchanged (the argument is never read).
pub fn with_nonce(source: &str, nonce: &str) -> String {
    let clean: String = nonce
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let arg = format!("uint lens_nonce_{clean} [[thread_index_in_simdgroup]], ");
    let mut out = String::with_capacity(source.len() + 64 * 16);
    let mut rest = source;
    while let Some(i) = rest.find("kernel void ") {
        let open = i + rest[i..].find('(').expect("kernel signature") + 1;
        out.push_str(&rest[..open]);
        out.push_str(&arg);
        rest = &rest[open..];
    }
    out.push_str(rest);
    out
}
