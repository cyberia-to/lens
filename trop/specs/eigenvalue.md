# tropical eigenvalue

## definition

the tropical eigenvalue of a square matrix A in T^(n x n) is:

```
λ(A) = min over all elementary cycles C of (weight(C) / length(C))
```

where weight(C) = sum of edge weights along C, and length(C) = number of edges.

this is the minimum mean cycle weight — the average cost per step of the cheapest cycle.

## eigenvector equation

the tropical eigenvalue λ satisfies:

```
A ⊗ v = λ ⊗ v
```

where λ ⊗ v means (λ + v₁, λ + v₂, ..., λ + vₙ), and equality is componentwise. the eigenvector v is the column of the "normalized" Kleene star of (A - λI), where (A - λI)_{ij} = A_{ij} - λ.

## critical cycle

the cycle achieving the minimum mean weight is the critical cycle. it determines the long-run behavior of tropical matrix powers:

```
lim_{k→∞} (1/k) · (A^(⊗k))_{ij} = λ(A)
```

for all i, j in the same strongly connected component.

## computation (Karp's algorithm)

```
D_{0,s} := 0 for a chosen source s
D_{0,v} := +∞ for v ≠ s
for k = 1 to n:
  D_{k,v} := min_u (D_{k-1,u} + A_{uv})

λ(A) = min_v max_{0 ≤ k < n} (D_{n,v} - D_{k,v}) / (n - k)
```

complexity: O(n³) time (n iterations, n vertices, n predecessors each).

alternative: Howard's policy iteration — often faster in practice, same worst case.

## properties

- λ(A) exists for any matrix with at least one finite cycle
- λ(A ⊕ B) ≤ min(λ(A), λ(B)) — eigenvalue respects tropical addition
- λ(A ⊗ B) relates to λ(A) and λ(B) but without simple formula
- if all diagonal entries A_{ii} ≥ 0, then λ(A) ≥ 0
- the number of distinct tropical eigenvalues is at most n

## F_p encoding

eigenvalue computation uses F_p division for the ratio weight/length. since length is a small integer (at most n), this is standard field arithmetic. the min over cycles uses tropical comparison.
