---
tags: jali, crypto
crystal-type: entity
crystal-domain: crypto
---
# ntt — negacyclic number theoretic transform

the NTT converts between coefficient form and evaluation form in R_q = F_p[x]/(x^n+1). multiplication in evaluation form is pointwise — n independent F_p multiplications instead of O(n²) convolution.

## the problem

naive polynomial multiplication in R_q requires convolving two length-n coefficient vectors modulo x^n+1. cost: O(n²) F_p multiplications. at n = 1024, that is ~1 million operations.

the NTT reduces this to O(n log n) — roughly 3n total F_p multiplications for a full multiply (NTT + pointwise + INTT).

## negacyclic vs standard NTT

the standard NTT computes evaluation at n-th roots of unity: the points ω^0, ω^1, ..., ω^{n-1} where ω^n = 1. this gives polynomial multiplication modulo x^n - 1 (cyclic convolution).

jali needs modular reduction by x^n + 1 (negacyclic convolution). the negacyclic NTT evaluates at the _twisted_ points: ψ^1, ψ^3, ψ^5, ..., ψ^{2n-1} where ψ is a primitive 2n-th root of unity (ψ^{2n} = 1, ψ^n = -1).

## the twisting trick

instead of implementing a separate negacyclic NTT from scratch, apply a pre-processing twist:

```
forward (coefficient → NTT):
  1. multiply coefficient a[i] by ψ^i        (twist)
  2. apply standard radix-2 NTT of length n  (butterfly network)

inverse (NTT → coefficient):
  1. apply standard radix-2 INTT of length n
  2. multiply coefficient a[i] by ψ^{-i}     (untwist)
```

the twist absorbs the x^n + 1 reduction into the root selection. after twisting, standard NTT butterflies produce the correct negacyclic result.

## root of unity selection

Goldilocks p = 2^64 - 2^32 + 1 has multiplicative group of order p - 1 = 2^64 - 2^32 = 2^32 * (2^32 - 1).

the 2-adic valuation of p - 1 is 32: the largest power of 2 dividing p - 1 is 2^32. this means Goldilocks has a primitive 2^32-th root of unity.

```
g = generator of F_p*
ω_{2^k} = g^{(p-1)/2^k}         k-th root of unity (2^k-th)

for n = 1024:  ψ = ω_{2048} = g^{(p-1)/2048}     2n-th root
for n = 2048:  ψ = ω_{4096} = g^{(p-1)/4096}     2n-th root
for n = 4096:  ψ = ω_{8192} = g^{(p-1)/8192}     2n-th root
```

all three parameter sets require 2n | 2^32, which holds since 2n <= 8192 = 2^13 << 2^32. Goldilocks has abundant room for negacyclic NTT at these degrees.

## twiddle table

precompute and store:

```
twist[i] = ψ^i                for i in 0..n       (twisting factors)
twiddle[j] = ω_n^{bit_rev(j)} for j in 0..n/2     (butterfly twiddle factors)
inv_twist[i] = ψ^{-i}         for i in 0..n       (untwisting factors)
```

total storage: 2.5n Goldilocks elements. at n = 4096 this is 80 KiB — fits in L1 cache.

## cost model

```
operation              F_p muls    F_p adds    total (n = 1024)
─────────────────────  ────────    ────────    ─────────────────
twist (pre-multiply)   n           0           1024
NTT butterflies        n/2 log n   n log n     5120 + 10240
pointwise multiply     n           0           1024
INTT butterflies       n/2 log n   n log n     5120 + 10240
untwist + scale        n           0           1024

full ring multiply     ~3n muls                ~3072
```

the "3n muls" approximation counts the dominant cost. adds are cheaper and typically overlapped with muls in the pipeline.

## in-place butterfly

the radix-2 DIT (decimation-in-time) butterfly:

```
butterfly(a, b, w):
  t = b * w
  (a, b) = (a + t, a - t)
```

one F_p multiply, two F_p adds. the entire NTT is n/2 * log₂(n) butterflies applied in bit-reversal order.

## see also

- [ring](ring.md) — the RingElement type and operations that use NTT
- [encoding](encoding.md) — serialization of NTT-form elements
- [automorphism](automorphism.md) — automorphisms are permutations in NTT form
