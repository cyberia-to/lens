---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: square root, Tonelli-Shanks, Legendre symbol, quadratic residue test
diffusion: 0.00010722364868599256
springs: 0.0005931823714880819
heat: 0.0004631115060868785
focus: 0.0003241888370067924
gravity: 0
density: 0.32
---

# square root specification

modular square root over the Goldilocks field. given a ∈ F_p, find r such that r² = a (mod p), or determine that no such r exists.

## Legendre symbol

the Legendre symbol determines whether a has a square root:

```
legendre(a) = a^((p−1)/2) mod p
```

| result | meaning |
|--------|---------|
| 0 | a = 0 |
| 1 | a is a quadratic residue (has a square root) |
| p−1 | a is a quadratic non-residue (no square root) |

exactly (p−1)/2 non-zero elements are quadratic residues. the other (p−1)/2 are non-residues.

this is one exponentiation: (p−1)/2 = 0x7FFFFFFF80000000. the same square-and-multiply chain as inversion, with a different exponent.

## Tonelli-Shanks algorithm

since p ≡ 1 (mod 4), not every element has a square root, and the simple formula r = a^((p+1)/4) does not apply (that works only when p ≡ 3 mod 4). the general algorithm is Tonelli-Shanks.

### setup

factor out powers of 2 from p − 1:

```
p − 1 = 2³² × s     where s = 2³² − 1 = ε
```

the two-adicity of p is 32. this means Tonelli-Shanks uses at most 32 iterations.

choose a known quadratic non-residue: z = 7 (confirmed by [[vectors]] § primitive root).

### algorithm

```
sqrt(n):
  if n = 0: return 0
  if legendre(n) = p−1: return ⊥             // no square root

  M = 32                                      // two-adicity
  c = 7^s mod p                               // 7^ε, a 2^M-th root of unity
  t = n^s mod p                               // n^s
  R = n^((s+1)/2) mod p                       // initial guess

  loop:
    if t = 1:
      if R > (p−1)/2: R = p − R              // canonical: pick r ≤ (p−1)/2
      return R
    find least i > 0 such that t^(2^i) = 1    // i < M always
    b = c^(2^(M−i−1)) mod p                   // adjustment factor
    M = i
    c = b² mod p
    t = t · c mod p
    R = R · b mod p
```

### cost

each iteration: one squaring chain (to find i), one exponentiation (to compute b), two multiplications. the initial setup costs one Legendre check + two exponentiations by s.

worst case: 32 iterations (when t has order exactly 2³²). typical case: far fewer.

### constant-time variant

the variable-time `find least i` loop leaks information about the input through timing. for constant-time operation, always perform all 32 iterations, using conditional moves to select the result:

```
sqrt_ct(n):
  // same setup
  for round in 0..32:
    // find i by squaring t up to 32 times
    // conditionally update (M, c, t, R) based on whether t^(2^i) = 1
```

this is required when the input is secret (e.g., in zero-knowledge circuits).

## sign convention

every non-zero quadratic residue has exactly two square roots: r and p − r. the canonical square root is the one where r ≤ (p−1)/2.

```
(p−1)/2 = 0x7FFFFFFF80000000
```

if r > (p−1)/2, return p − r instead.

## see also

- [[field]] § primitive root — g = 7 as quadratic non-residue
- [[field]] § inversion — Fermat exponentiation (same technique)
- [[vectors]] § square root — known-answer tests