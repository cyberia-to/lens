---
tags: lens, spec, prover, gpu, backend
crystal-type: entity
crystal-domain: comp
alias: prover backend, lens backend
---
# prover backends

The bulk kernels a prover spends its time in, behind one trait
(`rspcs/src/backend.rs`, `cyber_lens_rspcs::backend::Backend`), so one
proof comes out of a CPU, a Metal GPU or a WebGPU adapter. Implementations:
`Cpu` (rspcs), `Metal` and `WebGpu` (`gpu/`, crate `cyber-lens-gpu`).

## contract

Every operation is a pure function over Goldilocks; inputs and outputs are
canonical `u64` limbs (`< p`). A backend may change how a result is
computed, never which result. Proof bytes are therefore identical on every
backend.

| operation | result |
|---|---|
| `permute(states)` | hemera's permutation (`hemera/rs/src/permutation.rs`, profile `HEMERA`) of every 16-lane state, in place |
| `grind(template, inject, bits)` | the least `n ≥ 0` such that output lane 0 of `permute(inject(template, n))` has its low `bits` bits zero, `1 ≤ bits < 64` |
| `coset_ntt(coeffs, shifts)` | for each shift `s`: `ntt(c_i · s^i)` in nebu's order (`nebu::ntt::ntt`: natural-order evaluations on the subgroup of order `coeffs.len()`) |

`inject(template, n)` adds `(n mod 2^lo_bits) · 2^lo_shift` to lane `lo`
and `⌊n / 2^hi_shift⌋` to lane `hi` (field additions; values below `2^56`):

- `Inject::lane(l)` — the nonce is lane `l` of a field-native duplex block
  (zheng's recursion transcript, `recursion/sponge.rs`);
- `Inject::hash40()` with `hash40_template(seed)` — `hemera::hash(seed ‖
  n_le)` for a 32-byte seed: one rate block of 7-byte limbs, length 40 in
  capacity lane 2 (lens's byte transcript, `rspcs/src/pow.rs`).

The least-nonce rule makes grinding deterministic: a backend searches in
any order but returns the minimum over every nonce below the first hit.

## selection

The process holds one current backend (`backend::current`,
`backend::install`), `Cpu` until a binary installs another; library code
never chooses. `cyber_lens_gpu::{select, install}` resolve `auto`
(Metal, else WebGPU, else CPU), `cpu`, `metal`, `webgpu`; joy exposes it as
`joy prove --backend`.

## call sites

| caller | operation |
|---|---|
| `rspcs::pow::grind` (WHIR, TensorRs grinding) | `grind` with `hash40` |
| `rspcs::rs::encode_base` (RS encoding, parallel path) | `coset_ntt` |
| zheng `recursion::perm::permute_many` (batches ≥ 4096: Merkle leaves, nodes, sponge blocks of committed words) | `permute` |
| zheng `recursion::sponge::grind` (recursion profile grinding) | `grind` with `lane` |

## GPU kernels

- Metal (`gpu/src/msl.rs`): native 64-bit integers, `mulhi` products,
  hemera's constants inlined; one state (or `k`, sharing the partial-round
  inversions) per thread; grinding writes the least passing offset with an
  atomic minimum per dispatch; coset NTT as a scatter (coset powers,
  bit reversal) and one radix-2 stage per dispatch.
- WebGPU (`gpu/src/wgsl.rs`): 32-bit limbs (WGSL has no 64-bit integers),
  the same kernels for permutations and grinding; at most two storage
  buffers per kernel (PowerVR drivers hang above four). The NTT runs on
  the CPU. On wasm32 the crate builds; a synchronous prover cannot block on
  GPU readback in a browser, so `webgpu()` refuses there.

Batches below 4096 states and NTTs below 2^12 stay on the CPU (a dispatch
costs about 0.2 ms).

## verification

`gpu/tests/differential.rs` checks every built backend against `Cpu` on
random states (zero and `p − 1` lanes included), both injections, nonces
past the first dispatch, and coset NTTs; `LENS_GPU_REQUIRE=metal,webgpu`
turns a missing backend into a failure. Measurements: zheng
`audit/prover-profile-2026-10.md`.
