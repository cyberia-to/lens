---
tags: jali
crystal-type: entity
crystal-domain: crypto
---
# jali reference

canonical specification for polynomial ring arithmetic R_q = F_p[x]/(x^n+1) over Goldilocks.

## what jali is

jali (जाली — lattice/mesh) is the fifth execution algebra for cyber. polynomial ring elements are structured vectors of n Goldilocks field elements with multiplication defined by convolution modulo x^n+1.

one R_q multiply = 3n F_p multiplies (via NTT). the 3072x gap (at n=1024) over scalar nebu arithmetic justifies a dedicated algebra — same criterion that separates kuro (32x gap for binary).

## spec pages

| page | defines |
|------|---------|
| [ring](ring.md) | R_q element type, add, sub, mul, automorphisms |
| [noise](noise.md) | noise tracking, bounds, growth estimation |
| [ntt](ntt.md) | negacyclic NTT transform, twisting trick, root selection |
| [sample](sample.md) | error distributions, seeded sampling, security parameters |
| [encoding](encoding.md) | RingElement to/from bytes, coefficient vs NTT form serialization |
| [automorphism](automorphism.md) | Galois group of Phi_{2n}, action on coefficient and NTT forms |
| [vectors](vectors.md) | test vectors for ring operations (placeholder) |

## algebra

```
R_q = F_p[x] / (x^n + 1)

  p = 2^64 - 2^32 + 1           Goldilocks prime (nebu)
  n = power of 2 (1024, 2048, 4096)
  x^n + 1 = cyclotomic polynomial Φ_{2n}

  element: vector of n Goldilocks field elements (coefficients)
  addition: coefficient-wise (n parallel nebu adds)
  multiplication: NTT → pointwise → INTT (3n nebu muls)
  automorphisms: x → x^{5^k} (Galois group of Φ_{2n})
```

via NTT, R_q ≅ F_p^n (n independent copies of Goldilocks). the ring structure (cyclotomic wrapping) is what makes Ring-LWE hard. the NTT decomposition is what makes computation fast.

## consumers

| consumer | what it uses | how |
|----------|-------------|-----|
| mudra::seal | R_q for Module-RLWE key encapsulation | polynomial matrix x vector + noise |
| mudra::veil | R_q for TFHE ciphertexts and bootstrapping | polynomial multiply, automorphisms, noise tracking |
| zheng (PCS₃) | ring-aware polynomial commitment | NTT batching, automorphism exploitation |
| nox jets | gadget_decompose, ntt_batch, key_switch | accelerated FHE operations |

## dependency

```
nebu (F_p scalar arithmetic + NTT roots of unity)
  ↓
jali (R_q polynomial ring arithmetic)
  ↓
mudra (seal, veil — protocols over jali)
zheng (PCS₃ — ring-aware proving)
nox (jets — accelerated execution)
```

jali depends only on nebu. no hemera dependency (hashing is the consumer's job). pure arithmetic.

## see also

- [nebu](https://github.com/mastercyb/nebu) — Goldilocks field arithmetic (the scalar field)
- jali is to R_q what nebu is to F_p
