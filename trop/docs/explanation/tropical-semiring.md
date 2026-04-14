# the tropical semiring: min replaces +

the tropical semiring replaces addition with min and multiplication with ordinary addition. the result is an algebraic structure that encodes optimization — shortest paths, minimum cost flows, scheduling — as linear algebra. trop exists because this encoding turns NP-hard verification into polynomial-time arithmetic.

## the axioms

a semiring has two operations (call them + and *) satisfying:

1. (S, +) is a commutative monoid with identity 0
2. (S, *) is a monoid with identity 1
3. * distributes over +
4. 0 annihilates: 0 * a = a * 0 = 0

the tropical semiring T = (R union {+inf}, min, +) satisfies all four:

| axiom | tropical add (min) | tropical mul (+) |
|-------|-------------------|-----------------|
| closure | min(a, b) in T | a + b in T |
| associativity | min(a, min(b, c)) = min(min(a, b), c) | (a + b) + c = a + (b + c) |
| commutativity | min(a, b) = min(b, a) | a + b = b + a |
| identity | +inf (min(a, +inf) = a) | 0 (a + 0 = a) |
| distributivity | a + min(b, c) = min(a + b, a + c) | -- |

the critical check is distributivity: a + min(b, c) = min(a + b, a + c). adding a constant to both sides of a min does not change which side wins. this is why shortest-path computations respect the algebra.

## what is missing: no additive inverse

in a ring, every element a has an additive inverse -a such that a + (-a) = 0. in the tropical semiring, this would require: for every a, there exists x with min(a, x) = +inf. but min(a, x) <= a for all x, so min(a, x) = +inf only if a = +inf. no finite element has an additive inverse.

this is not a limitation — it is the point. the absence of additive inverses is what makes the tropical semiring model optimization rather than arithmetic. you cannot "undo" a minimum the way you can undo addition. optimization is irreversible.

## idempotency: a + a = a

tropical addition is idempotent: min(a, a) = a. this never holds in a field (where a + a = 2a, and 2a = a implies a = 0). idempotency means tropical "summation" is projection to the minimum, not accumulation.

consequences:
- tropical linear combinations collapse: min(a, a, a, ..., a) = a regardless of repetition
- tropical rank is not determined by linear independence in the classical sense
- tropical eigenvalues measure cycle weights, not scaling factors

## the trop encoding

trop represents tropical elements as u64 values:
- finite elements: u64 values in [0, u64::MAX - 1]
- infinity (+inf): u64::MAX (the tropical zero)
- tropical one: 0 (the multiplicative identity)

tropical addition: min(a, b) — one comparison.
tropical multiplication: a.saturating_add(b) — one addition with overflow to infinity.

```rust
Tropical(3).add(Tropical(7))  // = Tropical(3)  — min(3, 7)
Tropical(3).mul(Tropical(7))  // = Tropical(10) — 3 + 7
Tropical::INF.add(Tropical(5)) // = Tropical(5) — min(+inf, 5)
Tropical::INF.mul(Tropical(5)) // = Tropical::INF — +inf + 5 = +inf
```

the u64::MAX sentinel is the absorbing element for mul (inf + anything = inf) and the identity for add (min(inf, x) = x). both properties are essential for correct matrix algebra.

## naming: why "tropical"

the name honors Imre Simon, a Brazilian mathematician who studied the (min, +) semiring in the context of automata theory. "tropical" refers to Brazil. the name was coined by French mathematicians in the 1990s and stuck, despite being geographically whimsical rather than mathematically descriptive.

alternative names in the literature: (min, +) algebra, min-plus algebra, schedule algebra, path algebra. they all refer to the same structure.

## the dual: (max, +)

swapping min for max gives the dual tropical semiring. it is isomorphic to (min, +) via negation: max(a, b) = -min(-a, -b). the dual appears in longest-path problems, max-plus spectral theory, and auction theory.

trop implements (min, +) directly. the (max, +) dual is obtained by negating inputs, computing in (min, +), and negating the output. one implementation covers both semirings.

## see also

- [[matrix-algebra]] — how tropical matrices encode shortest paths
- [[kleene-star]] — the all-pairs shortest-path closure
- [[f2-fp-trop]] — comparing tropical arithmetic with F₂ and F_p
