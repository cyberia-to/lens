---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: expander lens, Brakedown, recursive Brakedown, linear-code lens
---
# scalar field lens (Brakedown)

the [[Goldilocks field|Goldilocks]] polynomial commitment. commits via expander-graph linear codes. opens via recursive tensor decomposition. zero Merkle trees. O(N) commit. O(log N + λ) proof. one [[hemera]] call for binding.

implements the [[trait|Lens]] trait for $\mathbb{F}_p$. part of the five-lens architecture — see [[commitment]] for the full picture.

## encoding

the expander graph $E = (L, R, \text{edges})$ with $|L| = N$, $|R| = c \cdot N$, left-degree $d$:

```
encode(evaluation_table v ∈ F^N):
  w = E · v                 sparse matrix-vector multiply, O(d·N) field ops
  binding = hemera(w)        one hash call, 32 bytes

  d ≈ 20-30 for 128-bit security over Goldilocks
  total: ~25N field multiplications
```

the expander property: any sufficiently large subset of encoded positions determines the original polynomial. this replaces Reed-Solomon's algebraic distance with combinatorial expansion.

## recursive opening

to prove $f(r) = y$:

```
OPEN(f, r):
  level 0: q₀ = tensor_reduce(f, r)        √N elements
            C₁ = commit(q₀)                 commit the opening (not send it)
  level 1: q₁ = tensor_reduce(q₀, r₁)      N^{1/4} elements
            C₂ = commit(q₁)
  ...
  level d: q_d has ≤ λ elements             d = log log N
            send q_d directly

  proof = (C₁, C₂, ..., C_d, q_d)

VERIFY(C₀, r, y, proof):
  for level i = 0..d-1:
    r_i = transcript.squeeze()               Fiat-Shamir challenge
    check C_{i+1} consistent with tensor reduction at r_i
  check q_d evaluates to y at composed point
```

each level SQUARES the compression. log log N levels. prover: O(N + √N + ...) = O(N). proof: O(log N + λ) field elements.

## batch opening

multiple openings amortized into one proof via the multilinear equality polynomial:

```
batch_open(f, [(r₁, y₁), (r₂, y₂), ..., (r_m, y_m)]):
  α = transcript.squeeze_field()
  r* = transcript.squeeze_field()
  combined_value = Σ αⁱ · yᵢ · eq(rᵢ, r*)
  proof = open(f, r*)

  eq(r, x) = Π_j (r_j x_j + (1-r_j)(1-x_j))

cost: one recursive opening + O(m·ν) for combining claims
```

critical for: state jets (4 openings per cyberlink → 1 proof), DAS (20 samples → 1 proof), namespace queries (N entries → 1 proof).

## numbers

at N = 2²⁰, λ = 128:

```
commit:     O(N) field ops, ~40 ms single core
proof:      ~1.3 KiB (log log N commitments + log N sumcheck + λ direct)
verify:     ~660 field ops, ~5 μs
binding:    1 hemera call (32 bytes)
```

## Goldilocks compatibility

requires:
1. fast field arithmetic — Goldilocks: 4-5 cycle multiply ([[nebu]])
2. expander graph family — Margulis or Ramanujan, explicit construction
3. linear-time encoding — sparse matrix multiply, [[nebu]] SIMD

the expander graph needs left-degree $d \approx 20\text{-}30$ for 128-bit security over Goldilocks ($|\mathbb{F}| \approx 2^{64}$).

see [[commitment]] for the shared interface, [[binary-tower]] for Binius, [[isogeny-curves]] for Porphyry
