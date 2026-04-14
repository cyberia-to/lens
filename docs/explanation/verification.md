# verifying optimality: LP duality for proofs

computing a shortest path is easy. proving it is optimal is the hard part. LP duality provides the mathematical machinery: every optimization result comes with a dual certificate that can be checked in polynomial time using field arithmetic. the tropical computation finds the answer; the dual certificate proves it is correct.

## the verification problem

a tropical computation produces a primal result — for example, "the shortest path from s to t has weight 17." how does a verifier confirm this without re-running the computation?

the answer is a dual certificate: an algebraic object that witnesses optimality. given the certificate, verification is pure arithmetic — field additions, comparisons, and range checks. no graph traversal, no priority queues, no dynamic programming.

## LP duality for shortest paths

the shortest-path problem has a clean LP formulation. for a graph with vertices V and edges E with weights w:

**primal LP** (minimize path weight):
```
minimize   sum_{(i,j) in E} w_{ij} * x_{ij}
subject to flow conservation at each vertex
           x_{ij} >= 0
```

**dual LP** (maximize potential difference):
```
maximize   d_t - d_s
subject to d_j - d_i <= w_{ij}   for all edges (i, j)
```

strong duality: the optimal primal value equals the optimal dual value.

the dual variables d_i are vertex potentials (distance labels). the dual constraint d_j - d_i <= w_{ij} says: the potential difference across any edge does not exceed its weight. this is exactly the triangle inequality.

## the dual certificate

a dual certificate for shortest paths consists of:

1. **vertex potentials** d_0, d_1, ..., d_{n-1} — one value per vertex
2. **claimed distance** D = d_t - d_s

verification checks:
- **dual feasibility**: d_j - d_i <= w_{ij} for every edge (i, j)
- **primal feasibility**: there exists a path from s to t with weight D
- **strong duality**: the path weight equals d_t - d_s

each dual feasibility check is one subtraction and one comparison — about 10 F_p constraints when encoded in a STARK. for a graph with m edges, the total verification cost is O(m) field operations.

## why verification is arithmetic

the dual certificate transforms a graph algorithm question into an algebraic question. given potentials d_i, checking d_j - d_i <= w_{ij} requires:

1. compute d_j - d_i (one field subtraction)
2. compute w_{ij} - (d_j - d_i) (one field subtraction)
3. range-check that the result is non-negative (~8 constraints)

no iteration. no recursion. no data-dependent branching. the verifier performs a fixed sequence of arithmetic operations, making it ideal for STARK verification.

## the three-phase protocol

```
phase 1: tropical computation (prover, using trop)
  - run Floyd-Warshall / Dijkstra / any algorithm
  - extract the optimal solution (primal witness)
  - extract vertex potentials (dual certificate)

phase 2: certificate packaging (prover)
  - encode primal witness and dual potentials as field elements
  - submit to the proof system

phase 3: arithmetic verification (verifier, using zheng over F_p)
  - check dual feasibility: d_j - d_i <= w_{ij} for all edges
  - check strong duality: primal cost = dual cost
  - accept or reject
```

the prover does the hard work (tropical algorithm, any complexity). the verifier does cheap work (polynomial arithmetic checks). this separation is the essence of proof systems.

## beyond shortest paths

LP duality extends to all tropical optimization problems:

**assignment problem** (tropical determinant): the dual certificate is a pair of vertex potentials (u_i, v_j) satisfying u_i + v_j <= A[i][j] for all i, j, with total dual value equal to the assignment cost.

**minimum mean cycle** (tropical eigenvalue): the dual certificate is a set of vertex potentials satisfying a tightened version of the triangle inequality, with the eigenvalue equal to the critical cycle weight.

**network flow**: dual certificates are node potentials and complementary slackness conditions. verification cost is O(|E|) field operations.

in every case, the pattern is the same: the prover runs an algorithm in the tropical semiring, extracts a dual certificate, and the verifier checks it using field arithmetic.

## cost comparison

| operation | tropical cost | verification cost (F_p) |
|-----------|--------------|------------------------|
| single-source shortest path | O(n^2) | O(m) comparisons = ~10m constraints |
| all-pairs shortest paths | O(n^3) | O(n * m) constraints |
| assignment | O(n^3) Hungarian | O(n^2) constraints |
| min mean cycle | O(n^3) Karp | O(n * m) constraints |

verification is always cheaper than computation. this is the fundamental asymmetry that makes proof systems useful.

## see also

- [[tropical-semiring]] — the algebraic foundation
- [[kleene-star]] — the computation being verified
- [[applications]] — problems where verification matters
