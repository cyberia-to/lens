---
tags: cyber, cip
crystal-type: entity
crystal-domain: math
alias: trop, tropical semiring, (min +) arithmetic
---
# trop

tropical semiring arithmetic for provable optimization. trop is to optimization what [[nebu]] is to field arithmetic and [[kuro]] is to binary arithmetic.

```
a ⊕ b = min(a, b)
a ⊗ b = a + b
```

no additive inverse. this is a semiring, not a field. the missing inverse is what makes optimization irreducible to field arithmetic.

## the semiring

| operation | definition | identity |
|-----------|-----------|----------|
| tropical add (⊕) | min(a, b) | +∞ |
| tropical mul (⊗) | a + b (standard addition) | 0 |

tropical "zero" is +∞ (absorbing element for min). tropical "one" is 0 (identity for standard addition). all elements live in Goldilocks F_p, with p-1 as the sentinel for +∞.

## why trop exists

encoding min(a, b) over [[Goldilocks field|Goldilocks]] (F_p) requires bit decomposition for comparison — ~10 constraints per operation. tropical matrix multiply (n x n) costs ~10n³ F_p constraints. native tropical: n³ operations at unit cost. 10x cost reduction for optimization proofs.

## operations

| operation | description | complexity |
|-----------|-------------|------------|
| tmin(a, b) | tropical addition: min | 1 comparison |
| tmax(a, b) | dual semiring: max | 1 comparison |
| neg(a) | duality map: (min,+) ↔ (max,+) | 1 negation |
| trop_add(A, B) | elementwise tropical add | O(n²) |
| trop_matmul(A, B) | tropical matrix multiply | O(n³) |
| trop_power(A, k) | k-th tropical power | O(n³ log k) |
| trop_star(A) | Kleene star: A* = I ⊕ A ⊕ ... ⊕ A^(n-1) | O(n³) |
| trop_eigenvalue(A) | minimum mean cycle weight | O(n³) |
| trop_det(A) | min-cost perfect matching | O(n!) exact |
| dual_verify(x, y, c) | LP duality certificate check | O(n²) |

algorithms using trop (Dijkstra, Hungarian, Viterbi, Sinkhorn) execute as [[nox]] programs with optional jet acceleration. trop provides the arithmetic; nox provides the control flow.

## (max, +) dual

the dual semiring swaps min for max: a ⊕' b = max(a, b). obtained by negating inputs, computing in (min, +), and negating the output. one implementation covers both semirings. used for longest path, max reliability, max-plus spectral theory.

## verification

tropical execution produces a result. the proof of optimality is a dual certificate — an algebraic object verified over F_p:

1. primal feasibility (structural check)
2. dual feasibility (range proofs, ~10 F_p constraints each)
3. strong duality (single equality)

the optimization runs tropical. the proof runs prime.

## structure

```
trop/
├── rs/              core library (no_std, zero deps)
│   └── src/lib.rs   tropical semiring, matrix ops
├── cli/             command-line tool
└── reference/       canonical specifications (9 docs)
```

## companion repos

| repo | role |
|------|------|
| [[nebu]] | Goldilocks field (verification backbone) |
| [[kuro]] | F₂ tower field (binary arithmetic) |
| [[nox]] | VM (tropical algorithms run as nox programs) |
| [[zheng]] | proof system (verifies optimality via dual certificates) |

## license

cyber license: don't trust. don't fear. don't beg.
