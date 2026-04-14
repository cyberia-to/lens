// ── nebu/ntt_kernels ─────────────────────────────────────────────
//
// Compute shader entry points for NTT operations.
// Concatenate: field.wgsl + ntt.wgsl + ntt_kernels.wgsl
//
// Host dispatches:
//   1. bit_reverse_kernel: N threads
//   2. ntt_stage_kernel:   N/2 threads × k passes (stage = 0..k-1)
//   or for inverse:
//   1. intt_stage_kernel:  N/2 threads × k passes (stage = k-1..0)
//   2. bit_reverse_kernel: N threads
//   3. intt_scale_kernel:  N threads

struct NttParams {
    n: u32,        // NTT length (power of 2)
    k: u32,        // log2(n)
    stage: u32,    // current stage index
    _pad: u32,
}

@group(0) @binding(0)
var<storage, read_write> ntt_data: array<u32>;

@group(0) @binding(1)
var<uniform> ntt_params: NttParams;

// ── Bit-reversal permutation ─────────────────────────────────────

@compute @workgroup_size(256)
fn bit_reverse_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tid = gid.x;
    if tid >= ntt_params.n { return; }

    let j = bit_reverse(tid, ntt_params.k);
    if tid < j {
        let a_lo = ntt_data[tid * 2u];
        let a_hi = ntt_data[tid * 2u + 1u];
        let b_lo = ntt_data[j * 2u];
        let b_hi = ntt_data[j * 2u + 1u];
        ntt_data[tid * 2u] = b_lo;
        ntt_data[tid * 2u + 1u] = b_hi;
        ntt_data[j * 2u] = a_lo;
        ntt_data[j * 2u + 1u] = a_hi;
    }
}

// ── Forward NTT stage (Cooley-Tukey DIT) ─────────────────────────

@compute @workgroup_size(256)
fn ntt_stage_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tid = gid.x;
    let half_n = ntt_params.n >> 1u;
    if tid >= half_n { return; }

    let stage = ntt_params.stage;
    let m = 1u << (stage + 1u);
    let half_m = m >> 1u;
    let group = tid / half_m;
    let idx = tid % half_m;
    let j = group * m;

    let pos_a = j + idx;
    let pos_b = j + idx + half_m;

    let a_lo = ntt_data[pos_a * 2u];
    let a_hi = ntt_data[pos_a * 2u + 1u];
    let b_lo = ntt_data[pos_b * 2u];
    let b_hi = ntt_data[pos_b * 2u + 1u];

    let w = compute_twiddle_power(stage, idx);
    let result = ntt_butterfly(a_lo, a_hi, b_lo, b_hi, w.x, w.y);

    ntt_data[pos_a * 2u] = result.x;
    ntt_data[pos_a * 2u + 1u] = result.y;
    ntt_data[pos_b * 2u] = result.z;
    ntt_data[pos_b * 2u + 1u] = result.w;
}

// ── Inverse NTT stage (Gentleman-Sande DIF) ──────────────────────

@compute @workgroup_size(256)
fn intt_stage_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tid = gid.x;
    let half_n = ntt_params.n >> 1u;
    if tid >= half_n { return; }

    let stage = ntt_params.stage;
    let m = 1u << (stage + 1u);
    let half_m = m >> 1u;
    let group = tid / half_m;
    let idx = tid % half_m;
    let j = group * m;

    let pos_a = j + idx;
    let pos_b = j + idx + half_m;

    let u_lo = ntt_data[pos_a * 2u];
    let u_hi = ntt_data[pos_a * 2u + 1u];
    let v_lo = ntt_data[pos_b * 2u];
    let v_hi = ntt_data[pos_b * 2u + 1u];

    let w = compute_inv_twiddle_power(stage, m, idx);
    let result = intt_butterfly(u_lo, u_hi, v_lo, v_hi, w.x, w.y);

    ntt_data[pos_a * 2u] = result.x;
    ntt_data[pos_a * 2u + 1u] = result.y;
    ntt_data[pos_b * 2u] = result.z;
    ntt_data[pos_b * 2u + 1u] = result.w;
}

// ── N⁻¹ scaling after inverse NTT ───────────────────────────────

@compute @workgroup_size(256)
fn intt_scale_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tid = gid.x;
    if tid >= ntt_params.n { return; }

    let n_inv = compute_n_inv(ntt_params.n);

    let a_lo = ntt_data[tid * 2u];
    let a_hi = ntt_data[tid * 2u + 1u];
    let scaled = gl_mul(a_lo, a_hi, n_inv.x, n_inv.y);
    let c = gl_canon(scaled.x, scaled.y);
    ntt_data[tid * 2u] = c.x;
    ntt_data[tid * 2u + 1u] = c.y;
}
