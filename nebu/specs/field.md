---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: Goldilocks field, Goldilocks prime, F_p, field specification
diffusion: 0.0015825387348163517
springs: 0.00013365308300601137
heat: 0.0005946386474714702
focus: 0.0009502930218042611
gravity: 68
density: 0.92
---

# field specification

the Goldilocks prime field. arithmetic substrate for [[hemera]], [[trident]], [[nox]], and every computational domain in [[cyber]].

## the prime

```
p = 2⁶⁴ − 2³² + 1 = 0xFFFFFFFF00000001 = 18446744069414584321
```

the Goldilocks prime. the name comes from its structure: a 64-bit prime with a 32-bit "hole" that enables fast reduction.

## reduction identity

```
2⁶⁴ ≡ 2³² − 1 (mod p)
```

define the correction constant:

```
ε = 2³² − 1 = 0xFFFFFFFF
```

this is also `p.wrapping_neg()` in two's complement u64. every reduction uses ε — no division, no trial subtraction loops.

## field elements

a Goldilocks field element is an integer in the range [0, p). every element is represented as a canonical u64 in little-endian byte order.

canonical form: a u64 value `v` is canonical if `v < p`. non-canonical values (where `v ≥ p`) are reduced by subtracting p once.

equality: two elements are equal if and only if their canonical forms are identical.

## arithmetic

all operations are modular arithmetic over F_p. all algorithms are constant-time (no secret-dependent branches).

### addition

```
add(a, b):
  (sum, carry₁) = a + b                    // overflowing u64 add
  (sum, carry₂) = sum + carry₁ · ε         // correction for overflow
  if carry₂: sum = sum + ε                  // double overflow (rare)
  return sum
```

when the u64 addition overflows, the discarded 2⁶⁴ is replaced by ε = 2³² − 1. a second overflow can occur from the correction, handled identically.

the result may be non-canonical (in [p, 2⁶⁴)). canonicalization subtracts p if needed, but is deferred — intermediate results tolerate non-canonical form.

### subtraction

```
sub(a, b):
  (diff, borrow₁) = a − b                  // overflowing u64 sub
  (diff, borrow₂) = diff − borrow₁ · ε     // correction for underflow
  if borrow₂: diff = diff − ε               // double underflow (rare)
  return diff
```

underflow borrows 2⁶⁴, which equals p + ε. subtracting ε from the result corrects for the borrowed 2⁶⁴ modulo p. this is the mirror of addition — underflow subtracts ε where overflow adds ε.

### multiplication

```
mul(a, b):
  x = a × b                                // u128 full product
  x_lo = x[0:64]                           // low 64 bits
  x_hi = x[64:128]                         // high 64 bits
  x_hi_hi = x_hi >> 32                     // high 32 bits of x_hi
  x_hi_lo = x_hi & ε                       // low 32 bits of x_hi

  (t₀, borrow) = x_lo − x_hi_hi           // 2⁶⁴ → subtract x_hi_hi
  if borrow: t₀ = t₀ − ε                   // borrow correction

  t₁ = x_hi_lo × ε                         // 2³² → multiply by ε

  (result, carry) = t₀ + t₁                // combine
  return result + carry · ε                 // carry correction
```

the 128-bit product splits into high and low halves. the high 64 bits represent multiples of 2⁶⁴. the reduction identity replaces 2⁶⁴ with ε:

- `x_hi_hi` (bits 96–127): contributes `x_hi_hi · 2⁹⁶ ≡ x_hi_hi · ε · 2³² = x_hi_hi · (2⁶⁴ − 2³²) = x_hi_hi · (p − 1) ≡ −x_hi_hi` (mod p), hence the subtraction
- `x_hi_lo` (bits 64–95): contributes `x_hi_lo · 2⁶⁴ ≡ x_hi_lo · ε`, hence the multiplication

no division. no trial subtraction loop. three 64-bit operations after the u128 multiply.

### inversion

```
a⁻¹ = a^(p − 2) mod p
```

Fermat's little theorem. zero has no inverse — the caller must handle this.

p − 2 = 0xFFFFFFFEFFFFFFFF. in binary: all 64 bits set except bit 32. this gives a simple square-and-multiply loop:

```
inv(a):
  t = a
  for i in 62 down to 0:                      // process bits 62..0
    t = t²
    if i ≠ 32: t = t · a                      // bit 32 of p−2 is 0
  return t
```

63 squarings + 62 multiplications. an optimized addition chain exploits the Mersenne structure of ε = 2³² − 1:

```
inv(a):                                        // optimized chain
  e1  = a                                      // a^(2¹−1)
  e2  = e1² · e1                               // a^(2²−1)
  e4  = e2^(2²) · e2                           // a^(2⁴−1)
  e8  = e4^(2⁴) · e4                           // a^(2⁸−1)
  e16 = e8^(2⁸) · e8                           // a^(2¹⁶−1)
  e32 = e16^(2¹⁶) · e16                        // a^(2³²−1) = a^ε

  t   = e32^(2³²)                              // a^(ε·2³²) = a^(2⁶⁴−2³²)
  // p−2 = 2⁶⁴ − 2³² − 1. remaining: multiply by a^(−1).
  // instead, note p−2 = (2³²−2)·2³² + (2³²−1), so:
  //   a^(p−2) = (a^(2³²−2))^(2³²) · a^(2³²−1)
  //           = ((e32 · a⁻¹) · ...)  — circular.
  // the loop form above avoids this. the chain computes e32
  // in 31 squarings + 5 multiplications; the remaining 32 bits
  // of p−2 are processed by square-and-multiply. total: ~96 muls.
  ...
```

the exact addition chain is implementation-defined. the loop form is canonical; the chain form is an optimization.

### negation

```
neg(a):
  if a = 0: return 0
  return p − a
```

### exponentiation (S-box)

```
pow7(x):
  x² = x · x
  x³ = x² · x
  x⁴ = x² · x²
  x⁷ = x³ · x⁴
  return x⁷
```

4 multiplications. d = 7 is the minimum invertible exponent for this field: gcd(d, p−1) = 1 requires d coprime to p−1 = 2³² × 3 × 5 × 17 × 257 × 65537. d=2 fails (even). d=3 fails (divides p−1). d=5 fails (divides p−1). d=7 succeeds.

## primitive root

g = 7 is the smallest generator of the multiplicative group F_p*.

verification: a generator must satisfy g^((p−1)/q) ≠ 1 for every prime factor q of p−1. the prime factors of p−1 are {2, 3, 5, 17, 257, 65537}. all six checks pass for g = 7:

| q | 7^((p−1)/q) mod p | ≠ 1? |
|---|---|---|
| 2 | `0xFFFFFFFF00000000` (= p−1) | ✓ |
| 3 | ≠ 1 | ✓ |
| 5 | ≠ 1 | ✓ |
| 17 | ≠ 1 | ✓ |
| 257 | ≠ 1 | ✓ |
| 65537 | ≠ 1 | ✓ |

the q = 2 check is the Euler criterion. concrete values for q ∈ {3, 5, 17, 257, 65537} are verified by the reference implementation (see [[vectors]] § primitive root).

smaller candidates fail: 2^((p−1)/2) = 1, 3^((p−1)/2) = 1, 5^((p−1)/2) = 1, 6^((p−1)/2) = 1. these are quadratic residues, not generators.

g = 7 is used to derive all roots of unity for [[NTT]].

## properties

| property | value |
|---|---|
| prime | p = 2⁶⁴ − 2³² + 1 |
| size | 64 bits |
| characteristic | p (prime field) |
| order | p − 1 = 2³² × (2³² − 1) |
| factorization of p − 1 | 2³² × 3 × 5 × 17 × 257 × 65537 |
| two-adicity | 32 (largest k where 2ᵏ divides p−1) |
| primitive root | 7 (smallest generator of F_p*) |
| correction constant | ε = 2³² − 1 = 0xFFFFFFFF |

the high two-adicity (32) makes the field efficient for [[NTT]] (Number Theoretic Transform), which is why it is widely used in STARK proof systems.

## see also

- [[goldilocks]] — rationale for field choice: native u64, STARK compatibility, universal substrate, the double seven
- [[sqrt]] — square root (Tonelli-Shanks) and Legendre symbol
- [[batch]] — batch inversion (Montgomery's trick)
- [[fp2]] — quadratic extension F_{p²} for 128-bit security
- [[vectors]] — known-answer test vectors for all operations

## references

- [1] Plonky2: "Plonky2: Fast Recursive Arguments with PLONK and FRI." Polygon Zero Team, 2022.
- [2] Goldilocks field analysis in the context of STARK systems: documented in Polygon, Plonky3, and SP1 implementations.