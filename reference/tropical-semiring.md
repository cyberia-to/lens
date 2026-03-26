---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: tropical PCS, semiring PCS, trop PCS
---
# tropical PCS

the (min,+) PCS backend for [[zheng]]. optimization workloads (shortest path, assignment, Viterbi, transport) prove natively through their algebraic structure. the tropical semiring has no additive inverse — min(a,b) cannot be undone — so the PCS delegates commitment to [[Brakedown]] (PCS₁) while exploiting tropical structure at the constraint level.

implements the [[pcs|PCS]] trait via delegation to Brakedown with tropical-aware constraint encoding.

## architecture

```
zheng
├── IOP:          SuperSpartan + sumcheck     (field-generic, shared)
├── Composition:  HyperNova folding           (field-generic, shared)
├── Hash:         hemera                      (one hash, universal)
├── PCS₁:         Brakedown (Goldilocks)      (arithmetic workloads)
├── PCS₂:         Binius (F₂ tower)           (binary workloads)
├── PCS₃:         Ikat (R_q)            (FHE/lattice workloads)
├── PCS₄:         Isogeny (F_q)              (privacy workloads)
└── PCS₅:         Tropical (min,+)            (optimization workloads)
```

## why a separate PCS spec

trop is not a field. (min,+) has no additive inverse — you cannot "un-min." polynomial commitment requires a ring or field. so tropical polynomials do not exist in the classical sense.

the resolution: tropical computation produces a WITNESS (the optimal assignment and its cost). the PROOF verifies the witness in F_p via Brakedown. the PCS₅ spec defines exactly what this witness-verify pattern looks like for each tropical workload.

this is not "trop doesn't have a PCS." this is "trop's PCS is a structured witness-verify protocol that delegates commitment to Brakedown."

```
trait PCS<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;
    fn open(poly: &MultilinearPoly<F>, point: &[F]) -> Opening;
    fn verify(commitment: &Commitment, point: &[F], value: F, proof: &Opening) -> bool;
}

// tropical PCS implements this via:
//   commit: Brakedown commit of the WITNESS polynomial (assignment + costs)
//   open:   Brakedown open + tropical validity certificate
//   verify: Brakedown verify + dual feasibility check
```

## tropical witness structure

every tropical computation has the same proof shape:

```
tropical_prove(problem, solution):
  1. WITNESS:   the optimal assignment (which edges, which nodes, which path)
  2. COST:      the claimed optimal cost (sum of assigned weights under min,+)
  3. CERTIFICATE: dual feasibility proof (no cheaper alternative exists)

  commit: Brakedown(witness_polynomial)
  open:   Brakedown opening at verification points
  verify: three checks in F_p
```

### three verification checks

```
check 1 — structural validity:
  the assignment is a valid solution to the problem
  (e.g., the path visits each node at most once, the matching is perfect)
  cost: O(|assignment|) F_p constraints (membership + structure checks)

check 2 — cost correctness:
  the claimed cost equals the sum of assigned edge weights
  sum(weight[e] for e in assignment) == claimed_cost
  cost: O(|assignment|) F_p additions

check 3 — optimality (dual certificate):
  no cheaper assignment exists
  LP dual: feasible dual solution with matching objective value
  cost: O(|problem|) F_p constraints (dual feasibility checks)
```

the prover does the hard work (tropical optimization). the verifier checks three simple properties in F_p. this is the hint (Layer 2) pattern: non-deterministic witness, deterministic verification.

## per-workload specifications

### shortest path

```
problem:  weighted directed graph G = (V, E, w), source s, target t
witness:  path P = (s, v₁, v₂, ..., t) with edges e₁, e₂, ..., eₖ
cost:     Σ w(eᵢ) (sum of edge weights along path)
certificate: distance labels d[v] satisfying d[t] - d[s] = cost
             and ∀(u,v) ∈ E: d[v] ≤ d[u] + w(u,v) (dual feasibility)

constraints: O(|V| + |E|) F_p checks
tropical cost: O(|E| log |V|) trop operations (Dijkstra)
```

### optimal assignment (Hungarian)

```
problem:  n×n cost matrix C
witness:  permutation σ mapping rows to columns
cost:     Σ C[i, σ(i)]
certificate: dual variables u[i], v[j] satisfying
             Σu[i] + Σv[j] = cost (strong duality)
             u[i] + v[j] ≤ C[i,j] ∀i,j (dual feasibility)

constraints: O(n²) F_p checks
tropical cost: O(n³) trop operations
```

### Viterbi decoding

```
problem:  HMM (states S, transitions T, emissions E), observation sequence O
witness:  state sequence Q = (q₁, q₂, ..., qₜ)
cost:     Σ log T(qᵢ, qᵢ₊₁) + Σ log E(qᵢ, oᵢ) (log-probability = tropical cost)
certificate: backward pass values confirming no higher-probability path

constraints: O(|S|² × |O|) F_p checks
tropical cost: O(|S|² × |O|) trop operations
```

### optimal transport

```
problem:  source distribution μ, target distribution ν, cost matrix C
witness:  transport plan π[i,j] (how much mass moves from i to j)
cost:     Σ π[i,j] × C[i,j]
certificate: Kantorovich dual potentials (f, g) satisfying
             f[i] + g[j] ≤ C[i,j] ∀i,j (dual feasibility)
             Σf[i]μ[i] + Σg[j]ν[j] = cost (strong duality)

constraints: O(n²) F_p checks
tropical cost: O(n³ log n) trop operations
```

## cost comparison

| workload | tropical computation | verification (F_p) | ratio |
|----------|---------------------|-------------------|-------|
| shortest path (V=1000, E=5000) | ~50K trop ops | ~6K F_p constraints | 8× |
| assignment (n=100) | ~1M trop ops | ~10K F_p constraints | 100× |
| Viterbi (S=50, T=1000) | ~2.5M trop ops | ~2.5M F_p constraints | 1× |
| transport (n=100) | ~3M trop ops | ~10K F_p constraints | 300× |

verification is always cheaper than computation (except Viterbi where they match). the asymmetry is the point — the prover optimizes, the verifier checks.

## cross-algebra composition

tropical sub-traces fold into the shared F_p accumulator via HyperNova:

```
trop computation → witness + certificate
                       ↓
    commit witness via Brakedown (PCS₁)
    encode three checks as F_p CCS constraints
                       ↓
    HyperNova fold into shared F_p accumulator
    cost: ~766 F_p constraints per boundary crossing
```

universal CCS with selectors:

```
universal_ccs = {
  sel_Fp:   1 for Goldilocks rows
  sel_F2:   1 for binary rows
  sel_ring: 1 for ring-structured rows
  sel_Fq:   1 for isogeny rows
  sel_trop: 1 for tropical witness-verify rows
}
```

## trop dependency

a separate repo **trop** provides tropical semiring operations:
- (min, +) arithmetic
- tropical matrix multiply
- shortest path, Hungarian, Viterbi algorithms
- no hemera dependency, no nebu dependency
- pure semiring algebra

zheng depends on trop for witness generation. verification runs on Brakedown (PCS₁).

## open questions

1. **optimality gap tolerance**: should the dual certificate prove EXACT optimality or allow ε-gap? exact requires more constraints, approximate is faster
2. **incremental tropical proofs**: when the graph changes by one edge, can the witness update incrementally without full recomputation?
3. **tropical recursion**: can tropical witnesses fold into the accumulator via a tropical-specific folding step, or must everything cross to F_p first?

see [[polynomial-commitment]] for Brakedown PCS, [[binary-pcs]] for F₂ backend, [[isogeny-pcs]] for F_q backend, [[recursion]] for cross-algebra folding
