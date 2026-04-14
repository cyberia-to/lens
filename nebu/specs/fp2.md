---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: quadratic extension, F_p², extension field, Fp2
diffusion: 0.0013651161170082113
springs: 0.00021566140233459524
heat: 0.0005858798587450669
focus: 0.0008644324509534864
gravity: 9
density: 0.96
---

# quadratic extension field specification

the quadratic extension F_{p²} over the Goldilocks field. required for 128-bit security in recursive STARK verification.

## construction

```
F_{p²} = F_p[u] / (u² − 7)
```

the irreducible polynomial is x² − 7. it is irreducible over F_p because 7 is a quadratic non-residue (see [[field]] § primitive root: 7^((p−1)/2) = p−1 ≠ 1).

## elements

an element of F_{p²} is a pair (a, b) ∈ F_p × F_p, representing a + b·u where u² = 7.

```
element:  a + b·u     where a, b ∈ F_p
zero:     0 + 0·u
one:      1 + 0·u
```

the extension has p² = (2⁶⁴ − 2³² + 1)² elements. the multiplicative group F_{p²}* has order p² − 1.

## arithmetic

all operations reduce to F_p operations. all algorithms are constant-time.

### addition

```
(a + bu) + (c + du) = (a + c) + (b + d)·u
```

two F_p additions.

### subtraction

```
(a + bu) − (c + du) = (a − c) + (b − d)·u
```

two F_p subtractions.

### multiplication

```
(a + bu)(c + du) = (ac + 7bd) + (ad + bc)·u
```

since u² = 7, the cross term bd·u² = 7bd folds into the real part.

cost: 4 F_p multiplications + 1 multiplication by 7 + 2 F_p additions. can be reduced to 3 multiplications using Karatsuba:

```
mul(x, y):                                     // x = (a, b), y = (c, d)
  v₀ = a · c
  v₁ = b · d
  re = v₀ + 7 · v₁                            // ac + 7bd
  im = (a + b) · (c + d) − v₀ − v₁            // ad + bc
  return (re, im)
```

3 F_p multiplications + 1 multiplication by 7 + 5 F_p additions/subtractions.

### squaring

```
(a + bu)² = (a² + 7b²) + 2ab·u
```

optimized form using the identity a² + 7b² = (a + b)(a + 7b) − 8ab:

```
sqr(x):                                        // x = (a, b)
  ab = a · b
  re = (a + b) · (a + 7 · b) − 8 · ab
  im = 2 · ab
  return (re, im)
```

2 F_p multiplications + multiplications by small constants.

### conjugate

```
conj(a + bu) = a − bu
```

the conjugate of u is −u. conjugation is the Frobenius automorphism:

```
(a + bu)^p = a + b · u^p = a + b · 7^((p−1)/2) · u = a − bu
```

since 7^((p−1)/2) = −1. one F_p negation.

### norm

```
norm(a + bu) = (a + bu) · conj(a + bu) = a² − 7b²
```

the norm maps F_{p²} → F_p. it is multiplicative: norm(xy) = norm(x) · norm(y).

2 F_p squarings + 1 multiplication by 7 + 1 F_p subtraction.

### inversion

```
inv(a + bu) = conj(a + bu) / norm(a + bu) = (a − bu) · (a² − 7b²)⁻¹
```

compute the norm (in F_p), invert it (single F_p inversion), then scale the conjugate.

```
inv(x):                                        // x = (a, b)
  n = a² − 7 · b²                             // norm, in F_p
  n_inv = inv(n)                               // F_p inversion
  return (a · n_inv, (−b) · n_inv)
```

cost: 1 F_p inversion + 2 F_p multiplications + 2 F_p squarings + small operations.

zero has no inverse. an element is zero iff both components are zero; equivalently, iff its norm is zero.

## why 128-bit security

the base field F_p has ~64 bits. a random challenge from F_p can be guessed with probability 2⁻⁶⁴. for 128-bit security, challenges must come from F_{p²}, where guessing probability is 2⁻¹²⁸.

in recursive STARK verification, the verifier samples FRI challenges. if challenges come from F_p alone, the soundness is ≤64 bits — below the 100-128 bit target. sampling from F_{p²} doubles the security margin.

## embedding

F_p embeds into F_{p²} as elements of the form (a, 0). all F_p operations are compatible:

```
embed(a) = (a, 0)
```

the embedding preserves addition, multiplication, and inversion. F_p arithmetic is a special case of F_{p²} arithmetic with b = d = 0.

## see also

- [[field]] § primitive root — 7 as quadratic non-residue (irreducibility proof)
- [[fp3]] — cubic extension F_{p³} for recursive composition
- [[fp4]] — quartic extension F_{p⁴}, tower Fp4 = Fp2[v]/(v²−u)
- [[ntt]] — NTT over F_p (base field); extension field NTT uses the same roots
- [[vectors]] § extension field — known-answer tests for F_{p²} arithmetic