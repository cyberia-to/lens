# negacyclic NTT: fast ring multiplication

multiplication in R_q = F_p[x]/(x^n+1) is the critical bottleneck in lattice cryptography. naive polynomial multiplication is O(n^2). the negacyclic NTT reduces this to O(n log n) — and more importantly, to exactly 3n field multiplications for a full ring multiply.

## standard NTT recap

the number theoretic transform is the finite-field analogue of the FFT. given a polynomial a(x) of degree < n and an n-th root of unity omega (omega^n = 1), the NTT evaluates a at all n-th roots:

```
NTT(a)[j] = a(omega^j) = Σ_{i=0}^{n-1} a_i * omega^{ij}
```

the inverse NTT reconstructs coefficients from evaluations:

```
INTT(A)[i] = (1/n) * Σ_{j=0}^{n-1} A_j * omega^{-ij}
```

the key property: pointwise multiplication of NTT outputs corresponds to polynomial multiplication modulo x^n - 1 (cyclic convolution).

but jali needs reduction modulo x^n + 1 (negacyclic convolution). the standard NTT gives the wrong modulus.

## the negacyclic problem

cyclic convolution: `(a * b) mod (x^n - 1)` — wrapping terms are _added_.
negacyclic convolution: `(a * b) mod (x^n + 1)` — wrapping terms are _subtracted_.

the sign flip matters. x^n = +1 in cyclic; x^n = -1 in negacyclic. the standard NTT produces cyclic results. we need negacyclic.

## the twisting trick

the elegant solution: twist the input before applying a standard NTT, then untwist after the inverse. let psi be a primitive 2n-th root of unity (psi^{2n} = 1, psi^n = -1). define:

```
twist:   a'_i = a_i * psi^i
untwist: a_i  = a'_i * psi^{-i}
```

then: `standard_NTT(twist(a))` computes the evaluations of a(x) at the points psi^1, psi^3, psi^5, ..., psi^{2n-1}. these are the primitive 2n-th roots of unity — the zeros of x^n + 1.

pointwise multiplication at these evaluation points, followed by INTT and untwist, gives exactly the negacyclic convolution.

## why it works

the polynomial x^n + 1 factors over F_p (when p admits 2n-th roots) as:

```
x^n + 1 = (x - psi)(x - psi^3)(x - psi^5)...(x - psi^{2n-1})
```

the roots are the odd powers of psi. evaluating at these roots gives the CRT decomposition of R_q into n copies of F_p. the twist pre-multiplies the coefficients so that a standard NTT (which evaluates at even-spaced roots omega^j) actually evaluates at the odd-spaced roots psi^{2j+1}.

formally: `a(psi^{2j+1}) = Σ a_i psi^{i(2j+1)} = Σ (a_i psi^i) omega^{ij}` where omega = psi^2. the expression in parentheses is exactly the twisted coefficient a'_i.

## why Goldilocks is perfect

the Goldilocks prime p = 2^64 - 2^32 + 1 has multiplicative group order p - 1 = 2^32 * (2^32 - 1). the 2-adic valuation is 32, meaning Goldilocks has a primitive 2^32-th root of unity.

for the negacyclic NTT, we need a 2n-th root of unity:

```
n = 1024:  need 2048-th root.  2048 = 2^11.  11 <= 32.  available.
n = 2048:  need 4096-th root.  4096 = 2^12.  12 <= 32.  available.
n = 4096:  need 8192-th root.  8192 = 2^13.  13 <= 32.  available.
```

Goldilocks supports NTT degrees up to 2^31 — far beyond what lattice cryptography requires. the prime was essentially designed for NTT-heavy workloads.

compare with BN254 (a pairing-friendly prime): its 2-adic valuation is only 1. NTT over BN254 requires embedding into a much larger field. Goldilocks has 2^32 times more NTT headroom.

## the butterfly

the radix-2 decimation-in-time (DIT) butterfly is the atomic operation of the NTT:

```
butterfly(a, b, w):
  t = b * w          // 1 field multiply
  a' = a + t         // 1 field add
  b' = a - t         // 1 field sub (reuses a, t)
```

one multiplication, two additions. the NTT is n/2 * log2(n) butterflies, applied in stages with bit-reversed addressing.

## cost breakdown

```
operation                  field muls    field adds
────────────────────────   ──────────    ──────────
twist (n multiplies)       n             0
forward NTT                n/2 * log n   n * log n
pointwise multiply         n             0
inverse NTT                n/2 * log n   n * log n
untwist + scale by 1/n     n             0

total for ring multiply    3n + n log n  2n log n
```

at n = 1024: 3072 field multiplications + 20480 field additions. the "3n" shorthand counts the dominant multiplications.

for comparison, naive polynomial multiplication: n^2 = 1,048,576 multiplications. the NTT saves a factor of 340x at n = 1024.

## merged twisting

an optimization: merge the twist into the first NTT stage and the untwist into the last INTT stage. the twiddle factors in the first butterfly layer are multiplied by the twist factors. this saves 2n multiplications (the separate twist and untwist passes).

with merged twisting, a full ring multiply costs approximately n + n log n field multiplications — about 11,264 at n = 1024. the "3n" figure assumes twist and untwist are separate passes.

## lazy reduction

Goldilocks addition produces results in [0, 2p). many butterfly computations chain additions before the next multiplication. delaying modular reduction until the next multiply (which needs canonical inputs) saves reduction steps. this is "lazy reduction" — a key implementation optimization.

the Goldilocks modular reduction is fast (one conditional subtraction), so the savings are modest per operation but significant over millions of butterflies.

## see also

- [[polynomial-rings]] — the ring R_q that the NTT operates on
- [[lattice-security]] — why fast ring multiply matters for FHE
- [[fhe-overview]] — the application that drives NTT performance requirements
