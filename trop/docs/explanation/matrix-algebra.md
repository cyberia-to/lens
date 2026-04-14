# tropical matrices: shortest paths as linear algebra

tropical matrix multiplication is not an analogy for shortest paths — it IS shortest paths. the (min, +) product of two matrices computes exactly the minimum-weight two-hop paths. repeated multiplication extends to k hops. the entire apparatus of linear algebra — products, powers, closure — maps directly to graph algorithms.

## the product

given n x n tropical matrices A and B, their product C = A * B is:

```
C[i][j] = min_k (A[i][k] + B[k][j])
```

where min is tropical addition and + is tropical multiplication (ordinary addition). compare with the classical matrix product:

```
C[i][j] = sum_k (A[i][k] * B[k][j])
```

the structural correspondence is exact: sum becomes min, product becomes add. every theorem about classical matrix multiplication has a tropical analogue. the difference is semantic: classical products accumulate; tropical products optimize.

## graph interpretation

let A be the adjacency matrix of a weighted digraph: A[i][j] is the weight of edge (i, j), with +inf meaning no edge. then:

- A[i][j] = weight of the shortest 1-edge path from i to j
- (A^2)[i][j] = weight of the shortest 2-edge path from i to j
- (A^k)[i][j] = weight of the shortest k-edge path from i to j

this is not a metaphor. the tropical matrix product literally enumerates all intermediate vertices k, computes the weight of the two-segment path i -> k -> j, and takes the minimum. it is Dijkstra's relaxation step, expressed as algebra.

## identity and zero

the tropical identity matrix I has 0 on the diagonal and +inf elsewhere:

```
I[i][j] = { 0      if i = j
           { +inf   if i != j
```

it satisfies A * I = I * A = A for all A, because:

```
(A * I)[i][j] = min_k (A[i][k] + I[k][j]) = A[i][j] + 0 = A[i][j]
```

the tropical zero matrix (all entries +inf) satisfies A * 0 = 0 * A = 0, since adding +inf to any path makes it infinite.

## powers and path length

tropical matrix power A^k has a clean graph-theoretic meaning:

```
A^k[i][j] = shortest path from i to j using exactly k edges
```

in trop, matrix power is computed by repeated squaring:

```
A^8 = ((A^2)^2)^2    — three matrix multiplications instead of seven
```

complexity: O(n^3 log k) for n x n matrices. each multiplication is O(n^3), and repeated squaring uses ceil(log_2(k)) multiplications.

## elementwise addition

tropical matrix addition is elementwise min:

```
(A + B)[i][j] = min(A[i][j], B[i][j])
```

this corresponds to taking the shorter of two alternative paths. if A represents one set of routes and B represents another, A + B represents the best of both.

the operation is idempotent (A + A = A) and commutative (A + B = B + A). these properties follow directly from the idempotency and commutativity of min.

## the from_weights constructor

trop builds matrices from weighted edge lists:

```rust
let m = TropMatrix::from_weights(4, &[
    (0, 1, 3),  // edge 0 -> 1 with weight 3
    (1, 2, 5),  // edge 1 -> 2 with weight 5
    (2, 3, 1),  // edge 2 -> 3 with weight 1
]);
```

duplicate edges are handled by taking the minimum weight (tropical addition). missing edges default to +inf. this handles multigraphs naturally.

## fixed-size storage

trop stores matrices in a flat `[Tropical; MAX_DIM * MAX_DIM]` array with MAX_DIM = 64. the actual dimension `n` is tracked at runtime. indexing is `i * MAX_DIM + j`, not `i * n + j` — this wastes some memory but avoids dynamic allocation, keeping the library no_std compatible.

the 64 x 64 cap is sufficient for most graph algorithms in proof systems, where the matrices represent small constraint graphs or scheduling problems. for larger problems, the GPU shader (wgsl/) takes over.

## complexity

| operation | complexity | trop function |
|-----------|-----------|--------------|
| matrix multiply | O(n^3) | TropMatrix::mul |
| matrix add | O(n^2) | TropMatrix::add |
| matrix power A^k | O(n^3 log k) | TropMatrix::power |
| Kleene star A* | O(n^3) | kleene_star |
| determinant | O(n!) | determinant |
| eigenvalue | O(n^3) | eigenvalue |

## see also

- [[tropical-semiring]] — the algebraic foundation
- [[kleene-star]] — the all-pairs shortest-path closure via Floyd-Warshall
- [[applications]] — where tropical matrices appear in practice
