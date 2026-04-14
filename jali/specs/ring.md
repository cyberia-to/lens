---
tags: jali, crypto
crystal-type: entity
crystal-domain: crypto
---
# ring — R_q element arithmetic

the core type: a polynomial of degree < n with coefficients in Goldilocks field, arithmetic modulo x^n + 1.

## type

```
RingElement {
  coeffs: [F_p; N]       // N = 1024, 2048, or 4096
}
```

an R_q element is a vector of N Goldilocks field elements. the ring structure comes from how multiplication wraps: coefficient multiplication is convolution modulo x^n + 1 (negacyclic).

## operations

```
add(a: &RingElement, b: &RingElement) → RingElement
  coefficient-wise: c[i] = a[i] + b[i]
  cost: N nebu adds

sub(a: &RingElement, b: &RingElement) → RingElement
  coefficient-wise: c[i] = a[i] - b[i]
  cost: N nebu subs

mul(a: &RingElement, b: &RingElement) → RingElement
  NTT(a) → pointwise multiply → INTT
  cost: 3N nebu muls (N per NTT, N pointwise, N per INTT)

neg(a: &RingElement) → RingElement
  coefficient-wise: c[i] = -a[i]
  cost: N nebu negs

scalar_mul(a: &RingElement, s: F_p) → RingElement
  coefficient-wise: c[i] = a[i] * s
  cost: N nebu muls

automorphism(a: &RingElement, k: usize) → RingElement
  x → x^{5^k}: permutes NTT evaluation points
  in NTT domain: permutation of coefficients (zero arithmetic cost)
  in coefficient domain: index remapping + conditional negation
  cost: N index operations (no field arithmetic)
```

## NTT representation

every RingElement has two forms:

```
coefficient form: [a_0, a_1, ..., a_{n-1}]  — natural for addition
NTT form:         [â_0, â_1, ..., â_{n-1}]  — natural for multiplication

NTT(coeffs) → ntt_form       cost: N nebu muls
INTT(ntt_form) → coeffs      cost: N nebu muls
```

elements can be stored in either form. lazy conversion: convert only when the operation requires the other form. multiply-heavy workloads stay in NTT form. add-heavy workloads stay in coefficient form.

## parameters

| parameter | value | meaning |
|-----------|-------|---------|
| n = 1024 | 128-bit LWE security at q ≈ 2^64 | standard FHE |
| n = 2048 | 192-bit security | high security |
| n = 4096 | 256-bit security | maximum security |

n must be a power of 2 (NTT requirement). n must divide 2^32 (Goldilocks two-adicity). all three values satisfy both constraints.

## Galois automorphisms

the Galois group of Φ_{2n} over F_p acts on R_q:

```
σ_k: x → x^{5^k mod 2n}

group size: n/2 automorphisms (for n = 1024: 512 automorphisms)
action on NTT form: permutation of evaluation points
```

automorphisms are the algebraic engine of key switching in FHE. they permute ciphertext slots without decryption.

## dependencies

- nebu: F_p field elements, NTT primitive roots (2^32-th root of unity)
