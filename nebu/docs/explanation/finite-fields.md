---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: finite field, prime field, field axioms, GF(p)
diffusion: 0.00010722364868599256
springs: 0.00039984814138527064
heat: 0.00031880238603150676
focus: 0.00023732674396487576
gravity: 0
density: 0.58
---

# finite fields

a finite field is a set with finitely many elements where addition, subtraction, multiplication, and division (except by zero) all work and produce elements that stay in the set. the integers are infinite and have no division (5/3 is not an integer). the rationals have division but are infinite. a finite field has both: closure and finitude.

## the axioms

a field is a set F with two operations (+, ×) satisfying:

| axiom | addition | multiplication |
|-------|----------|----------------|
| closure | a + b ∈ F | a × b ∈ F |
| associativity | (a + b) + c = a + (b + c) | (a × b) × c = a × (b × c) |
| commutativity | a + b = b + a | a × b = b × a |
| identity | a + 0 = a | a × 1 = a |
| inverse | a + (−a) = 0 | a × a⁻¹ = 1 (a ≠ 0) |
| distributivity | a × (b + c) = a × b + a × c | |

these are the same axioms as the real numbers. the difference: a finite field has finitely many elements.

## existence and uniqueness

finite fields exist only for prime powers: q = pⁿ where p is prime and n ≥ 1. there is exactly one finite field (up to isomorphism) for each such q, written GF(q) or F_q.

- GF(2): the field {0, 1} with arithmetic mod 2
- GF(7): the field {0, 1, 2, 3, 4, 5, 6} with arithmetic mod 7
- GF(p): for any prime p, the integers mod p form a field
- GF(p²): extension fields — elements are polynomials over GF(p), reduced modulo an irreducible polynomial

the Goldilocks field is GF(p) where p = 2⁶⁴ − 2³² + 1. this is a prime field (n = 1), so every element is simply an integer in [0, p).

## why primes

the integers mod n form a field if and only if n is prime. the reason: in a field, every nonzero element must have a multiplicative inverse. if n is composite, say n = a × b where 1 < a, b < n, then a has no inverse — because a × b = 0 mod n, meaning a is a zero divisor.

example: mod 6, the element 2 has no inverse. 2 × 1 = 2, 2 × 2 = 4, 2 × 3 = 0, 2 × 4 = 2, 2 × 5 = 4. no element produces 1. the integers mod 6 are not a field.

example: mod 7, every nonzero element has an inverse: 1⁻¹ = 1, 2⁻¹ = 4, 3⁻¹ = 5, 4⁻¹ = 2, 5⁻¹ = 3, 6⁻¹ = 6. the integers mod 7 are a field.

## the Goldilocks field concretely

```
p = 2⁶⁴ − 2³² + 1 = 18446744069414584321
```

the field F_p = {0, 1, 2, ..., p − 1}. every element fits in a u64. the four operations:

- **addition**: add two u64 values, reduce mod p
- **subtraction**: subtract two u64 values, reduce mod p
- **multiplication**: multiply two u64 values (producing a u128), reduce mod p
- **division**: multiply by the inverse (computed via Fermat's little theorem)

the special structure of p makes "reduce mod p" extremely fast — see [[goldilocks]].

## field characteristic

the characteristic of a field is the smallest positive integer n such that adding 1 to itself n times yields 0. for GF(p), the characteristic is p. this means:

```
1 + 1 + 1 + ... + 1  (p times)  =  0
```

over the Goldilocks field, adding 1 to itself 18446744069414584321 times wraps back to 0.

the characteristic determines which identities from ordinary arithmetic survive. for example, in characteristic 2, a + a = 0 for every element (so a = −a). in the Goldilocks field (odd characteristic), a ≠ −a unless a = 0.

## the multiplicative group

the nonzero elements {1, 2, ..., p − 1} form a group under multiplication, written F_p*. this group has order p − 1 and is always cyclic — there exists a generator g such that every nonzero element is a power of g.

for the Goldilocks field:

```
p − 1 = 2³² × (2³² − 1) = 2³² × 3 × 5 × 17 × 257 × 65537
```

the generator g = 7 produces every nonzero element: {7⁰, 7¹, 7², ..., 7^(p−2)} = {1, 2, ..., p − 1}. see [[roots-of-unity]] for how this cyclic structure enables the NTT.

## extension fields

GF(p²) extends GF(p) by adjoining a root of an irreducible quadratic. elements are pairs (a, b) representing a + b·α where α² is irreducible over GF(p). arithmetic follows polynomial rules with reduction modulo the minimal polynomial.

for the Goldilocks field: F_{p²} = F_p[u] / (u² − 7), where 7 is a quadratic non-residue. elements are a + b·u with u² = 7. multiplication: (a + bu)(c + du) = (ac + 7bd) + (ad + bc)u. inversion reduces to a single F_p inversion via the norm: (a + bu)⁻¹ = (a − bu) / (a² − 7b²). see [[fp2]] for the full specification.

extension fields matter for recursive STARK verification — the base field gives ~64 bits of security, while sampling FRI challenges from F_{p²} provides 128-bit security. they also appear in pairing-based cryptography (BN254, BLS12-381) and quantum simulation where unitary matrices live over F_{p²}.

most operations in [[hemera]], STARK proving, and polynomial arithmetic happen in F_p directly. F_{p²} is used specifically where the security margin requires it.

## see also

- [[modular-arithmetic]] — how arithmetic mod p actually works
- [[goldilocks]] — why this prime, how its reduction works
- [[roots-of-unity]] — the cyclic structure of F_p*
- [[fp2]] — F_{p²} construction and arithmetic