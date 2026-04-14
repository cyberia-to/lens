# Kleene star: all-pairs shortest paths

the Kleene star of a tropical matrix is the all-pairs shortest-path closure. it is to the tropical semiring what matrix inversion is to a field: the solution to a system of equations. trop computes it via Floyd-Warshall in O(n^3), which is exactly the Kleene star algorithm expressed in the (min, +) semiring.

## definition

for an n x n tropical matrix A, the Kleene star is:

```
A* = I (+) A (+) A^2 (+) ... (+) A^(n-1)
```

where (+) is tropical addition (elementwise min) and A^k is the k-th tropical power. the result:

```
A*[i][j] = shortest path from i to j using any number of edges (0 to n-1)
```

the I term handles zero-edge paths (distance from a vertex to itself is 0). the A term handles one-edge paths. the A^k term handles k-edge paths. the min over all terms gives the overall shortest path.

## why n-1 edges suffice

a shortest path in a graph with n vertices uses at most n-1 edges (otherwise it contains a cycle). if all edge weights are non-negative, a shortest path never benefits from cycling. therefore the sum A* = I + A + A^2 + ... converges after n-1 terms.

in trop's u64 encoding, all weights are non-negative by construction. the Kleene star always converges. negative-weight cycles cannot arise.

## Floyd-Warshall as Kleene star

the Floyd-Warshall algorithm is not merely "related to" the Kleene star — it IS the Kleene star computation, organized by intermediate vertices:

```
initialize D = I (+) A
for k = 0 to n-1:
  for i = 0 to n-1:
    for j = 0 to n-1:
      D[i][j] = min(D[i][j], D[i][k] + D[k][j])
```

after processing vertex k, D[i][j] holds the shortest path from i to j using only intermediate vertices {0, 1, ..., k}. after processing all n vertices, D = A*.

the algebraic interpretation: the loop over k eliminates variables one at a time, exactly like Gaussian elimination in a field. Floyd-Warshall is Gaussian elimination in the tropical semiring.

## trop's implementation

trop implements the Kleene star in `kleene.rs`:

```rust
let star = kleene_star(&a);
// star.get(i, j) is the shortest path from i to j
```

the implementation:
1. initializes D = I (+) A (diagonal entries get min(0, A[i][i]))
2. runs Floyd-Warshall relaxation: O(n^3)
3. returns the result as a TropMatrix

the early-exit optimization skips row i when D[i][k] = +inf, since adding infinity to any path gives infinity. for sparse graphs, this eliminates a significant fraction of the inner loop iterations.

## properties of A*

**idempotency.** A* * A* = A*. the closure of a closure is itself. once you have all-pairs shortest paths, multiplying again does not change anything.

**diagonal.** A*[i][i] = 0 when there are no negative-weight cycles. the shortest path from a vertex to itself is the zero-edge path (weight 0).

**fixed point.** A* = I (+) A * A*. the Kleene star satisfies the fixed-point equation: the shortest path from i to j is either the zero-edge path (I) or a shortest path that starts with one edge of A and then follows a shortest path in A*.

**monotonicity.** if A[i][j] <= B[i][j] for all i, j, then A*[i][j] <= B*[i][j]. shorter edges give shorter paths.

## the repeated squaring alternative

instead of Floyd-Warshall, one can compute A* via repeated squaring:

```
S = I (+) A
repeat ceil(log_2(n)) times:
  S = S * S
```

this takes O(n^3 log n) time — worse than Floyd-Warshall's O(n^3). however, repeated squaring is more parallelizable: each matrix multiplication can be offloaded to the GPU via the tropical matmul shader in wgsl/. for large n on GPU hardware, the constant factor advantage can dominate.

## relationship to Dijkstra and Bellman-Ford

Dijkstra's algorithm computes one row of A* (single-source shortest paths) in O(n^2) or O(m log n) with a priority queue. Bellman-Ford computes one row in O(nm). these are more efficient than computing the full A* when only one source is needed.

the algorithms are not alternatives to the Kleene star — they are optimizations for special cases. the full A* computation is unavoidable when all-pairs distances are required, as in network routing tables, metric closures, or tropical eigenvalue computation.

## see also

- [[tropical-semiring]] — the algebraic foundation
- [[matrix-algebra]] — tropical matrix multiplication
- [[verification]] — proving that shortest-path results are optimal
