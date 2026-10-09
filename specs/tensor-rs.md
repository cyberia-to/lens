---
tags: lens, spec, pcs
crystal-type: entity
crystal-domain: crypto
alias: TensorRs, Ligero RS lens
---
# TensorRs — Ligero geometry, Reed–Solomon rows

Multilinear PCS over Goldilocks `F` (p = 2^64 − 2^32 + 1) with challenges in
`K = Fp3 = F[t]/(t³ − t − 1)`. Implements `MultilinearPcs`
(`core/src/pcs.rs`), name `lens-tensor-rs-v1`, crate `cyber-lens-rspcs`.
Not zero knowledge.

## parameters

| field | default | meaning |
|---|---|---|
| `log_inv_rate` r | 2 | row code rate ρ = 2^-r, r ≥ 1 |
| `security_target` λ | 128 | bits; sets the column count |
| `pow_bits` b | 16 | grinding before column sampling |
| `row_vars` | auto | k1 = 2^row_vars rows; auto = argmin of the size estimate `48·k2 + 8·t·k1 + 32·t·(log N − log t)`, ties to fewer rows |

Header (6 B, in every proof, compared byte-for-byte with the verifier's
params): `01 ‖ r ‖ b ‖ λ (u16 LE) ‖ row_vars (FF = auto)`.

Derived at `m` variables: `k2 = 2^{m−row_vars}`, `N = k2·2^r ≤ 2^32`,
`t = max(1, ⌈(λ − b)/q⌉)` with `q = −log2((1 + ρ)/2)`. If `t ≥ N` every
column is opened, `t = N`, `b = 0` (no sampling, no nonce).

## commit

`U[i][j] = evals[i·k2 + j]` — rows indexed by the high `row_vars`
variables, columns by the low `col_vars`. Row `i` is the coefficient vector
of `Û_i(X) = Σ_j U[i][j] X^j`, encoded as `(Û_i(ω^c))_{c<N}`, `ω` the
generator of the order-`N` subgroup (`7^{(p−1)/N}`). Leaf `c` = canonical LE
u64 of `Û_0(ω^c) … Û_{k1−1}(ω^c)`; tree as in `whir.md` § tree. Commitment
= root.

## transcript

Prover and verifier, in order, on the caller's transcript:

1. absorb `"lens-tensor-rs-v1"`, header, `m` (u64), root, point (Fp3 limbs), value
2. `α ← squeeze_fp3`
3. absorb `u = Σ_i α^i U_i` (k2 × Fp3), then `y = Σ_i eq(z_hi, i) U_i`
4. if `b > 0`: grind (see `whir.md` § grinding), absorb nonce
5. `t` indices in `[0, N)` by `squeeze_indices`, sorted, deduplicated

## verifier

Reject unless: header equals params; `|u| = |y| = k2`; nonce present iff
`b > 0` and meets `b`; `Σ_j y_j eq(z_lo, j) = value`; opened columns
authenticate (multi-opening, exact sibling count); for every opened column
`c` with `x = ω^c`: `u(x) = Σ_i α^i Û_i(ω^c)` and `y(x) = Σ_i eq(z_hi,i) Û_i(ω^c)`.

## soundness

Let `e = ⌊(N − k2)/2⌋`, δ = e/N ≤ (1 − ρ)/2 (unique decoding; RS minimum
distance `N − k2 + 1`). Round-by-round errors:

| round | error | source |
|---|---|---|
| α | `(k1 − 1)·N/|K|` | correlated agreement of RS for the powers generator, δ ≤ (1−ρ)/2: BCIKS20, as restated in WHIR (eprint 2024/1586) Theorem 4.8, first case, with ℓ = k1 |
| columns | `((1+ρ)/2)^t · 2^{-b}` | below |

Columns: if `Û` has no common agreement set of size `N − e` with `C^{k1}`,
then except with the α error `u`'s combination is δ-far from the code and
each opened column passes with probability `≤ 1 − (e+1)/N`. Otherwise the
unique codewords `c_i` agree with `Û` off `≤ e` columns; if `Enc(y) ≠
Σ eq(z_hi,i) c_i` the two distinct codewords agree on `≤ k2 − 1` points, so a
column passes with probability `≤ (k2 − 1 + e)/N`. Both are `≤ (N+k2)/(2N)
= (1+ρ)/2`. This is the binding argument of Brakedown (eprint 2021/1043,
§4, "Completeness and binding") with the γ/3 radius replaced by the
unique-decoding radius, made admissible by the correlated-agreement bound
above. The extracted rows are base-field: they agree with base-field
columns on `N − e > k2` points. Opening every column makes the column error 0.

`security_bits = min(log2|K| − log2((k1−1)N), t·q + b)` (`∞` for the first
term when k1 = 1). It is the round-by-round soundness of the IOP; see
`whir.md` § what the number means.

## wire

```text
header 6 B · u: u32 n, n×(3×u64) · y: same · nonce: u32 c∈{0,1}, c×u64
· height: u32 (= k1) · columns: u32 s, s×u64 (column-major, increasing index)
· siblings: u32 d, d×32 B
```

All integers LE; field elements canonical (`< p`), digest limbs canonical;
no trailing bytes.
