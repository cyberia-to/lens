---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: NTT, Number Theoretic Transform, roots of unity
diffusion: 0.001352506934660513
springs: 0.00036690125119552303
heat: 0.0006783303485014985
focus: 0.0009219899123892011
gravity: 12
density: 0.89
---

# NTT specification

the Number Theoretic Transform over the Goldilocks field. the algebraic engine behind STARK proving, polynomial arithmetic, and [[TFHE]] operations in [[cyber]].

## roots of unity

the multiplicative group F_p* has order p − 1 = 2³² × (2³² − 1). the factor 2³² provides a principal 2³²-th root of unity:

```
ω = g^((p−1) / 2³²)
```

where g = 7 is the primitive root of F_p* (see [[field]] § primitive root). this root enables NTT of length up to 2³².

for NTT of length N = 2ᵏ (where k ≤ 32), the N-th root of unity is:

```
ω_N = g^((p−1) / N)
```

the inverse root ω_N⁻¹ = ω_N^(N−1) exists because N divides p−1. this guarantees the inverse NTT is well-defined.

## butterfly operation

the NTT butterfly is the core operation:

```
a' = a + ω · b
b' = a − ω · b
```

two field additions and one field multiplication. the twiddle factor `ω · b` is computed once and reused for both outputs:

```
butterfly(a, b, ω):
  t = ω · b
  return (a + t, a − t)
```

over Goldilocks, each field operation completes in one or two machine cycles — no multi-limb arithmetic, no Montgomery reduction.

the full NTT of length N = 2ᵏ performs N/2 × k butterflies. for N = 2³² (maximum length): 2³¹ × 32 = 2³⁶ butterflies.

## bit-reversal permutation

the decimation-in-time algorithm requires bit-reversed input ordering. for an N = 2ᵏ point NTT, index i maps to bit_reverse(i, k):

```
bit_reverse(i, k):
  result = 0
  for b in 0..k:
    result = result | (((i >> b) & 1) << (k − 1 − b))
  return result
```

the bit-reversal permutation is applied in-place before the forward NTT:

```
for i in 0..N:
  j = bit_reverse(i, k)
  if i < j:
    swap(a[i], a[j])
```

## forward NTT (Cooley-Tukey)

radix-2 decimation-in-time. input in bit-reversed order, output in natural order.

```
ntt(a[0..N], g):
  k = log2(N)
  bit_reverse_permute(a)
  for s in 0..k:
    m = 2^(s+1)
    ω_m = g^((p−1) / m)
    for j in (0..N).step_by(m):
      w = 1
      for i in 0..m/2:
        t = w · a[j + i + m/2]
        a[j + i + m/2] = a[j + i] − t
        a[j + i]       = a[j + i] + t
        w = w · ω_m
```

## inverse NTT (Gentleman-Sande)

radix-2 decimation-in-frequency. input in natural order, output in bit-reversed order (then permuted back).

```
intt(a[0..N], g):
  k = log2(N)
  for s in (0..k).rev():
    m = 2^(s+1)
    ω_m = g^((p−1) / m)
    ω_m_inv = ω_m^(m − 1)                   // = ω_m⁻¹, since ω_m^m = 1
    for j in (0..N).step_by(m):
      w = 1
      for i in 0..m/2:
        u = a[j + i]
        v = a[j + i + m/2]
        a[j + i]       = u + v
        a[j + i + m/2] = w · (u − v)
        w = w · ω_m_inv
  bit_reverse_permute(a)
  n_inv = p − (p−1)/N                       // N⁻¹ mod p, since N·(p−(p−1)/N) = (N−1)p+1 ≡ 1
  for i in 0..N:
    a[i] = a[i] · n_inv
```

the inverse NTT scales by N⁻¹ to satisfy the identity: `intt(ntt(a)) = a`.

N⁻¹ mod p exists because gcd(N, p) = 1 (N is a power of 2, p is odd).

## twiddle factor precomputation

for NTT of fixed length N, the twiddle factors ω_m^i can be precomputed and stored in a table of N − 1 elements (sum of m/2 entries across all k stages). this trades N − 1 field elements of memory for eliminating repeated exponentiations during the transform.

```
precompute_twiddles(N, g):
  k = log2(N)
  table = []
  for s in 0..k:
    m = 2^(s+1)
    ω_m = g^((p−1) / m)
    w = 1
    for i in 0..m/2:
      table.push(w)
      w = w · ω_m
  return table
```

the inverse NTT uses the conjugate twiddles: at each stage with root ω_m, replace ω_m^i with ω_m^(m−i) (i.e., the inverse root for that stage).

## hardware support

the [[GFP]] dedicates the `ntt` primitive to the butterfly operation. a single `ntt` instruction performs:

```
(a, b, ω) → (a + ω·b, a − ω·b)
```

one fused operation: one multiplication and two additions. the twiddle factor ω is a register operand. this eliminates the multiply-then-add pipeline stall that limits software NTT throughput.

## usage in cyber

| system | NTT role | typical length |
|--------|----------|----------------|
| STARK prover | polynomial evaluation/interpolation | 2¹⁸ − 2²⁴ |
| [[WHIR]] | polynomial commitment | 2¹⁶ − 2²⁰ |
| [[TFHE]] | ciphertext multiplication in R_p | 2¹⁰ − 2¹⁴ |
| [[hemera]] bootstrap | round constant generation (indirect) | — |