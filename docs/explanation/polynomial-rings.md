# polynomial rings: structured vectors

jali operates in the polynomial ring R_q = F_p[x]/(x^n+1). this is the central algebraic object in lattice cryptography. understanding it requires seeing three things at once: polynomials, vectors, and convolution.

## polynomials as vectors

a polynomial of degree < n over F_p is a list of n coefficients:

```
a(x) = a_0 + a_1 x + a_2 x^2 + ... + a_{n-1} x^{n-1}
```

this is a vector (a_0, a_1, ..., a_{n-1}) in F_p^n. addition is component-wise — add corresponding coefficients. scalar multiplication scales every coefficient. the vector space structure is ordinary.

what makes it a _ring_ (not just a vector space) is multiplication. two polynomials multiply by distributing and collecting terms. the product of two degree-(n-1) polynomials has degree 2(n-1) — it has grown beyond n coefficients. to keep the result in the same space, we reduce modulo a polynomial of degree n.

## the reduction polynomial: x^n + 1

jali reduces modulo x^n + 1. this means: wherever x^n appears, replace it with -1. wherever x^{n+k} appears, replace it with -x^k.

```
x^n     = -1
x^{n+1} = -x
x^{n+2} = -x^2
...
x^{2n-1} = -x^{n-1}
```

the result: multiplication of two length-n vectors produces another length-n vector. this is negacyclic convolution — ordinary convolution with the wraparound terms negated.

```
(a * b)[k] = Σ_{i+j=k} a_i b_j  -  Σ_{i+j=k+n} a_i b_j
```

the first sum collects terms that land directly at position k. the second sum collects terms that wrapped around past degree n, picking up a minus sign from x^n = -1.

## why x^n + 1 and not x^n - 1

two reasons, one algebraic and one cryptographic.

**algebraically**, x^n + 1 is the 2n-th cyclotomic polynomial Phi_{2n} when n is a power of 2. cyclotomic polynomials are irreducible over Q, which means R_q = F_p[x]/(x^n+1) has a clean NTT decomposition: it splits into n copies of F_p (when p admits 2n-th roots of unity). this clean splitting is what makes the NTT possible.

by contrast, x^n - 1 factors as (x-1)(x+1)(x^2+1)... — it has the trivial factor (x-1), which creates a degenerate component. lattice problems over R/(x^n - 1) are weaker than over R/(x^n + 1) because of this factorization.

**cryptographically**, the Ring-LWE problem over cyclotomic rings has the best known security reduction. Lyubashevsky, Peikert, and Regev (2010) showed that Ring-LWE over Phi_{2n} is at least as hard as worst-case SIVP on ideal lattices. no comparable reduction exists for x^n - 1.

## the vector space view

strip away multiplication and look at R_q purely as a vector space. an element is:

```
[a_0, a_1, a_2, ..., a_{n-1}]    each a_i in F_p (Goldilocks)
```

at n = 1024, this is 1024 Goldilocks elements = 8 KiB. at n = 4096, it is 32 KiB. these are not large objects — they fit comfortably in L1 cache.

addition is vector addition: component-wise, perfectly parallel, n independent F_p additions. this is the fast operation.

multiplication is negacyclic convolution: quadratic naively (O(n^2) multiplications), but O(n log n) via the NTT. this is the expensive operation — and the one that jali exists to accelerate.

## the NTT view

the number theoretic transform decomposes R_q into n independent copies of F_p:

```
R_q = F_p[x]/(x^n+1) ≅ F_p × F_p × ... × F_p    (n copies)
```

via the Chinese Remainder Theorem. in this decomposed (NTT) form:

- addition is still component-wise (n independent adds)
- multiplication is also component-wise (n independent muls)

the NTT converts between the "polynomial" view (where addition is natural) and the "evaluation" view (where multiplication is natural). the cost of conversion — O(n log n) field operations — is paid once, then amortized over many multiplications.

this is why FHE implementations keep ciphertexts in NTT form: multiplication is the bottleneck, and NTT form makes it n independent scalar multiplications.

## why n must be a power of 2

the NTT requires roots of unity of the right order. specifically, the negacyclic NTT needs a primitive 2n-th root of unity in F_p. Goldilocks p = 2^64 - 2^32 + 1 has multiplicative group of order p - 1 = 2^32 * (2^32 - 1). the 2-adic valuation is 32, so primitive 2^k-th roots exist for k up to 32.

for the NTT, we need 2n | 2^32, which means n | 2^31. any power of 2 up to 2^31 works. the standard choices — n = 1024, 2048, 4096 — are all well within this range.

the power-of-2 constraint also enables the radix-2 FFT butterfly structure, which is the most efficient NTT implementation.

## the inner product connection

why does lattice cryptography use polynomial rings instead of plain vectors? the ring structure enables _structured_ lattice problems, where the public matrix A has a special form (a circulant-like structure from the ring multiplication). this structure:

1. **compresses keys**: instead of storing an n x n matrix (n^2 elements), store one ring element (n elements)
2. **speeds computation**: ring multiply via NTT is O(n log n), not O(n^2)
3. **preserves hardness**: Ring-LWE is conjectured as hard as unstructured LWE at the same dimension

the trade-off is theoretical: structured lattice problems _might_ be easier than unstructured ones. twenty years of cryptanalysis have not found a significant attack. the practical benefits (compact keys, fast operations) justify the theoretical risk.

## see also

- [[negacyclic-ntt]] — the NTT that makes ring multiplication fast
- [[lattice-security]] — why the ring structure is believed secure
- [[fhe-overview]] — how ring elements become ciphertexts
