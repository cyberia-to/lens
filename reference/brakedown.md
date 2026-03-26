---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: expander PCS, Brakedown, recursive Brakedown, linear-code PCS
---
# expander PCS

the [[Goldilocks field|Goldilocks]] polynomial commitment scheme. commits via expander-graph linear codes. opens via recursive tensor decomposition. zero Merkle trees. O(N) commit. O(log N + λ) proof. one [[hemera]] call for binding.

implements the [[pcs|PCS]] trait for $\mathbb{F}_p$.

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

multiple openings amortised into one proof:

```
batch_open(f, [(r₁, y₁), (r₂, y₂), ..., (r_m, y_m)]):
  α = hemera_squeeze(r₁, ..., r_m)          random linear combination
  combined = Σ αⁱ · (f(rᵢ) - yᵢ) / (x - rᵢ)
  proof = open(combined, random_point)

cost: one recursive opening regardless of m
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

see [[pcs]] for the generic interface, [[binary-pcs]] for the F₂ backend, [[accumulator]] for folding, [[recursive brakedown]] for the research
