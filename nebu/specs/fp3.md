---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: cubic extension, F_p³, Fp3
diffusion: 0.0005508626584596788
springs: 0.00019911162716400538
heat: 0.0003362126806170614
focus: 0.00040240735350244804
gravity: 4
density: 0.71
---

# cubic extension field specification

the cubic extension F_{p³} over the Goldilocks field. required for recursive proof composition — verifying a STARK inside a STARK requires evaluation points from an extension with degree coprime to 2, separating inner and outer field domains.

## construction

```
F_{p³} = F_p[t] / (t³ − t − 1)
```

the irreducible polynomial is x³ − x − 1. it is irreducible over F_p because it has no roots in F_p: for any a ∈ F_p, a³ − a − 1 ≠ 0 mod p. a cubic over F_p is irreducible iff it has no roots.

reduction rule: t³ = t + 1. higher powers reduce via:

```
t³ = t + 1
t⁴ = t² + t
t⁵ = t³ + t² = t² + t + 1
```

## elements

an element of F_{p³} is a triple (c₀, c₁, c₂) ∈ F_p³, representing c₀ + c₁·t + c₂·t².

```
element:  c₀ + c₁·t + c₂·t²   where c₀, c₁, c₂ ∈ F_p
zero:     0 + 0·t + 0·t²
one:      1 + 0·t + 0·t²
```

the extension has p³ elements. the multiplicative group F_{p³}* has order p³ − 1.

## arithmetic

all operations reduce to F_p operations. all algorithms are constant-time.

### addition

```
(a₀ + a₁t + a₂t²) + (b₀ + b₁t + b₂t²) = (a₀ + b₀) + (a₁ + b₁)·t + (a₂ + b₂)·t²
```

three F_p additions.

### subtraction

```
(a₀ + a₁t + a₂t²) − (b₀ + b₁t + b₂t²) = (a₀ − b₀) + (a₁ − b₁)·t + (a₂ − b₂)·t²
```

three F_p subtractions.

### multiplication

```
(a₀ + a₁t + a₂t²)(b₀ + b₁t + b₂t²)
```

expand and reduce using t³ = t + 1:

```
mul(a, b):
  // schoolbook expansion
  d₀ = a₀·b₀
  d₁ = a₀·b₁ + a₁·b₀
  d₂ = a₀·b₂ + a₁·b₁ + a₂·b₀
  d₃ = a₁·b₂ + a₂·b₁
  d₄ = a₂·b₂

  // reduce: t³ = t + 1, t⁴ = t² + t
  c₀ = d₀ + d₃               // d₀ + d₃·(t+1)|_{const} = d₀ + d₃
  c₁ = d₁ + d₃ + d₄          // d₁ + d₃ + d₄·t|_{coeff of t}
  c₂ = d₂ + d₄               // d₂ + d₄
  return (c₀, c₁, c₂)
```

cost: 9 F_p multiplications + 6 F_p additions.

### squaring

```
sqr(a):
  s₀ = a₀²
  s₁ = 2·a₀·a₁
  s₂ = a₁² + 2·a₀·a₂
  s₃ = 2·a₁·a₂
  s₄ = a₂²

  c₀ = s₀ + s₃
  c₁ = s₁ + s₃ + s₄
  c₂ = s₂ + s₄
  return (c₀, c₁, c₂)
```

cost: 6 F_p multiplications + 6 F_p additions.

### norm

the norm N: F_{p³} → F_p is the determinant of the multiplication matrix. for a = c₀ + c₁t + c₂t²:

```
norm(a) = c₀³ + c₁³ + c₂³ − 3·c₀·c₁·c₂ + 2·c₀²·c₂ + c₀·c₂² − c₁·c₂² − c₀·c₁²
```

the norm is multiplicative: norm(a·b) = norm(a)·norm(b).

cost: ~15 F_p multiplications.

### inversion

```
inv(a) = adj(a) / norm(a)
```

compute the norm (in F_p), invert it (single F_p inversion), then multiply by the adjugate.

the adjugate adj(a) = (r₀, r₁, r₂) where:

```
inv(a):
  n = norm(a)                              // in F_p
  n_inv = inv(n)                           // F_p inversion

  // adjugate (cofactors of multiplication matrix)
  r₀ = (c₀² + c₂² + c₀·c₂ − c₁·c₂ − c₁²) · n_inv
  r₁ = (c₂ − c₀·c₁) · n_inv
  r₂ = (c₁² − c₀·c₂ − c₂) · n_inv
  return (r₀, r₁, r₂)
```

cost: 1 F_p inversion + ~15 F_p multiplications.

zero has no inverse. an element is zero iff all three components are zero; equivalently, iff its norm is zero.

## why cubic extension

recursive proof composition requires extension degrees coprime to 2. the base field F_p has two-adicity 2³² — every power-of-two domain is available for NTT. but when a STARK verifier runs inside another STARK, the inner and outer evaluation domains must be algebraically separated. F_{p³} provides this: degree 3 is coprime to all powers of 2, so inner-field challenges cannot collide with outer-field roots of unity.

## embedding

F_p embeds into F_{p³} as elements of the form (a, 0, 0). all F_p operations are compatible:

```
embed(a) = (a, 0, 0)
```

## see also

- [[field]] — base field specification
- [[fp2]] — quadratic extension F_{p²}
- [[fp4]] — quartic extension F_{p⁴}
- [[vectors]] § cubic extension — known-answer tests