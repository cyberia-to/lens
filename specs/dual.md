# (max, +) dual semiring

## definition

the dual tropical semiring T' = (R ∪ {-∞}, ⊕', ⊗') where:

- dual tropical addition: a ⊕' b = max(a, b)
- dual tropical multiplication: a ⊗' b = a + b (standard addition)

| property | (max, +) |
|----------|----------|
| additive identity | -∞ (max(a, -∞) = a) |
| multiplicative identity | 0 (a + 0 = a) |
| idempotent | yes: max(a, a) = a |
| commutative | yes |
| distributive | a + max(b,c) = max(a+b, a+c) |

## duality via negation

the (min, +) and (max, +) semirings are isomorphic via negation:

```
φ: T → T'
φ(a) = -a
```

this transforms:
- min(a, b) → -min(a, b) = max(-a, -b)
- a + b → (-a) + (-b) = -(a + b), then negate back

so any (min, +) computation on values {a_i} is equivalent to a (max, +) computation on {-a_i}.

## F_p negation

over Goldilocks F_p, negation is: neg(a) = p - 1 - a (mapping the value range [0, p-2] to itself, with p-1 reserved as the infinity sentinel).

the sentinel maps correctly:
- (min, +) infinity: p-1 maps to neg(p-1) = 0
- (max, +) identity -∞ is represented as 0 in the dual encoding

## when to use which

| semiring | natural problems |
|----------|-----------------|
| (min, +) | shortest path, minimum assignment, min-cost flow |
| (max, +) | longest path, maximum reliability, max-plus spectral theory |

most optimization problems have a natural formulation in one semiring. the duality means trop only needs to implement (min, +) — any (max, +) computation is obtained by negating inputs, computing in (min, +), and negating the output.

## matrix duality

for matrix A in (max, +), define A' with A'_{ij} = -A_{ij}. then:

- (max, +) matmul of A, B = negation of (min, +) matmul of A', B'
- (max, +) eigenvalue of A = negation of (min, +) eigenvalue of A'
- (max, +) determinant of A = negation of (min, +) determinant of A'

one implementation covers both semirings.
