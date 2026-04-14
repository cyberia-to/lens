# three arithmetics: F_2, F_p, and (min, +)

the cyber ecosystem uses three distinct algebraic structures, each optimal for a different class of computation. kuro provides F_2 tower arithmetic. nebu provides Goldilocks field arithmetic. trop provides tropical semiring arithmetic. understanding when to use which — and how they compose — is essential for efficient proof systems.

## the three algebras

| property | F_2 (kuro) | F_p (nebu) | tropical (trop) |
|----------|-----------|-----------|----------------|
| structure | field | field | semiring |
| addition | XOR | mod p addition | min |
| multiplication | tower Karatsuba | mod p multiplication | ordinary addition |
| additive identity | 0 | 0 | +inf |
| multiplicative identity | 1 | 1 | 0 |
| additive inverse | exists (a = -a) | exists | does NOT exist |
| characteristic | 2 | p ~ 2^64 | N/A (not a ring) |

the critical distinction: F_2 and F_p are fields (every nonzero element has a multiplicative inverse). the tropical semiring is not even a ring (no additive inverse). this structural difference determines what each can express.

## what each arithmetic is for

**F_2 (kuro): bitwise computation.** XOR, AND, bit shifts, boolean circuits, hash function internals, 1-bit quantized neural networks. the natural algebra for anything that operates on individual bits. 128 operations per instruction via packed representation.

**F_p (nebu): general arithmetic.** addition, multiplication, division of large numbers. polynomial evaluation, NTT, Poseidon2 hashing, Reed-Solomon encoding. the natural algebra for anything that needs full-precision arithmetic with inverses.

**tropical (trop): optimization.** shortest paths, minimum cost, scheduling, assignment. the natural algebra for anything involving min and + together. linear algebra over the tropical semiring IS shortest-path computation.

the rule: use the algebra that matches the problem. encoding optimization into F_p requires ~10x overhead (bit decomposition for comparisons). encoding bitwise operations into F_p requires ~32x overhead. encoding field arithmetic into the tropical semiring is impossible (no inverses). each algebra has a domain where it is irreplaceable.

## the cost of wrong-domain encoding

| operation | native cost | cost in F_p | cost in F_2 | cost in tropical |
|-----------|------------|------------|------------|-----------------|
| XOR | 1 (F_2) | ~32 | 1 | impossible |
| field multiply | 1 (F_p) | 1 | ~20 | impossible |
| min(a, b) | 1 (trop) | ~10 | ~64 | 1 |
| shortest path (n x n) | n^3 (trop) | ~10n^3 | impossible | n^3 |
| polynomial eval | n (F_p) | n | ~20n | impossible |

the penalties are structural, not implementational. no amount of engineering eliminates the 10x overhead of encoding min into F_p, because comparison requires bit decomposition. the overhead comes from the mismatch between algebraic structure and problem structure.

## how they compose in a proof system

a proof system (zheng) verifies computations. real computations mix all three types of work:

```
example: verify a network routing computation

step 1: shortest paths (tropical)
  - compute A* via Floyd-Warshall in the tropical semiring
  - extract dual certificate (vertex potentials)

step 2: certificate verification (F_p)
  - check d_j - d_i <= w_{ij} for all edges
  - this is range-check arithmetic, native to F_p

step 3: hash commitment (F_2)
  - hash the distance table for commitment
  - bitwise operations in the hash function, native to F_2
```

each step runs in its native algebra. the cross-algebra boundaries are crossed only when necessary.

## crossing boundaries

moving values between algebras has a cost:

**tropical to F_p**: embed the u64 value as a Goldilocks field element. cost: O(1) per element. the u64 representation is already a valid F_p element (assuming the value is less than p).

**F_p to tropical**: interpret the field element as a u64 weight. cost: O(1). requires the value to be in [0, p-1], which it always is.

**F_2 to F_p**: bit-decompose a field element or import bits as field elements. cost: ~64 constraints for a 64-bit value.

**F_p to F_2**: decompose the field element into 64 bits. cost: ~64 constraints for range checks.

**F_2 to/from tropical**: no direct path. go through F_p.

the cheapest boundary is tropical <-> F_p, because tropical values are already u64 integers that live inside the Goldilocks field. the most expensive is F_2 <-> F_p, because it requires bit decomposition.

## the three-zone architecture

```
┌─────────────────────────────┐
│      optimization zone      │
│      trop (min, +)          │
│      shortest paths,        │
│      assignment, scheduling │
└──────────┬──────────────────┘
           │ O(1) per element
┌──────────┴──────────────────┐
│      arithmetic zone        │
│      nebu (F_p)             │
│      verification,          │
│      polynomial ops, NTT    │
└──────────┬──────────────────┘
           │ ~64 constraints per crossing
┌──────────┴──────────────────┐
│      binary zone            │
│      kuro (F_2)             │
│      hashing, boolean       │
│      circuits, BitNet       │
└─────────────────────────────┘
```

the optimization zone (trop) handles combinatorial computation. results flow into the arithmetic zone (nebu) for verification via dual certificates. the binary zone (kuro) handles bitwise operations (hashing, commitments). each zone runs at native speed.

## the architectural principle

the right algebra for the right job:

- minimizing a sum? use tropical.
- multiplying field elements? use F_p.
- XORing bit vectors? use F_2.
- proving an optimization is correct? compute in tropical, verify in F_p.
- proving a hash is correct? compute in F_2, commit in F_p.

the proof system orchestrates all three, crossing boundaries only at sub-circuit boundaries where data naturally changes representation.

## see also

- [[tropical-semiring]] — the (min, +) algebra
- [[verification]] — how tropical results are verified in F_p
- [[applications]] — where tropical computation appears
