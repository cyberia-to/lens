# tropical determinant

## definition

the tropical determinant of a square matrix A in T^(n x n) is:

```
trop_det(A) = min over all permutations σ ∈ S_n of Σ_i A_{iσ(i)}
```

this replaces the classical sum-of-products-with-signs by a min-of-sums: the sign vanishes because tropical addition is idempotent (no cancellation).

## interpretation

trop_det(A) is the cost of the minimum-weight perfect matching in the bipartite graph where A_{ij} is the cost of assigning row i to column j. equivalently: the optimal assignment value.

the permutation achieving the minimum is the optimal assignment itself.

## properties

- trop_det(I) = 0 (tropical identity has zero diagonal, +∞ off-diagonal)
- trop_det(A ⊗ B) ≤ trop_det(A) + trop_det(B) (submultiplicative)
- trop_det(A) = +∞ if and only if no perfect matching exists (some row cannot be assigned)
- permuting rows or columns does not change the determinant value (up to relabeling)
- trop_det is NOT multiplicative in general: trop_det(A ⊗ B) ≠ trop_det(A) + trop_det(B)

## relation to tropical rank

A is tropically singular if the minimum in trop_det(A) is achieved by more than one permutation. equivalently: the optimal assignment is not unique.

tropical rank of A = largest k such that some k x k submatrix has a unique optimal permutation (tropically non-singular).

## computation

exact computation requires minimizing over n! permutations — this is the assignment problem. efficient algorithms (Hungarian, auction) solve it in O(n³) but are specific algorithms, not trop arithmetic primitives. they belong as nox programs.

for small n (n ≤ 8), direct enumeration over permutations is practical and serves as a reference implementation.

## dual certificate

the optimal assignment has an LP dual: row potentials u_i and column potentials v_j such that:

- u_i + v_j ≤ A_{ij} for all (i,j) — dual feasibility
- u_i + v_{σ(i)} = A_{iσ(i)} for assigned pairs — complementary slackness
- Σ u_i + Σ v_j = trop_det(A) — strong duality

the dual certificate proves optimality without re-solving: verification is O(n²) comparisons.
