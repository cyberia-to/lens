---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: batch inversion, Montgomery's trick, simultaneous inversion
diffusion: 0.00010722364868599256
springs: 0.0005931823714880819
heat: 0.0004631115060868785
focus: 0.0003241888370067924
gravity: 0
density: 0.32
---

# batch inversion specification

invert N field elements simultaneously using Montgomery's trick. cost: 1 inversion + 3(N−1) multiplications — amortized cost per element approaches 3 multiplications as N grows.

## algorithm

```
batch_inv(a[0..N]):
  // phase 1: accumulate prefix products
  prefix[0] = a[0]
  for i in 1..N:
    prefix[i] = prefix[i−1] · a[i]

  // phase 2: invert the product of all elements
  inv_all = inv(prefix[N−1])                   // single inversion

  // phase 3: propagate inverses backward
  for i in (N−1) down to 1:
    result[i] = inv_all · prefix[i−1]          // a[i]⁻¹ = (∏ a[j])⁻¹ · ∏_{j<i} a[j]
    inv_all = inv_all · a[i]                   // update running inverse
  result[0] = inv_all

  return result
```

## correctness

after phase 1: `prefix[i] = a[0] · a[1] · ... · a[i]`.

after phase 2: `inv_all = (a[0] · a[1] · ... · a[N−1])⁻¹`.

in phase 3, `inv_all` starts as (∏_{j=0}^{N−1} a[j])⁻¹. at each step i (iterating from N−1 down to 1):
- `result[i] = inv_all · prefix[i−1]` = (∏_{j=0}^{i} a[j])⁻¹ · (∏_{j=0}^{i−1} a[j]) = a[i]⁻¹
- `inv_all = inv_all · a[i]` updates inv_all to (∏_{j=0}^{i−1} a[j])⁻¹ for the next iteration

verification: `a[i] · result[i] = 1` for all i.

## cost analysis

| operation | count |
|-----------|-------|
| multiplications (phase 1) | N − 1 |
| inversions (phase 2) | 1 |
| multiplications (phase 3) | 2(N − 1) |
| **total multiplications** | **3(N − 1)** |
| **total inversions** | **1** |

a single inversion costs ~96 multiplications with an optimized addition chain, or ~125 with the naive square-and-multiply loop (see [[field]] § inversion). break-even vs. individual inversions (using optimized inversion):

```
N individual inversions: 96N multiplications
batch: 96 + 3(N−1) = 3N + 93 multiplications
```

batch wins for N ≥ 2. at N = 100: 393 vs. 9600 multiplications — a 24× speedup.

## zero handling

if any a[i] = 0, the product prefix[N−1] = 0 and the inversion fails. two strategies:

1. **caller guarantees non-zero** — simplest, appropriate when inputs are known non-zero (e.g., polynomial evaluation points).

2. **skip zeros** — replace zero elements with 1 in the prefix product, mark them, and set their output to 0:

```
batch_inv_safe(a[0..N]):
  is_zero[i] = (a[i] = 0)
  a'[i] = if is_zero[i] then 1 else a[i]
  result = batch_inv(a')
  for i in 0..N:
    if is_zero[i]: result[i] = 0
  return result
```

## usage

| system | use case | typical N |
|--------|----------|-----------|
| STARK prover | constraint evaluation denominators | 2¹⁸ − 2²⁴ |
| FRI | domain element inverses | 2¹⁶ − 2²⁰ |
| polynomial interpolation | Lagrange basis denominators | varies |

batch inversion is the single most impactful optimization for any system that inverts more than a handful of elements.

## see also

- [[field]] § inversion — the single-element inversion used in phase 2
- [[vectors]] § batch inversion — known-answer tests