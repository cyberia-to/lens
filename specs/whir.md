---
tags: lens, spec, pcs, whir
crystal-type: entity
crystal-domain: crypto
alias: WHIR lens, lens-whir-rs-v1
---
# WHIR — multilinear PCS over Goldilocks

WHIR of Arnon, Chiesa, Fenzi, Yogev ("WHIR: Reed–Solomon Proximity Testing
with Super-Fast Verification", eprint 2024/1586; theorem numbers below refer
to it) as a multilinear PCS. Field `F` = Goldilocks, challenges in
`K = Fp3`. Implements `MultilinearPcs` (`core/src/pcs.rs`), name
`lens-whir-rs-v1`, crate `cyber-lens-rspcs`. Hash: hemera only. Not zero
knowledge.

## parameters

| field | default | meaning |
|---|---|---|
| `log_inv_rate` r₀ | 4 | starting rate ρ₀ = 2^-r₀ |
| `folding_factor` k | 4 | variables folded per round |
| `security_target` λ | 128 | bits |
| `pow_bits` b | 16 | grinding budget per challenge |
| `decoding` | Johnson | `Unique` or `Johnson` |
| `max_final_vars` | 8 | stop committing when ≤ this many variables remain |

Header (8 B): `01 ‖ r₀ ‖ k ‖ b ‖ λ (u16 LE) ‖ decoding (0|1) ‖ max_final_vars`.

## schedule

`k₀ = min(k, m)`; then `k_i = min(k, rem)` while `rem > max_final_vars`.
`M` = number of folds = number of committed functions `f₀ … f_{M−1}`;
`f_i` has `m_i` variables, rate `r_{i+1} = r_i + k_i − 1`, domain
`L_i` = order-`n_i` subgroup, `n_i = 2^{m_i + r_i} = n₀/2^i ≤ 2^32`. The
last `m_M = rem` variables are sent as a polynomial.

Per function `i` (all derived, deterministic):

- regime: Unique → δ_i = (1−ρ_i)/2 − 1/n_i, list ℓ = 1. Johnson(μ), integer
  μ ∈ [3, 64] → δ_i = 1 − (1 + 1/(2μ))√ρ_i, ℓ_i ≤ μ/ρ_i (Theorem 4.3 with
  η = √ρ/(2μ)). μ minimises `t_i` subject to fold grinding ≤ b; ties to
  less grinding.
- `fold_pow_i = ⌈λ − fold bits⌉⁺` (must be ≤ b, else μ = 3 and b).
- `ood_i` = least s ≥ 1 with `ood` term ≥ λ (Johnson); 0 (Unique).
- `query_pow_i = min(b, λ)`; `t_i` = least t with the query term ≥ λ; if
  `t_i ≥ n_i/2^{k_i}` every leaf is opened, `t_i = n_i/2^{k_i}`,
  `query_pow_i = 0`.

## commit

`c` = monomial coefficients of `f` (Möbius transform of the table, bit `k`
of an index ↔ variable `k`). `f̂(X) = Σ c_i X^i`, so `f̂(x) = f(x, x², x⁴,
…)`. Codeword `(f̂(ω^j))_{j<n₀}`, ω = 7^{(p−1)/n₀}. Leaf `j < n₀/2^{k₀}` =
the coset `x_t = ω^j ζ^t`, `ζ = ω^{n₀/2^{k₀}}`, i.e. symbols
`j + t·n₀/2^{k₀}`, `t < 2^{k₀}`. Commitment = Merkle root.

### Fp3 messages

`Whir::commit_ext` commits an Fp3-valued table `f: {0,1}^m → K`: the
same Möbius transform (over K), the same domain `L ⊂ F` and leaf layout;
the codeword `(f̂(ω^j))_j` is encoded limb-wise (the code is F-linear and
`L ⊂ F`) and every leaf carries `3×u64` symbols. The code is
`RS[K, L, 2^m]`. Every soundness term below is already stated over `K`,
so `security_bits` holds unchanged; the base-field remark at the end of
§ soundness is simply not needed. An Fp3 opening absorbs the extra
domain string `"ext"` right after the scheme name in step 1 of the
transcript, so a base and an Fp3 opening never share a transcript; the
proof bytes have the same layout with `3×u64` symbols in the openings of
`f_0` (`WhirProof::from_bytes_with(bytes, ext0 = true)`). Base commitments,
their transcripts and their proof bytes are unchanged.

### leaf access

For protocols that read `f_0` before or instead of an opening (zheng's
accumulation spot checks): `LeafLayout::derive(params, m)` gives
`log2 n_0` and `log2 W`; symbol `s` is `f̂_0(ω^s)` and lives in leaf
`s mod (n_0/W)` at coset position `⌊s/(n_0/W)⌋`. `WhirData::open_leaves`
returns the deduplicated multi-opening of sorted distinct leaves (the
same `Opening` as inside a proof, `Opening::to_bytes`/`from_bytes`);
`verify_leaves(root, layout, ext, leaves, opening)` authenticates it and
returns every opened symbol. `WhirData::univariate(z)` answers `f̂_0(z)`
at any `z ∈ K` (out-of-domain samples).

### tree

Leaf `j` with canonical LE bytes `B`: `hemera::tree::hash_leaf(B, j,
count = 1)`; node: `hash_node(l, r, is_root)`. Chunk vs parent flags
separate leaves from nodes, the root flag the root, the counter binds the
position. 32-byte digests. A multi-opening of sorted distinct leaves sends,
level by level from the leaves, in increasing index order, exactly the
siblings the verifier cannot compute; any missing, extra or reordered
digest fails.

### grinding

With `b > 0` before a challenge: seed = `squeeze()` (32 B); nonce = least
u64 with `hemera::hash(seed ‖ nonce_le)` first limb ≡ 0 mod 2^b; absorb
nonce. `b = 0`: nothing.

### challenges

`squeeze_fp3`: three limbs of one 32-byte squeeze; each limb is a hemera
output element, uniform on F in the ideal-permutation model (limbs ≥ p are
impossible and would be rejected, never reduced): bias 0.
`squeeze_indices(t, log L)`: one limb per index, rejection above
`⌊p/L⌋·L`, then `mod L`: exactly uniform.

## open / verify — transcript order

1. absorb `"lens-whir-rs-v1"`, header, `m`, root₀, point `z`, value `v`.
2. for each of `ood₀` samples: `ζ ← squeeze_fp3`, absorb `y = f̂(ζ)`.
3. `γ ← squeeze_fp3`. Weights: `eq(z, ·)` with σ = v, and `γ^j·eq(pow(ζ_j), ·)`
   with `σ += γ^j y_j` (Construction 5.5).
4. sumcheck, `k₀` rounds: absorb `h(0), h(2)` (`h(1) = σ − h(0)`), grind
   `fold_pow₀`, `α ← squeeze_fp3`, `σ = h(α)`.
5. for `i = 1 … M−1`: absorb root_i; `ood_i` samples `ζ`, absorb
   `f̂_i(ζ)`; grind `query_pow_{i−1}`; `t_{i−1}` leaf indices of `f_{i−1}`;
   `γ ← squeeze_fp3`; open leaves; for each opened leaf `j`,
   `g_j = Fold(f_{i−1}, α_{i−1})(r_j)`, `r_j = ω_{i−1}^{j·2^{k_{i−1}}}`
   (Definition 4.14, iterated: `f'(x²) = (f(x)+f(−x))/2 + α(f(x)−f(−x))/(2x)`);
   add `γ^s eq(pow(ζ), ·)` then `γ^{s'} eq(pow(r_j), ·)` with
   `σ += γ^· y, γ^· g_j`; sumcheck `k_i` rounds with `fold_pow_i`.
6. absorb final coefficients `f̂_M` (2^{m_M} × Fp3); grind
   `query_pow_{M−1}`; `t_{M−1}` indices; check `f̂_M(r_j) = g_j`.
7. closing: `σ = Σ_c coef_c · eq(q_c[..pre], α[off_c..]) · f_M(q_c[pre..])`
   over all weights `c` (`off_c` = folds before the weight's round,
   `pre = m_round − m_M`); this is `Σ_b ŵ(f̂_M(b), α, b)` exactly
   (Construction 5.1, decision 3c).

The verifier also rejects on header ≠ params, any length ≠ the schedule's,
non-canonical encodings, and trailing bytes.

## soundness

Round-by-round errors, `|K| = p³` (log2|K| ≈ 191.99996):

| term | error | source |
|---|---|---|
| ood₀ | `(ℓ₀²/2)·(2^{m}/|K|)^{s₀}` | Theorem 7.5 (ε^out) with Lemma 4.25 |
| comb₀ | `s₀·ℓ₀/|K|` | Theorem 5.6 (`(t−1)ℓ/|F|`, t = 1 + s₀ weights) |
| fold_{i,s}, s = 1..k_i | `3ℓ_i/|K| + ε_mca(RS[n_i/2^s, 2^{m_i−s}])`, minus grinding | Theorem 5.2 (ε^fold, d* = d = 3 for `Z·eq(z, X)`) |
| ood_i | `(ℓ_i²/2)·(2^{m_i}/|K|)^{s_i}` | Theorem 5.2 (ε^out) with Lemma 4.25 |
| shift_i | `(1−δ_{i−1})^{t_{i−1}} + ℓ_i(t_{i−1}+s_i)/|K|`, minus grinding | Theorem 5.2 (ε^shift; the γ-combination has degree t + s) |
| fin | `(1−δ_{M−1})^{t_{M−1}}`, minus grinding | Theorem 5.2 (ε^fin) |

Opening every leaf sets the `(1−δ)^t` part to 0.

`ε_mca` for the affine line `(1, α)` (WHIR folds one variable per sumcheck
round):

- Unique: `n/|K|` — Theorem 4.8 (BCIKS20), first case, ℓ = 2, with
  Lemma 4.10 / Corollary 4.11 (mutual correlated agreement = correlated
  agreement in the unique-decoding regime).
- Johnson(μ): `(μ+½)^7/(3ρ^{3/2}) · n²/|K|` at δ = 1 − (1+1/(2μ))√ρ —
  Bordage, Chiesa, Guan, Manzur, "All Polynomial Generators Preserve
  Distance with Mutual Correlated Agreement", eprint 2025/2051 (CCC 2026),
  Definition 9.1 and Lemma 9.3 with generator degree d = 1; the affine-line
  case first by Haböck, eprint 2025/2110, Theorem 2. This resolves WHIR's
  Conjecture 4.12 (1); no conjecture is used.

A codeword in RS over K within δ < 1 − ρ of an F-valued word has F
coefficients (agreement on > 2^m points of F), so the extracted polynomial
of the base-field commitment is over F.

`security_bits = min over all terms of (−log2 error + grinding bits)`.

### what the number means

It is the round-by-round soundness of the IOP of proximity, the quantity
WHIR and the ethSTARK-style grinding accounting report. The
non-interactive proof (Merkle commitments, Fiat–Shamir with hemera as a
random oracle) is then broken by an adversary making Q hemera calls with
probability about `Q·2^{−bits}` plus Merkle collision terms of order
`Q²/2^{256}`; those hash terms are not included, and no quantum-ROM
bound is claimed by this number.

## wire

```text
header 8 B
ood0 exts · sumcheck0 exts (h0,h2 per round) · fold_nonces0 u64s (k₀ or 0)
rounds: u32 R=M−1, each: root 32 B · ood exts · query_nonce u64s (0|1)
        · opening of f_{i−1} · sumcheck exts · fold_nonces u64s
final_poly exts (2^{m_M}) · final_nonce u64s (0|1) · final opening of f_{M−1}
opening: u32 width · u32 count + symbols (u64, or 3×u64 for f_i, i ≥ 1)
         · u32 d + d×32 B siblings
exts: u32 n + n×(3×u64) · u64s: u32 n + n×u64
```

All integers LE, field elements and digest limbs canonical, no trailing
bytes.
