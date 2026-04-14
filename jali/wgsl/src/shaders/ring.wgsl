// ---------------------------------------------------------------------------
// ring.wgsl — Ring-level operations for R_q = F_p[x]/(x^n+1) on GPU
// ---------------------------------------------------------------------------
//
// DEPENDS ON: nebu field.wgsl (Fp struct, fp_add, fp_sub, fp_mul, fp_neg)
//             nebu ntt.wgsl (butterfly operations)
// Prepend both nebu shaders before this file at load time.
//
// This shader provides the coefficient-wise and ring-level operations needed
// for polynomial arithmetic in the ring R_q over the Goldilocks field.
//
// === Memory layout ===
//
// A ring element (polynomial of degree < n) is stored as n consecutive Fp
// structs in a storage buffer. Each Fp is 8 bytes (lo: u32, hi: u32),
// so a polynomial of degree n occupies 8n bytes.
//
// For binary operations (add, sub, pointwise_mul), the two input polynomials
// are stored in `lhs` and `rhs`, and the result is written to `out`.
// For unary operations (neg, twist, untwist, scalar_mul), the input is in
// `lhs` and the result is written to `out`.
//
// === Entry points ===
//
// Each operation is a separate @compute entry point with @workgroup_size(256).
// The host dispatches ceil(n / 256) workgroups, and each thread processes
// one coefficient. This gives perfect parallelism for all ring sizes up to
// the GPU's thread capacity.
//
// === Negacyclic NTT twist ===
//
// For the negacyclic NTT (NTT modulo x^n + 1), we need to pre-multiply
// (twist) by powers of a primitive 2n-th root of unity psi:
//
//   twisted[i] = coeffs[i] * psi^i
//
// After the inverse NTT, we post-multiply (untwist) by psi^(-i):
//
//   coeffs[i] = ntt_result[i] * psi^(-i)
//
// The twist/untwist tables are precomputed on the host (psi^0, psi^1, ...,
// psi^(n-1)) and stored in the `rhs` buffer.
//
// ---------------------------------------------------------------------------

// Include Goldilocks field arithmetic (Fp struct, fp_add, fp_sub, fp_mul, etc.)
// In the host pipeline, goldilocks.wgsl is prepended to this source before
// shader compilation:
//   let source = format!("{}\n{}", GOLDILOCKS, RING);

// ── Bindings ──────────────────────────────────────────────────────────────

// Left operand / input polynomial.
@group(0) @binding(0) var<storage, read> lhs: array<Fp>;

// Right operand / twist table / scalar (depending on the operation).
@group(0) @binding(1) var<storage, read> rhs: array<Fp>;

// Output polynomial.
@group(0) @binding(2) var<storage, read_write> out: array<Fp>;

// Ring parameters.
struct RingParams {
    n: u32,           // polynomial degree
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}
@group(0) @binding(3) var<uniform> ring_params: RingParams;

// ── Coefficient-wise addition ─────────────────────────────────────────────
//
// out[i] = lhs[i] + rhs[i]  for i in [0, n)
//
// Works in both coefficient and NTT domain (the operation is the same).

@compute @workgroup_size(256)
fn ring_add(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= ring_params.n) {
        return;
    }
    out[i] = fp_add(lhs[i], rhs[i]);
}

// ── Coefficient-wise subtraction ──────────────────────────────────────────
//
// out[i] = lhs[i] - rhs[i]  for i in [0, n)

@compute @workgroup_size(256)
fn ring_sub(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= ring_params.n) {
        return;
    }
    out[i] = fp_sub(lhs[i], rhs[i]);
}

// ── Coefficient-wise negation ─────────────────────────────────────────────
//
// out[i] = -lhs[i]  for i in [0, n)
// The rhs buffer is unused.

@compute @workgroup_size(256)
fn ring_neg(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= ring_params.n) {
        return;
    }
    out[i] = fp_neg(lhs[i]);
}

// ── Scalar multiplication ─────────────────────────────────────────────────
//
// out[i] = lhs[i] * scalar  for i in [0, n)
// The scalar is passed as rhs[0] (the first element of the rhs buffer).
// All n threads read the same scalar value.

@compute @workgroup_size(256)
fn ring_scalar_mul(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= ring_params.n) {
        return;
    }
    let scalar = rhs[0];
    out[i] = fp_mul(lhs[i], scalar);
}

// ── Pointwise multiplication (NTT domain) ─────────────────────────────────
//
// out[i] = lhs[i] * rhs[i]  for i in [0, n)
//
// This is the core of ring multiplication: after both polynomials are
// transformed to NTT domain, their product in R_q is just n independent
// field multiplications — perfectly parallel on GPU.

@compute @workgroup_size(256)
fn ring_pointwise_mul(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= ring_params.n) {
        return;
    }
    out[i] = fp_mul(lhs[i], rhs[i]);
}

// ── Negacyclic twist (pre-NTT) ───────────────────────────────────────────
//
// out[i] = lhs[i] * psi_table[i]  for i in [0, n)
//
// where psi_table[i] = psi^i, with psi a primitive 2n-th root of unity.
// The psi_table is passed in the rhs buffer.
//
// This converts the negacyclic convolution (mod x^n + 1) into a standard
// cyclic convolution (mod x^n - 1) that the vanilla NTT can handle:
//
//   f(x) mod (x^n + 1)  <-->  f(psi*x) mod (x^n - 1)
//
// After twisting, a standard forward NTT is applied.

@compute @workgroup_size(256)
fn ring_twist(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= ring_params.n) {
        return;
    }
    // rhs[i] holds psi^i (precomputed by host).
    out[i] = fp_mul(lhs[i], rhs[i]);
}

// ── Negacyclic untwist (post-INTT) ───────────────────────────────────────
//
// out[i] = lhs[i] * psi_inv_table[i]  for i in [0, n)
//
// where psi_inv_table[i] = psi^(-i). The psi_inv_table is passed in the
// rhs buffer.
//
// After the inverse NTT recovers the (twisted) coefficients, this step
// removes the twist to yield the true polynomial product modulo x^n + 1.

@compute @workgroup_size(256)
fn ring_untwist(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= ring_params.n) {
        return;
    }
    // rhs[i] holds psi^(-i) (precomputed by host).
    out[i] = fp_mul(lhs[i], rhs[i]);
}
