---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: quartic extension, F_p⁴, Fp4
diffusion: 0.0005830459640314197
springs: 0.00020214262964505256
heat: 0.0003395497564318387
focus: 0.00042007572219558795
gravity: 4
density: 0.78
---

# quartic extension field specification

the quartic extension F_{p⁴} over the Goldilocks field. provides 256-bit security for long-lived commitments and enables deeper recursion towers via the tower decomposition Fp4 = Fp2[v] / (v² − u).

## construction

```
F_{p⁴} = F_p[w] / (w⁴ − 7)
```

the irreducible polynomial is x⁴ − 7. it is irreducible over F_p because 7 is a quadratic non-residue (so x² − 7 is irreducible) and x⁴ − 7 has no factors of degree 1 or 2 over F_p. this matches the Plonky3 `BinomialExtensionField<Goldilocks, 4>`.

reduction rule: w⁴ = 7. higher powers reduce via:

```
w⁴ = 7
w⁵ = 7w
w⁶ = 7w²
w⁷ = 7w³
```

## tower decomposition

F_{p⁴} admits a natural tower structure:

```
F_{p⁴} = F_{p²}[v] / (v² − u)
```

where u is the Fp2 generator (u² = 7) and v = w. an Fp4 element (c₀, c₁, c₂, c₃) decomposes as:

```
A = c₀ + c₂·u   ∈ F_{p²}     (even coefficients)
B = c₁ + c₃·u   ∈ F_{p²}     (odd coefficients)

element = A + B·v
```

this tower view enables efficient inversion and Frobenius computation.

## elements

an element of F_{p⁴} is a 4-tuple (c₀, c₁, c₂, c₃) ∈ F_p⁴, representing c₀ + c₁·w + c₂·w² + c₃·w³.

```
element:  c₀ + c₁·w + c₂·w² + c₃·w³   where c₀, c₁, c₂, c₃ ∈ F_p
zero:     (0, 0, 0, 0)
one:      (1, 0, 0, 0)
```

the extension has p⁴ elements. the multiplicative group has order p⁴ − 1.

## arithmetic

all operations reduce to F_p operations. all algorithms are constant-time.

### addition

```
(a₀, a₁, a₂, a₃) + (b₀, b₁, b₂, b₃) = (a₀+b₀, a₁+b₁, a₂+b₂, a₃+b₃)
```

four F_p additions.

### subtraction

```
(a₀, a₁, a₂, a₃) − (b₀, b₁, b₂, b₃) = (a₀−b₀, a₁−b₁, a₂−b₂, a₃−b₃)
```

four F_p subtractions.

### multiplication

expand and reduce using w⁴ = 7:

```
mul(a, b):
  // schoolbook expansion produces degree-6 polynomial
  d₀ = a₀·b₀
  d₁ = a₀·b₁ + a₁·b₀
  d₂ = a₀·b₂ + a₁·b₁ + a₂·b₀
  d₃ = a₀·b₃ + a₁·b₂ + a₂·b₁ + a₃·b₀
  d₄ = a₁·b₃ + a₂·b₂ + a₃·b₁
  d₅ = a₂·b₃ + a₃·b₂
  d₆ = a₃·b₃

  // reduce: w⁴ = 7, w⁵ = 7w, w⁶ = 7w²
  c₀ = d₀ + 7·d₄
  c₁ = d₁ + 7·d₅
  c₂ = d₂ + 7·d₆
  c₃ = d₃
  return (c₀, c₁, c₂, c₃)
```

cost: 16 F_p multiplications + 3 mul-by-7 + 9 F_p additions. can be reduced to 9 F_p multiplications with 2-level Karatsuba.

### squaring

```
sqr(a):
  s₀ = a₀²
  s₁ = 2·a₀·a₁
  s₂ = a₁² + 2·a₀·a₂
  s₃ = 2·(a₀·a₃ + a₁·a₂)
  s₄ = a₂² + 2·a₁·a₃
  s₅ = 2·a₂·a₃
  s₆ = a₃²

  c₀ = s₀ + 7·s₄
  c₁ = s₁ + 7·s₅
  c₂ = s₂ + 7·s₆
  c₃ = s₃
  return (c₀, c₁, c₂, c₃)
```

cost: 10 F_p multiplications + 3 mul-by-7 + 6 F_p additions.

### conjugate (tower)

using the tower view element = A + B·v:

```
conj(c₀, c₁, c₂, c₃) = (c₀, −c₁, c₂, −c₃)
```

negate odd coefficients. two F_p negations.

### norm (to Fp2)

```
norm_fp2(a) = A² − u·B²
```

where A = (c₀, c₂), B = (c₁, c₃) in Fp2 representation, and u·B² means multiply by the Fp2 element (0, 1). this maps F_{p⁴} → F_{p²}.

### norm (to Fp)

```
norm(a) = norm_fp(norm_fp2(a))
```

compose the Fp4→Fp2 norm with the Fp2→Fp norm to get a base field element.

### inversion

tower-based inversion via Fp2 norm:

```
inv(a):
  A = (c₀, c₂)              // Fp2 element
  B = (c₁, c₃)              // Fp2 element
  N = A² − u·B²              // Fp2 norm (in Fp2)
  N_inv = inv_fp2(N)          // Fp2 inversion (1 Fp inv + ~4 Fp muls)
  conj_a = (A, −B)           // tower conjugate
  result = conj_a · N_inv    // Fp2 scalar multiplication
  return result
```

cost: 1 F_p inversion + ~18 F_p multiplications.

### Frobenius

the Frobenius endomorphism σ(x) = x^p acts on the basis element as:

```
σ(w) = w^p = 7^((p−1)/4) · w
```

the constant 7^((p−1)/4) = 2⁴⁸ = 0x0001000000000000. so:

```
frobenius(c₀, c₁, c₂, c₃) = (c₀, 2⁴⁸·c₁, −c₂, −2⁴⁸·c₃)
```

four F_p multiplications (two by constant, two negations).

## why quartic extension

two use cases:

1. **deeper recursion towers.** Fp4 = Fp2[v]/(v²−u) extends the quadratic tower. when a proof system needs multiple levels of recursive verification, each level can use a different extension degree. the tower Fp → Fp2 → Fp4 provides a clean algebraic ladder.

2. **256-bit security.** for long-lived commitments in [[bbg]] that must resist attacks beyond the 128-bit horizon, challenges from F_{p⁴} give 256-bit security margin.

## embedding

F_p embeds as (a, 0, 0, 0). F_{p²} embeds via the tower: (re, im) ↦ (re, 0, im, 0).

```
embed_fp(a) = (a, 0, 0, 0)
embed_fp2(re, im) = (re, 0, im, 0)
```

both embeddings preserve all arithmetic operations.

## see also

- [[field]] — base field specification
- [[fp2]] — quadratic extension F_{p²}
- [[fp3]] — cubic extension F_{p³}
- [[vectors]] § quartic extension — known-answer tests