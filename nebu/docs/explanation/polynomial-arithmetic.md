---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: polynomial arithmetic, polynomial evaluation, interpolation, Lagrange, Reed-Solomon
diffusion: 0.00010722364868599256
springs: 0.0006575138792249028
heat: 0.0004999265842514629
focus: 0.00035085130496075517
gravity: 0
density: 0.17
---

# polynomial arithmetic

polynomials over finite fields are the computational workhorse of modern proof systems, error-correcting codes, and secret sharing. every STARK proof is a statement about polynomials over F_p. the NTT makes these operations efficient.

## polynomials over F_p

a polynomial over F_p is a formal expression:

```
f(x) = c₀ + c₁x + c₂x² + ... + cₙxⁿ
```

where each coefficient cᵢ ∈ F_p. the degree of f is n (the highest power with nonzero coefficient). the set of all polynomials over F_p is written F_p[x].

arithmetic in F_p[x] follows the usual rules, with all coefficient operations performed in F_p:

- **addition**: add corresponding coefficients
- **multiplication**: convolve coefficients, then reduce each mod p
- **division**: polynomial long division (quotient and remainder)

## evaluation and interpolation

**evaluation**: given f(x) and a point a, compute f(a) ∈ F_p. Horner's method evaluates in n multiplications and n additions:

```
f(a) = c₀ + a(c₁ + a(c₂ + ... + a·cₙ))
```

**interpolation**: given n + 1 point-value pairs {(xᵢ, yᵢ)}, find the unique degree-≤n polynomial passing through all points. this is Lagrange interpolation:

```
f(x) = Σᵢ yᵢ · Lᵢ(x)

where Lᵢ(x) = Π_{j≠i} (x − xⱼ) / (xᵢ − xⱼ)
```

the denominators (xᵢ − xⱼ) require field inversions. when the evaluation points are roots of unity, the Lagrange basis simplifies dramatically, and the interpolation becomes the inverse NTT.

## the NTT as evaluation/interpolation

the forward NTT evaluates a polynomial at all N-th roots of unity:

```
NTT: (c₀, c₁, ..., c_{N-1}) → (f(1), f(ω), f(ω²), ..., f(ω^{N-1}))
```

the inverse NTT interpolates from values at roots of unity back to coefficients:

```
INTT: (f(1), f(ω), ..., f(ω^{N-1})) → (c₀, c₁, ..., c_{N-1})
```

both operations take O(N log N) — the same as evaluating at N arbitrary points would take O(N²) with Horner's method.

## polynomial multiplication (convolution)

multiplying two degree-N polynomials produces a degree-2N polynomial. in coefficient form, this is the convolution of coefficient vectors: O(N²).

using the NTT:

```
c = a · b:
  â = NTT(a, 2N)     // pad to length 2N, transform
  b̂ = NTT(b, 2N)
  ĉ = â ⊙ b̂           // pointwise multiply
  c = INTT(ĉ)
```

total cost: 3 NTTs of length 2N + 2N pointwise multiplications = O(N log N).

this is the foundation of STARK proving: the prover computes polynomial products repeatedly, and each product uses the NTT.

## polynomial division

dividing f(x) by g(x) produces a quotient q(x) and remainder r(x):

```
f(x) = q(x) · g(x) + r(x)    where deg(r) < deg(g)
```

in coefficient form, polynomial long division costs O(n · m) where n = deg(f) and m = deg(g).

for STARK proofs, the critical division is f(x) / Z(x) where Z(x) = x^N − 1 is the vanishing polynomial of a domain. this division verifies that f vanishes on the domain — a core proof step. the quotient q(x) = (f(x) − f_interp(x)) / Z(x) exists if and only if f agrees with the claimed values on the domain.

## polynomial commitment

a polynomial commitment scheme allows a prover to commit to a polynomial and later prove evaluations at arbitrary points. the FRI (Fast Reed-Solomon Interactive Oracle Proof) protocol used in STARKs commits via repeated folding:

```
commit(f):
  evaluate f on a domain D (using NTT)
  hash the evaluations (using hemera)
  build a Merkle tree of the hash values
  return the Merkle root
```

the Merkle tree is built from hemera hash outputs, which are Goldilocks field elements. no field conversion at the hash-to-commitment boundary.

## Reed-Solomon codes

a Reed-Solomon code encodes a message of k symbols as n symbols (n > k) such that any k of the n symbols suffice to recover the message. the encoding is polynomial evaluation: interpret the message as coefficients of a degree-(k−1) polynomial, evaluate at n distinct points.

```
encode(message[0..k]):
  f(x) = message[0] + message[1]·x + ... + message[k-1]·x^{k-1}
  return [f(α₀), f(α₁), ..., f(α_{n-1})]
```

decoding is interpolation from any k of the n values.

over the Goldilocks field, the evaluation points are roots of unity, making the encoding an NTT. Reed-Solomon codes over this field are used in STARK proofs for low-degree testing (the FRI protocol) and in erasure coding for data availability.

## the Schwartz-Zippel lemma

a fundamental tool in probabilistic proof systems:

> a nonzero polynomial of degree d over F_p has at most d roots.

corollary: if f(r) = 0 for a random r ← F_p, then either f is the zero polynomial or this event has probability ≤ d/p. for Goldilocks with p ≈ 2⁶⁴ and typical degrees d < 2³², this probability is negligible (< 2⁻³²).

this lemma justifies random evaluation as a test for polynomial identity: to check f = g, pick random r and verify f(r) = g(r). the soundness error is d/p.

## vanishing polynomials

the vanishing polynomial of a set S = {s₁, ..., sₖ} is:

```
Z_S(x) = (x − s₁)(x − s₂)···(x − sₖ)
```

Z_S(x) = 0 for all x ∈ S and Z_S(x) ≠ 0 for x ∉ S.

when S is the set of N-th roots of unity, Z_S(x) = x^N − 1. this polynomial is trivial to evaluate (one exponentiation and one subtraction) and plays a central role in STARK constraint checking.

## multilinear polynomials

a multilinear polynomial is a polynomial in multiple variables where each variable appears with degree at most 1:

```
f(x₁, x₂, ..., xₙ) = Σ_{S ⊆ [n]} c_S · Π_{i ∈ S} xᵢ
```

multilinear polynomials over F_p are the representation used in sumcheck-based proof systems (GKR, Spartan). a function on n boolean inputs corresponds uniquely to a multilinear polynomial in n variables.

## see also

- [[ntt-theory]] — the fast algorithm for evaluation/interpolation
- [[applications]] — where polynomial arithmetic appears in practice
- [[roots-of-unity]] — the evaluation domain