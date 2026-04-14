# batch inversion specification

invert N tower field elements simultaneously using Montgomery's trick. cost: 1 inversion + 3(N-1) multiplications. the algorithm applies identically at every tower level.

## algorithm

```
batch_inv(a[0..N]):
  // phase 1: accumulate prefix products
  prefix[0] = a[0]
  for i in 1..N:
    prefix[i] = prefix[i-1] * a[i]

  // phase 2: invert the product of all elements
  inv_all = inv(prefix[N-1])                    // single inversion

  // phase 3: propagate inverses backward
  for i in (N-1) down to 1:
    result[i] = inv_all * prefix[i-1]           // a[i]^{-1}
    inv_all = inv_all * a[i]                    // update running inverse
  result[0] = inv_all

  return result
```

## correctness

after phase 1: `prefix[i] = a[0] * a[1] * ... * a[i]`.

after phase 2: `inv_all = (a[0] * a[1] * ... * a[N-1])^{-1}`.

in phase 3, at each step i (from N-1 down to 1):
- `result[i] = inv_all * prefix[i-1]` = (prod_{j=0}^{N-1} a[j])^{-1} * (prod_{j=0}^{i-1} a[j]) = (prod_{j=i}^{N-1} a[j])^{-1} * (prod_{j=i+1}^{N-1} a[j])^{-1}... more precisely:
- at the start of iteration i, `inv_all = (prod_{j=0}^{i} a[j])^{-1}` (after update)
- `result[i] = inv_all * prefix[i-1]` = (prod_{j=0}^{i} a[j])^{-1} * (prod_{j=0}^{i-1} a[j]) = a[i]^{-1}

verification: `a[i] * result[i] = 1` for all i.

## cost analysis

| operation | count |
|-----------|-------|
| multiplications (phase 1) | N - 1 |
| inversions (phase 2) | 1 |
| multiplications (phase 3) | 2(N - 1) |
| **total multiplications** | **3(N - 1)** |
| **total inversions** | **1** |

### comparison with individual inversions

the cost of a single tower-recursive inversion at level k depends on the level. let I(k) denote this cost in terms of base-level operations. for F₂¹²⁸ (level 7), I(7) is substantial. batch inversion amortizes it:

```
N individual inversions: N * I(k)
batch inversion:         I(k) + 3(N-1) * M(k)
```

where M(k) is the multiplication cost at level k. since I(k) >> M(k) at every level, batch wins for N >= 2.

| N | individual (units of I(k)) | batch (units of M(k)) | speedup (assuming I(k) = 50*M(k)) |
|---|---------------------------|----------------------|----------------------------------|
| 1 | 1 I(k) = 50 M(k) | 1 I(k) = 50 M(k) | 1x |
| 10 | 500 M(k) | 50 + 27 = 77 M(k) | 6.5x |
| 100 | 5000 M(k) | 50 + 297 = 347 M(k) | 14.4x |
| 1000 | 50000 M(k) | 50 + 2997 = 3047 M(k) | 16.4x |

## binary field specifics

in characteristic 2, addition = subtraction = XOR. the algorithm is identical to the prime field version. the only difference is that the underlying mul and inv operations use tower arithmetic (Karatsuba and tower-recursive inversion) instead of modular arithmetic.

## zero handling

if any a[i] = 0, the product prefix[N-1] = 0 and inv(0) is undefined. two strategies:

**1. caller guarantees nonzero** -- simplest. appropriate when inputs are known nonzero (e.g., evaluation domain points for FRI).

**2. skip zeros** -- replace zero elements with 1 in the prefix product:

```
batch_inv_safe(a[0..N]):
  is_zero[i] = (a[i] == 0)
  a'[i] = if is_zero[i] then ONE else a[i]
  result = batch_inv(a')
  for i in 0..N:
    if is_zero[i]: result[i] = ZERO
  return result
```

the definition 0^{-1} = 0 is non-standard but useful for batch operations where sparse zeros occur.

## usage in binary proving

| system | use case | typical N |
|--------|----------|-----------|
| binary PCS (zheng) | FRI evaluation denominators | 2^16 - 2^20 |
| binary STARK prover | constraint evaluation | 2^18 - 2^24 |
| polynomial interpolation | Lagrange basis denominators | varies |

## see also

- [inversion](inversion.md) -- the single-element inversion used in phase 2
- [field](field.md) -- tower field multiplication used in phases 1 and 3
- [vectors](vectors.md) -- test vectors for batch inversion
