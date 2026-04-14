// ── nebu/ntt ─────────────────────────────────────────────────────
//
// NTT butterfly and bit-reversal for GPU compute shaders.
// Pure functions for butterfly operations and twiddle computation.
// Entry-point shaders are in ntt_kernels.wgsl.
// Requires: field.wgsl

// Primitive root g = 7
const G_LO: u32 = 7u;
const G_HI: u32 = 0u;

// ── Butterfly ────────────────────────────────────────────────────
// (a, b, ω) → (a + ω·b, a − ω·b)
// Returns (out_a_lo, out_a_hi, out_b_lo, out_b_hi) packed as vec4.

fn ntt_butterfly(
    a_lo: u32, a_hi: u32,
    b_lo: u32, b_hi: u32,
    w_lo: u32, w_hi: u32
) -> vec4<u32> {
    let t = gl_mul(w_lo, w_hi, b_lo, b_hi);
    let out_a = gl_add(a_lo, a_hi, t.x, t.y);
    let out_b = gl_sub(a_lo, a_hi, t.x, t.y);
    return vec4<u32>(out_a.x, out_a.y, out_b.x, out_b.y);
}

// Inverse butterfly for Gentleman-Sande DIF:
// (u, v, ω_inv) → (u + v, ω_inv · (u − v))
fn intt_butterfly(
    u_lo: u32, u_hi: u32,
    v_lo: u32, v_hi: u32,
    w_inv_lo: u32, w_inv_hi: u32
) -> vec4<u32> {
    let out_a = gl_add(u_lo, u_hi, v_lo, v_hi);
    let diff = gl_sub(u_lo, u_hi, v_lo, v_hi);
    let out_b = gl_mul(w_inv_lo, w_inv_hi, diff.x, diff.y);
    return vec4<u32>(out_a.x, out_a.y, out_b.x, out_b.y);
}

// ── Bit reversal ─────────────────────────────────────────────────

fn bit_reverse(i: u32, k: u32) -> u32 {
    var result = 0u;
    for (var b = 0u; b < k; b++) {
        result |= ((i >> b) & 1u) << (k - 1u - b);
    }
    return result;
}

// ── Twiddle factor computation ───────────────────────────────────
// ω_m = g^((p-1) / m) where m = 2^(s+1).
// (p-1) / m = (p-1) >> (s+1).
// p-1 = 0xFFFFFFFF00000000 = (lo=0x00000000, hi=0xFFFFFFFF).

fn compute_twiddle(stage: u32) -> vec2<u32> {
    let shift = stage + 1u;
    var exp_lo = 0u;
    var exp_hi = 0u;
    if shift < 32u {
        exp_lo = 0xFFFFFFFFu << (32u - shift);
        exp_hi = 0xFFFFFFFFu >> shift;
    } else if shift == 32u {
        exp_lo = 0xFFFFFFFFu;
        exp_hi = 0u;
    } else if shift < 64u {
        exp_lo = 0xFFFFFFFFu >> (shift - 32u);
        exp_hi = 0u;
    } else {
        exp_lo = 0u;
        exp_hi = 0u;
    }
    return gl_exp(G_LO, G_HI, exp_lo, exp_hi);
}

// Compute ω_m^idx by repeated multiplication.
fn compute_twiddle_power(stage: u32, idx: u32) -> vec2<u32> {
    let omega_m = compute_twiddle(stage);
    var w = vec2<u32>(1u, 0u);
    for (var i = 0u; i < idx; i++) {
        w = gl_mul(w.x, w.y, omega_m.x, omega_m.y);
    }
    return w;
}

// Compute inverse twiddle ω_m_inv^idx.
fn compute_inv_twiddle_power(stage: u32, m: u32, idx: u32) -> vec2<u32> {
    let omega_m = compute_twiddle(stage);
    let omega_m_inv = gl_exp(omega_m.x, omega_m.y, m - 1u, 0u);
    var w = vec2<u32>(1u, 0u);
    for (var i = 0u; i < idx; i++) {
        w = gl_mul(w.x, w.y, omega_m_inv.x, omega_m_inv.y);
    }
    return w;
}

// Compute N^(-1) mod p for power-of-2 N.
fn compute_n_inv(n: u32) -> vec2<u32> {
    let k = firstTrailingBit(n);
    var div_lo = 0u;
    var div_hi = 0u;
    if k < 32u {
        div_lo = 0xFFFFFFFFu << (32u - k);
        div_hi = 0xFFFFFFFFu >> k;
    } else if k == 32u {
        div_lo = 0xFFFFFFFFu;
        div_hi = 0u;
    }
    return sub64(P_LO, P_HI, div_lo, div_hi);
}
