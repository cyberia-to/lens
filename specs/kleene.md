# Kleene star (tropical closure)

## definition

for a square matrix A in T^(n x n), the Kleene star is:

```
A* = I ⊕ A ⊕ A^(⊗2) ⊕ ... ⊕ A^(⊗(n-1))
```

where ⊕ is elementwise min and A^(⊗k) is the k-th tropical power.

equivalently: (A*)_{ij} = min over all paths from i to j using 0 to n-1 edges.

A* exists (is finite) if and only if A has no negative-weight cycles. over F_p this is always the case when inputs are non-negative.

## computation

### iterative (Floyd-Warshall-like)

```
D := A  (copy)
for k = 1 to n:
  for i = 1 to n:
    for j = 1 to n:
      D_{ij} := min(D_{ij}, D_{ik} + D_{kj})
return D with D_{ii} := 0
```

complexity: O(n³) time, O(n²) space.

### repeated squaring

```
S := I ⊕ A
repeat ceil(log₂(n)) times:
  S := S ⊗ S
return S
```

complexity: O(n³ log n) time. faster constant factor when n is large and tropical matmul is hardware-accelerated.

## properties

- A* ⊗ A* = A* (idempotent — the closure is a closure)
- (A*)_{ii} = 0 when no negative cycles (diagonal is the identity)
- A* = (I ⊕ A)^(n-1) (a single expression via repeated squaring)
- if λ(A) > 0 (positive tropical eigenvalue), then A* converges in exactly n-1 steps

## relation to other operations

the Kleene star is the fundamental matrix operation in tropical algebra. it is to (min, +) what matrix inversion is to (+, ×): it solves systems of tropical linear equations.

specific algorithms (Dijkstra for single-source, Bellman-Ford for negative edges) compute subsets of A* more efficiently for sparse graphs. these are nox programs that may call trop matrix operations as subroutines.

## F_p encoding

A* is computed over F_p values. the sentinel value p-1 represents +∞ (unreachable). the min operation uses field comparison; the + operation uses field addition with overflow check (if a + b >= p-1, result is p-1).
