# tropical matrix algebra

## tropical matrix multiplication

for A ∈ T^(m×n) and B ∈ T^(n×p):

(A ⊗ B)_{ij} = min_k (A_{ik} + B_{kj})

this is the (min, +) analog of standard matrix multiply where Σ becomes min and × becomes +.

complexity: O(mnp) tropical operations (same as classical matmul).

## interpretation

(A ⊗ B)_{ij} = shortest path from i to j through one intermediate node k, where A_{ik} is the cost from i to k and B_{kj} is the cost from k to j.

## tropical matrix power

A^(⊗k) = A ⊗ A ⊗ ... ⊗ A (k times)

(A^(⊗k))_{ij} = shortest path from i to j using exactly k edges.

the tropical closure (Kleene star):

A* = I ⊕ A ⊕ A^(⊗2) ⊕ ... ⊕ A^(⊗n-1)

(A*)_{ij} = shortest path from i to j using any number of edges (up to n-1).

this is Floyd-Warshall: compute all-pairs shortest paths.

## tropical eigenvalue

the tropical eigenvalue λ of matrix A satisfies:

A ⊗ v = λ ⊗ v

where λ ⊗ v means (λ + v_1, λ + v_2, ..., λ + v_n), and equality means componentwise min.

the maximum cycle mean:

λ(A) = min over all elementary cycles C of (weight(C) / length(C))

this is the minimum average edge weight over all cycles — the critical cycle.

computation: O(n³) via Karp's algorithm or Howard's policy iteration.

## tropical determinant

trop_det(A) = min over all permutations σ of Σ_i A_{iσ(i)}

this is the optimal assignment problem: assign each row to a column minimizing total cost.

computation: O(n³) via the Hungarian algorithm.

## tropical rank

the tropical rank of A is the minimum k such that A can be written as a tropical product of an (n×k) and a (k×m) matrix.

tropical rank ≤ classical rank. determining tropical rank is NP-hard in general, but approximations suffice for most applications.

## identity and zero

tropical identity matrix I: I_{ii} = 0, I_{ij} = +∞ (i ≠ j).
tropical zero matrix 0: all entries +∞.

A ⊗ I = I ⊗ A = A (identity property).
A ⊕ 0 = A (zero property).
