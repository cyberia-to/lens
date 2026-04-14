---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: modular arithmetic, congruence, residue classes, mod p
diffusion: 0.00016580159983383091
springs: 0.0001399793337241195
heat: 0.00016408495493343376
focus: 0.00015771159102083603
gravity: 1
density: 0.17
---

# modular arithmetic

arithmetic where numbers wrap around after reaching a modulus. the clock is the canonical example: 10 + 5 = 3 on a 12-hour clock. modular arithmetic is the foundation of all finite field computation.

## congruence

two integers a and b are congruent modulo n, written a ≡ b (mod n), when n divides their difference: n | (a − b). equivalently, a and b have the same remainder when divided by n.

```
17 ≡ 3 (mod 7)     because 7 | (17 − 3) = 14
−1 ≡ 6 (mod 7)     because 7 | (−1 − 6) = −7
2⁶⁴ ≡ 2³² − 1 (mod p)   the Goldilocks reduction identity
```

congruence is an equivalence relation: reflexive (a ≡ a), symmetric (a ≡ b implies b ≡ a), and transitive (a ≡ b and b ≡ c implies a ≡ c).

## residue classes

congruence partitions the integers into n equivalence classes, called residue classes. each class contains all integers with the same remainder mod n. the complete residue system {0, 1, ..., n − 1} picks one representative from each class.

for the Goldilocks field, the residue system is {0, 1, ..., p − 1} — every integer maps to exactly one element in this set. a u64 value v maps to its canonical representative: v if v < p, otherwise v − p.

## addition mod p

add two residues. if the result exceeds p, subtract p:

```
a + b mod p:
  s = a + b
  if s ≥ p: s = s − p
  return s
```

on a machine, "a + b" can overflow u64 when a + b ≥ 2⁶⁴. the overflow discards 2⁶⁴ from the true sum. the Goldilocks reduction identity recovers this:

```
2⁶⁴ ≡ ε (mod p)    where ε = 2³² − 1
```

so an overflow of 1 means the result is short by 2⁶⁴, which equals p + ε. since we work mod p, the missing 2⁶⁴ is replaced by adding ε to the truncated result.

this is the core insight of Goldilocks arithmetic: overflow correction is a single add of a 32-bit constant, not a division or multi-step reduction.

## subtraction mod p

subtract two residues. if the result underflows (a < b), add p:

```
a − b mod p:
  if a ≥ b: return a − b
  return a − b + p
```

on a machine, underflow wraps the u64 result by adding 2⁶⁴. since 2⁶⁴ = p + ε, the wrapped result is too large by 2⁶⁴ mod p = ε. subtracting ε corrects it.

addition adds ε on overflow. subtraction subtracts ε on underflow. the symmetry is exact.

## multiplication mod p

multiply two residues. the product can be as large as (p − 1)² ≈ 2¹²⁸, so reduction is more involved. see [[goldilocks]] for the full algorithm. the key idea: split the 128-bit product into 64-bit halves and apply the reduction identity to eliminate the high half.

## the integers mod p as a ring

the integers mod n always form a ring: addition and multiplication are well-defined, associative, commutative, and distributive. what distinguishes a field from a ring is the existence of multiplicative inverses.

mod 12: the element 4 has no inverse (4 × 3 = 0, so 4 is a zero divisor). the integers mod 12 are a ring, not a field.

mod p (p prime): every nonzero element has an inverse. the integers mod p are a field.

## Bézout's identity and inverses

for any integers a and n with gcd(a, n) = 1, Bézout's identity guarantees integers x and y such that ax + ny = 1. reducing mod n: ax ≡ 1 (mod n), so x = a⁻¹ mod n.

the extended Euclidean algorithm computes x directly. for prime fields, Fermat's little theorem provides a simpler path: a⁻¹ = a^(p−2) mod p. see [[goldilocks]].

## the Chinese Remainder Theorem

if gcd(m, n) = 1, then for any pair (a, b) there exists a unique x mod m·n such that x ≡ a (mod m) and x ≡ b (mod n). this is the Chinese Remainder Theorem (CRT).

CRT is the algebraic tool behind multi-modular arithmetic: perform computations modulo several small primes, then reconstruct the result modulo their product. this technique appears in:

- multi-precision arithmetic (splitting large numbers across machine words)
- error-correcting codes (Reed-Solomon)
- some proof systems that operate over multiple fields simultaneously

for the Goldilocks field specifically, CRT is less central — the field fits in a single machine word. but the theorem illuminates why the factorization of p − 1 matters: the multiplicative group F_p* decomposes (by CRT) into cyclic groups of prime-power order, one for each factor of p − 1.

## Fermat's little theorem

for any a ≠ 0 in F_p:

```
a^(p−1) ≡ 1 (mod p)
```

this is the fundamental theorem of prime-field arithmetic. consequences:

- **inversion**: a^(p−2) ≡ a⁻¹ (mod p)
- **roots of unity**: a^((p−1)/n) is an n-th root of unity when n | (p − 1)
- **primality witness**: if a^(p−1) ≢ 1, then p is composite (Miller-Rabin basis)

for the Goldilocks field: 7^(p−1) = 1, confirming both Fermat's theorem and that p is prime for the base 7.

## constant-time arithmetic

in cryptographic contexts, arithmetic must be constant-time: execution time and memory access patterns must not depend on the values being computed. variable-time code leaks secrets through timing side channels.

the Goldilocks field is naturally constant-time because:

- addition/subtraction: the overflow/underflow correction is a fixed sequence of operations (no branches on secret data needed — use conditional moves)
- multiplication: the u128 product and reduction are data-independent operations
- reduction: always the same sequence of adds, subtracts, and shifts

the only operation requiring care is inversion (exponentiation), which must use a constant-time square-and-multiply chain. see [[goldilocks]].

## see also

- [[finite-fields]] — the algebraic structure
- [[goldilocks]] — reduction, multiplication, inversion algorithms
- [[roots-of-unity]] — consequences of Fermat's little theorem