// ============================================================================
// tower.wgsl — F₂ tower field arithmetic for GPU compute
// ============================================================================
//
// Tower construction:
//   F₂ → F₂² → F₂⁴ → F₂⁸ → F₂¹⁶ → F₂³² → F₂⁶⁴ → F₂¹²⁸
//
// Each level F_{2^{2k}} is built as F_{2^k}[x] / (x² + x + α_k) where α_k
// is the canonical generator of the previous level. Concretely α_k = 0b10
// in the representation of F_{2^k} — the element whose only set bit is
// position 1 (the "x" coefficient at the previous level).
//
// Representation:
//   F₂  through F₂³²  — single u32  (only the low n bits are significant)
//   F₂⁶⁴              — array<u32, 2>  (lo, hi)
//   F₂¹²⁸             — array<u32, 4>  (w0, w1, w2, w3) little-endian
//
// Addition at every level is XOR — this is the defining property of
// characteristic-2 fields.
//
// Multiplication uses the "schoolbook" tower decomposition (equivalent to
// Karatsuba when only 3 sub-multiplications are needed):
//   Given a = a_lo + a_hi · x  and  b = b_lo + b_hi · x
//     ll    = a_lo * b_lo                     (sub-mul 1)
//     hh    = a_hi * b_hi                     (sub-mul 2)
//     cross = a_lo * b_hi  +  a_hi * b_lo     (sub-mul 3, via Karatsuba trick
//             or two sub-muls; we use the direct form for clarity)
//   Then:
//     result_lo = ll  +  hh * α               (reduce x² = x + α)
//     result_hi = cross + hh
//
// The Karatsuba trick replaces the two cross multiplications with:
//     cross = (a_lo + a_hi) * (b_lo + b_hi) + ll + hh
// which still costs 3 sub-multiplications total. Both forms give identical
// results in characteristic 2 (where + is XOR). We use the Karatsuba form
// at every level for consistency.
//
// WGSL constraints:
//   - No u64 or u128 types; we emulate with arrays of u32
//   - No recursion; each tower level is a separate function
//   - Workgroup size tuned for typical GPU architectures (256 threads)
// ============================================================================


// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

// F₂⁶⁴ — two 32-bit words, little-endian
alias F2_64 = array<u32, 2>;

// F₂¹²⁸ — four 32-bit words, little-endian
alias F2_128 = array<u32, 4>;

// Packed128 — 128 F₂ elements packed in 4×u32 (bitwise operations)
alias Packed128 = array<u32, 4>;


// ============================================================================
// F₂ (1-bit) — base field
// ============================================================================
// add(a, b) = a ^ b   (XOR)
// mul(a, b) = a & b   (AND)
// Both a and b use only bit 0 of u32.

fn f2_add(a: u32, b: u32) -> u32 {
    return (a ^ b) & 1u;
}

fn f2_mul(a: u32, b: u32) -> u32 {
    return (a & b) & 1u;
}


// ============================================================================
// F₂² (2-bit) — first extension
// ============================================================================
// Irreducible: x² + x + 1  over F₂.
// Element: a = a₀ + a₁·x stored in bits [0:1] of a u32.
//
// Multiplication:
//   (a₀ + a₁x)(b₀ + b₁x) mod (x² + x + 1)
//   Karatsuba:
//     ll    = a₀ · b₀
//     hh    = a₁ · b₁
//     cross = (a₀ ^ a₁) · (b₀ ^ b₁) ^ ll ^ hh
//   Result:
//     c₀ = ll ^ hh          (since α = 1 for this level, hh·α = hh)
//     c₁ = cross ^ hh       (= cross + hh in char 2)
//
// Note: at the F₂² level, α = 1 (the generator of F₂ is just 1).
// The irreducible is x² + x + 1, so x² = x + 1, meaning
// hh·x² = hh·x + hh·1. We collect: c₀ = ll + hh, c₁ = cross + hh.

fn f2_2_add(a: u32, b: u32) -> u32 {
    return (a ^ b) & 3u;
}

fn f2_2_mul(a: u32, b: u32) -> u32 {
    let a0 = a & 1u;
    let a1 = (a >> 1u) & 1u;
    let b0 = b & 1u;
    let b1 = (b >> 1u) & 1u;

    // Karatsuba: 3 base-field multiplications
    let ll = a0 & b0;
    let hh = a1 & b1;
    let cross = ((a0 ^ a1) & (b0 ^ b1)) ^ ll ^ hh;

    // x² = x + 1, so hh·x² contributes hh to both c₀ and c₁
    let c0 = ll ^ hh;
    let c1 = cross ^ hh;

    return c0 | (c1 << 1u);
}


// ============================================================================
// F₂⁴ (4-bit) — second extension
// ============================================================================
// Irreducible: x² + x + α  over F₂²  where α = 0b10 (the element "x" in F₂²).
// Element: a = a_lo + a_hi·x stored in bits [0:3] of a u32.
//   a_lo = bits [0:1],  a_hi = bits [2:3]
//
// Karatsuba decomposition:
//   ll    = f2_2_mul(a_lo, b_lo)
//   hh    = f2_2_mul(a_hi, b_hi)
//   cross = f2_2_mul(a_lo ^ a_hi, b_lo ^ b_hi) ^ ll ^ hh
//   c_lo  = ll ^ f2_2_mul(hh, alpha)       alpha = 0b10
//   c_hi  = cross ^ hh

fn f2_4_add(a: u32, b: u32) -> u32 {
    return (a ^ b) & 0xFu;
}

fn f2_4_mul(a: u32, b: u32) -> u32 {
    let a_lo = a & 3u;
    let a_hi = (a >> 2u) & 3u;
    let b_lo = b & 3u;
    let b_hi = (b >> 2u) & 3u;

    let alpha = 2u; // 0b10 — generator of F₂²

    let ll = f2_2_mul(a_lo, b_lo);
    let hh = f2_2_mul(a_hi, b_hi);
    let cross = f2_2_mul(a_lo ^ a_hi, b_lo ^ b_hi) ^ ll ^ hh;

    let c_lo = ll ^ f2_2_mul(hh, alpha);
    let c_hi = cross ^ hh;

    return c_lo | (c_hi << 2u);
}


// ============================================================================
// F₂⁸ (8-bit) — third extension
// ============================================================================
// Irreducible: x² + x + α  over F₂⁴  where α = 0b0010.
// Element stored in bits [0:7] of a u32.
//   a_lo = bits [0:3],  a_hi = bits [4:7]

fn f2_8_add(a: u32, b: u32) -> u32 {
    return (a ^ b) & 0xFFu;
}

fn f2_8_mul(a: u32, b: u32) -> u32 {
    let a_lo = a & 0xFu;
    let a_hi = (a >> 4u) & 0xFu;
    let b_lo = b & 0xFu;
    let b_hi = (b >> 4u) & 0xFu;

    let alpha = 2u; // 0b0010 — generator of F₂⁴

    let ll = f2_4_mul(a_lo, b_lo);
    let hh = f2_4_mul(a_hi, b_hi);
    let cross = f2_4_mul(a_lo ^ a_hi, b_lo ^ b_hi) ^ ll ^ hh;

    let c_lo = ll ^ f2_4_mul(hh, alpha);
    let c_hi = cross ^ hh;

    return c_lo | (c_hi << 4u);
}


// ============================================================================
// F₂¹⁶ (16-bit) — fourth extension
// ============================================================================
// Irreducible: x² + x + α  over F₂⁸  where α = 0x02.
// Element stored in bits [0:15] of a u32.
//   a_lo = bits [0:7],  a_hi = bits [8:15]

fn f2_16_add(a: u32, b: u32) -> u32 {
    return (a ^ b) & 0xFFFFu;
}

fn f2_16_mul(a: u32, b: u32) -> u32 {
    let a_lo = a & 0xFFu;
    let a_hi = (a >> 8u) & 0xFFu;
    let b_lo = b & 0xFFu;
    let b_hi = (b >> 8u) & 0xFFu;

    let alpha = 2u; // 0x02 — generator of F₂⁸

    let ll = f2_8_mul(a_lo, b_lo);
    let hh = f2_8_mul(a_hi, b_hi);
    let cross = f2_8_mul(a_lo ^ a_hi, b_lo ^ b_hi) ^ ll ^ hh;

    let c_lo = ll ^ f2_8_mul(hh, alpha);
    let c_hi = cross ^ hh;

    return c_lo | (c_hi << 8u);
}


// ============================================================================
// F₂³² (32-bit) — fifth extension
// ============================================================================
// Irreducible: x² + x + α  over F₂¹⁶  where α = 0x0002.
// Element stored in all 32 bits of a u32.
//   a_lo = bits [0:15],  a_hi = bits [16:31]

fn f2_32_add(a: u32, b: u32) -> u32 {
    return a ^ b;
}

fn f2_32_mul(a: u32, b: u32) -> u32 {
    let a_lo = a & 0xFFFFu;
    let a_hi = (a >> 16u) & 0xFFFFu;
    let b_lo = b & 0xFFFFu;
    let b_hi = (b >> 16u) & 0xFFFFu;

    let alpha = 2u; // 0x0002 — generator of F₂¹⁶

    let ll = f2_16_mul(a_lo, b_lo);
    let hh = f2_16_mul(a_hi, b_hi);
    let cross = f2_16_mul(a_lo ^ a_hi, b_lo ^ b_hi) ^ ll ^ hh;

    let c_lo = ll ^ f2_16_mul(hh, alpha);
    let c_hi = cross ^ hh;

    return c_lo | (c_hi << 16u);
}


// ============================================================================
// F₂⁶⁴ (64-bit) — sixth extension
// ============================================================================
// Irreducible: x² + x + α  over F₂³²  where α = 0x00000002.
// Element stored as array<u32, 2> = (lo_word, hi_word) little-endian.
//   a_lo = word 0 (an F₂³² element)
//   a_hi = word 1 (an F₂³² element)

fn f2_64_add(a: F2_64, b: F2_64) -> F2_64 {
    return F2_64(a[0] ^ b[0], a[1] ^ b[1]);
}

fn f2_64_mul(a: F2_64, b: F2_64) -> F2_64 {
    let a_lo = a[0];
    let a_hi = a[1];
    let b_lo = b[0];
    let b_hi = b[1];

    let alpha = 2u; // 0x00000002 — generator of F₂³²

    // Karatsuba: 3 sub-multiplications in F₂³²
    let ll = f2_32_mul(a_lo, b_lo);
    let hh = f2_32_mul(a_hi, b_hi);
    let cross = f2_32_mul(a_lo ^ a_hi, b_lo ^ b_hi) ^ ll ^ hh;

    // Reduce: x² = x + α  =>  hh·x² = hh·x + hh·α
    let c_lo = ll ^ f2_32_mul(hh, alpha);
    let c_hi = cross ^ hh;

    return F2_64(c_lo, c_hi);
}


// ============================================================================
// F₂¹²⁸ (128-bit) — seventh extension, top of the tower
// ============================================================================
// Irreducible: x² + x + α  over F₂⁶⁴  where α = (0x00000002, 0x00000000).
// Element stored as array<u32, 4> = (w0, w1, w2, w3) little-endian.
//   a_lo = F₂⁶⁴(w0, w1)   — lower 64 bits
//   a_hi = F₂⁶⁴(w2, w3)   — upper 64 bits
//
// This is the primary working type for Binius on the GPU. Each invocation
// of the compute shader operates on one or more F₂¹²⁸ elements.

fn f2_128_add(a: F2_128, b: F2_128) -> F2_128 {
    return F2_128(a[0] ^ b[0], a[1] ^ b[1], a[2] ^ b[2], a[3] ^ b[3]);
}

fn f2_128_mul(a: F2_128, b: F2_128) -> F2_128 {
    // Decompose into F₂⁶⁴ halves (little-endian word order)
    let a_lo = F2_64(a[0], a[1]);
    let a_hi = F2_64(a[2], a[3]);
    let b_lo = F2_64(b[0], b[1]);
    let b_hi = F2_64(b[2], b[3]);

    // α for this level = generator of F₂⁶⁴ = (2, 0)
    let alpha = F2_64(2u, 0u);

    // Karatsuba: 3 sub-multiplications in F₂⁶⁴
    let ll = f2_64_mul(a_lo, b_lo);
    let hh = f2_64_mul(a_hi, b_hi);

    // cross = (a_lo + a_hi) * (b_lo + b_hi) + ll + hh
    let a_sum = f2_64_add(a_lo, a_hi);
    let b_sum = f2_64_add(b_lo, b_hi);
    let mid = f2_64_mul(a_sum, b_sum);
    let cross = f2_64_add(f2_64_add(mid, ll), hh);

    // Reduce: x² = x + α  =>  hh·x² = hh·x + hh·α
    let hh_alpha = f2_64_mul(hh, alpha);
    let c_lo = f2_64_add(ll, hh_alpha);
    let c_hi = f2_64_add(cross, hh);

    return F2_128(c_lo[0], c_lo[1], c_hi[0], c_hi[1]);
}


// ============================================================================
// Packed128 operations — 128 F₂ elements in parallel
// ============================================================================
// These treat F₂¹²⁸ as a flat vector of 128 independent F₂ elements.
// Bitwise operations give SIMD-style parallelism "for free" on the GPU.

/// Vectorized F₂ addition: 128 parallel XORs.
fn packed128_add(a: Packed128, b: Packed128) -> Packed128 {
    return Packed128(a[0] ^ b[0], a[1] ^ b[1], a[2] ^ b[2], a[3] ^ b[3]);
}

/// Vectorized F₂ multiplication: 128 parallel ANDs.
fn packed128_mul(a: Packed128, b: Packed128) -> Packed128 {
    return Packed128(a[0] & b[0], a[1] & b[1], a[2] & b[2], a[3] & b[3]);
}

/// Vectorized NOT: 128 parallel complements.
fn packed128_not(a: Packed128) -> Packed128 {
    return Packed128(~a[0], ~a[1], ~a[2], ~a[3]);
}

/// Popcount of a single u32 — Hamming weight via the classic bit-parallel
/// algorithm (no hardware popcount in WGSL).
fn popcount_u32(x_in: u32) -> u32 {
    var x = x_in;
    x = x - ((x >> 1u) & 0x55555555u);
    x = (x & 0x33333333u) + ((x >> 2u) & 0x33333333u);
    x = (x + (x >> 4u)) & 0x0F0F0F0Fu;
    x = x + (x >> 8u);
    x = x + (x >> 16u);
    return x & 0x3Fu;
}

/// Popcount of 128 bits: number of set bits across all 4 words.
fn packed128_popcount(a: Packed128) -> u32 {
    return popcount_u32(a[0])
         + popcount_u32(a[1])
         + popcount_u32(a[2])
         + popcount_u32(a[3]);
}

/// Inner product of two packed F₂ vectors: popcount(a AND b).
/// This is the fundamental kernel for binary matrix-vector multiplication.
fn packed128_inner_product(a: Packed128, b: Packed128) -> u32 {
    return packed128_popcount(packed128_mul(a, b));
}


// ============================================================================
// Compute shader: process arrays of F₂¹²⁸ tower elements
// ============================================================================
//
// The shader reads pairs of F₂¹²⁸ elements from input_a and input_b,
// computes their tower field product, and writes the result to output_c.
//
// Each thread handles one element. The dispatch size should be
// ceil(element_count / 256).
//
// Buffer layout:
//   input_a  — array of F₂¹²⁸ elements (4 × u32 each)
//   input_b  — array of F₂¹²⁸ elements (4 × u32 each)
//   output_c — array of F₂¹²⁸ elements (4 × u32 each), results written here
//   params   — uniform with element_count (number of elements to process)

struct Params {
    element_count: u32,
}

@group(0) @binding(0) var<storage, read>       input_a:  array<u32>;
@group(0) @binding(1) var<storage, read>       input_b:  array<u32>;
@group(0) @binding(2) var<storage, read_write> output_c: array<u32>;
@group(0) @binding(3) var<uniform>             params:   Params;

/// Load an F₂¹²⁸ element from a flat u32 storage array at the given index.
fn load_f2_128(buf: ptr<storage, array<u32>, read>, idx: u32) -> F2_128 {
    let base = idx * 4u;
    return F2_128(
        (*buf)[base],
        (*buf)[base + 1u],
        (*buf)[base + 2u],
        (*buf)[base + 3u],
    );
}

/// Store an F₂¹²⁸ element into a flat u32 storage array at the given index.
fn store_f2_128(buf: ptr<storage, array<u32>, read_write>, idx: u32, val: F2_128) {
    let base = idx * 4u;
    (*buf)[base]      = val[0];
    (*buf)[base + 1u] = val[1];
    (*buf)[base + 2u] = val[2];
    (*buf)[base + 3u] = val[3];
}

@compute @workgroup_size(256)
fn tower_mul_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.element_count {
        return;
    }

    let a = load_f2_128(&input_a, idx);
    let b = load_f2_128(&input_b, idx);
    let c = f2_128_mul(a, b);
    store_f2_128(&output_c, idx, c);
}
