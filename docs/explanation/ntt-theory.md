---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: NTT theory, Number Theoretic Transform, FFT over finite fields, polynomial multiplication
diffusion: 0.00010722364868599256
springs: 0.00043606680862567747
heat: 0.0003517925405885429
focus: 0.0002547903750484048
gravity: 0
density: 0.27
---

# NTT theory

the Number Theoretic Transform is the finite field analogue of the Fast Fourier Transform. it converts between coefficient and evaluation representations of polynomials in O(N log N) operations, enabling polynomial multiplication in O(N log N) instead of O(N²).

## polynomial representations

a polynomial f(x) of degree < N can be represented in two ways:

**coefficient form**: f(x) = c₀ + c₁x + c₂x² + ... + c_{N−1}x^{N−1}. stored as a vector of N coefficients.

**evaluation form**: f(x) described by its values at N distinct points: {(x₀, f(x₀)), (x₁, f(x₁)), ..., (x_{N−1}, f(x_{N−1}))}. stored as a vector of N values.

the NTT converts coefficient form → evaluation form (at the N-th roots of unity). the inverse NTT converts evaluation form → coefficient form.

## why evaluation form

in evaluation form, polynomial operations are pointwise:

| operation | coefficient form | evaluation form |
|-----------|-----------------|-----------------|
| addition | O(N) — add coefficients | O(N) — add values |
| multiplication | O(N²) — convolution | O(N) — multiply values |
| division | O(N²) — polynomial long division | O(N) — divide values |

multiplication is the critical case. multiplying two degree-N polynomials in coefficient form requires computing the convolution of their coefficients: O(N²) operations. in evaluation form, it is N independent field multiplications: O(N).

the catch: converting between forms costs O(N log N). so the full pipeline for multiplication is:

```
NTT(a) → â          O(N log N)
NTT(b) → b̂          O(N log N)
â · b̂ → ĉ           O(N)        (pointwise)
INTT(ĉ) → c         O(N log N)
```

total: O(N log N). this is the Schönhage-Strassen insight — polynomial (and integer) multiplication via transform.

## the DFT matrix

the NTT at length N evaluates f at the N-th roots of unity ω⁰, ω¹, ..., ω^{N−1}. in matrix form:

```
    ┌                              ┐   ┌    ┐       ┌    ┐
    │ 1    1      1      ...  1    │   │ c₀ │       │ f₀ │
    │ 1    ω      ω²     ... ω^{N-1}│ │ c₁ │       │ f₁ │
    │ 1    ω²     ω⁴     ... ω^{2(N-1)}│ c₂ │   =   │ f₂ │
    │ ⋮    ⋮      ⋮           ⋮    │   │ ⋮  │       │ ⋮  │
    │ 1  ω^{N-1} ω^{2(N-1)} ...   │   │c_{N-1}│   │f_{N-1}│
    └                              ┘   └    ┘       └    ┘
```

this is the DFT matrix F_N with entries F[j][k] = ω^{jk}. the inverse matrix has entries F⁻¹[j][k] = N⁻¹ · ω^{−jk}.

direct matrix-vector multiplication costs O(N²). the NTT computes the same result in O(N log N) by exploiting the structure of the DFT matrix.

## the butterfly decomposition

the key identity: ω^{N/2} = −1 (since ω is a primitive N-th root of unity and has order N). this means:

```
f(ω^k) = f_even(ω^{2k}) + ω^k · f_odd(ω^{2k})
f(ω^{k+N/2}) = f_even(ω^{2k}) − ω^k · f_odd(ω^{2k})
```

where f_even and f_odd are the polynomials formed from the even and odd coefficients of f. the values ω^{2k} for k = 0, ..., N/2 − 1 are exactly the (N/2)-th roots of unity.

this splits one length-N NTT into two length-N/2 NTTs plus N/2 butterfly operations. each butterfly:

```
a' = a + ω · b
b' = a − ω · b
```

one multiplication, two additions. the total work: N/2 butterflies per stage × log₂ N stages = (N/2) log₂ N multiplications and N log₂ N additions.

## Cooley-Tukey (decimation in time)

the forward NTT uses the Cooley-Tukey algorithm: split by even/odd indices (decimation in time), process bottom-up.

```
input:  bit-reversed order
output: natural order

for each stage s = 0, 1, ..., log₂(N) − 1:
  block size m = 2^(s+1)
  ω_m = primitive m-th root of unity
  for each block starting at j:
    for each butterfly in the block:
      t = ω^i · a[j + i + m/2]
      a[j + i + m/2] = a[j + i] − t
      a[j + i]       = a[j + i] + t
```

the bit-reversal permutation at the input reorders elements so that the butterfly's access pattern is sequential within each stage.

## Gentleman-Sande (decimation in frequency)

the inverse NTT uses the Gentleman-Sande algorithm: split by first/second half (decimation in frequency), process top-down.

```
input:  natural order
output: bit-reversed order (then permuted back)

for each stage s = log₂(N) − 1, ..., 1, 0:
  block size m = 2^(s+1)
  ω_m_inv = inverse of primitive m-th root
  for each block starting at j:
    for each butterfly in the block:
      u = a[j + i]
      v = a[j + i + m/2]
      a[j + i]       = u + v
      a[j + i + m/2] = ω^i · (u − v)
```

followed by bit-reversal permutation and scaling by N⁻¹.

the inverse NTT satisfies INTT(NTT(a)) = a. the scaling factor N⁻¹ exists because gcd(N, p) = 1 (N is a power of 2, p is odd).

## bit-reversal permutation

the decimation-in-time NTT expects input in bit-reversed order. for index i in a 2ᵏ-point NTT, the bit-reversed index is obtained by reversing the k least significant bits of i:

```
example (k = 3, N = 8):
  000 → 000  (0 → 0)
  001 → 100  (1 → 4)
  010 → 010  (2 → 2)
  011 → 110  (3 → 6)
  100 → 001  (4 → 1)
  101 → 101  (5 → 5)
  110 → 011  (6 → 3)
  111 → 111  (7 → 7)
```

the permutation is an involution (applying it twice returns to the original order) and can be computed in-place by swapping elements where i < bit_reverse(i).

## computational complexity

| operation | multiplications | additions | total for N = 2²⁰ |
|-----------|----------------|-----------|-------------------|
| forward NTT | N/2 · log₂ N | N · log₂ N | ~10M muls, ~20M adds |
| inverse NTT | N/2 · log₂ N | N · log₂ N | ~10M muls, ~20M adds |
| pointwise multiply | N | 0 | ~1M muls |

the NTT dominates: polynomial multiplication is 2× NTT + pointwise + INTT = 3× NTT cost.

at 5 ns per field multiplication (Goldilocks on modern x86), a 2²⁰-point NTT takes approximately 50 ms. this is the ballpark for one STARK proof layer.

## convolution theorem

the convolution theorem states:

```
NTT(a ∗ b) = NTT(a) · NTT(b)
```

where ∗ is convolution (polynomial multiplication in coefficient form) and · is pointwise multiplication. equivalently:

```
a ∗ b = INTT(NTT(a) · NTT(b))
```

this is why the NTT enables fast polynomial multiplication. it also works for cyclic convolution: when the polynomial modulus is x^N − 1, the NTT naturally computes the cyclic product. for negacyclic convolution (modulus x^N + 1, as in TFHE), a twist by half-roots is applied before the NTT.

## NTT-friendly fields

a field is NTT-friendly if it has high two-adicity (large k where 2ᵏ | p − 1). comparison:

| field | bits | two-adicity | max NTT length |
|-------|------|-------------|----------------|
| Goldilocks (2⁶⁴ − 2³² + 1) | 64 | 32 | 2³² |
| BabyBear (2³¹ − 2²⁷ + 1) | 31 | 27 | 2²⁷ |
| Mersenne31 (2³¹ − 1) | 31 | 1 | 2 |
| BN254 scalar field | 254 | 28 | 2²⁸ |

Goldilocks has the highest two-adicity among commonly used fields. Mersenne31 is nearly useless for NTT (two-adicity 1 means only length-2 NTTs). BabyBear is popular in newer STARK systems but has a smaller field (31-bit elements vs 64-bit).

## see also

- [[roots-of-unity]] — the algebraic foundation
- [[polynomial-arithmetic]] — operations that use the NTT
- [[goldilocks]] — the underlying field arithmetic
- [[applications]] — where the NTT is used in practice