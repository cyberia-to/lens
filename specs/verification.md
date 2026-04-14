# optimality verification

how [[zheng]] proves that a tropical computation produced an optimal result.

## the verification principle

tropical execution produces a primal solution (the optimal value and witness). the proof of optimality requires a dual certificate — a witness that no better solution exists.

this follows from LP duality: for every minimization problem, there exists a dual maximization problem. strong duality says their optima are equal. the dual solution IS the optimality certificate.

## dual certificate structure

for any tropical optimization with LP dual, the certificate has three parts:

1. **primal feasibility** — the claimed solution satisfies constraints (structural check)
2. **dual feasibility** — the dual variables satisfy all dual constraints (range proofs over F_p)
3. **strong duality** — primal objective = dual objective (single equality check)

complementary slackness follows from (2) and (3) but may be checked explicitly for tighter proofs.

## general verification protocol

```
prover (tropical):
  1. solve the optimization problem (using any algorithm)
  2. extract primal solution x
  3. extract dual certificate y

verifier (F_p via zheng):
  1. check primal feasibility: x satisfies structural constraints
  2. check dual feasibility: y_i + y_j ≤ c_{ij} for all relevant (i,j)
  3. check strong duality: primal_cost(x) = dual_cost(y)
```

the prover does the hard work (tropical computation, any algorithm). the verifier does cheap checks (F_p arithmetic). verification cost is typically O(n²) or O(|E|) — polynomial in problem size, independent of solution algorithm.

## F_p verification cost

each dual feasibility check requires a comparison over F_p, which decomposes into:
- 1 subtraction (1 constraint)
- 1 range proof for non-negativity (~8 constraints)

total per constraint: ~10 F_p constraints.

for a problem with m dual constraints:
- dual feasibility: ~10m constraints
- complementary slackness: m equality checks (m constraints)
- strong duality: 1 sum constraint

total: O(m) F_p constraints.

## why this is arithmetic

the dual certificate is not an algorithm — it is an algebraic object. given the certificate, verification is pure arithmetic: field additions, subtractions, and range checks. no control flow, no iteration, no graph traversal.

this is why verification belongs in trop (the arithmetic library) rather than in nox (the algorithm executor). trop defines what a valid certificate looks like. nox programs produce certificates. zheng verifies them using trop's arithmetic.

## composability

certificates compose: if problem P decomposes into subproblems P₁, ..., Pₖ, the certificate for P is the collection of certificates for P₁, ..., Pₖ plus a proof that the decomposition is valid. this enables incremental verification of large tropical computations.
