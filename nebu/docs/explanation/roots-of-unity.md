---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: roots of unity, cyclic group, primitive root, generator, twiddle factors
diffusion: 0.00010722364868599256
springs: 0.00007019991600688145
heat: 0.00003419142694206788
focus: 0.00008151008453347325
gravity: 0
density: 0
---

# roots of unity

roots of unity are elements that become 1 when raised to some power. they are the algebraic foundation of the NTT — the finite field analogue of the FFT. understanding roots of unity is understanding why polynomial arithmetic over finite fields is fast.

## definition

an n-th root of unity is an element ω such that ω^n = 1. a primitive n-th root of unity additionally satisfies: ω^k ≠ 1 for 0 < k < n. the primitive root generates all n-th roots as its powers: {1, ω, ω², ..., ω^(n−1)}.

over the complex numbers, the n-th roots of unity are e^(2πik/n) for k = 0, ..., n − 1 — equally spaced points on the unit circle. over a finite field, there is no circle, but the algebraic properties are identical.

## when do n-th roots exist?

in F_p, an n-th root of unity exists if and only if n divides p − 1. the multiplicative group F_p* is cyclic of order p − 1, so it contains elements of every order dividing p − 1 (and no others).

for the Goldilocks field:

```
p − 1 = 2³² × 3 × 5 × 17 × 257 × 65537
```

n-th roots of unity exist for every n dividing this number. in particular, 2ᵏ-th roots exist for all k ≤ 32. this is the two-adicity — the largest power of 2 dividing p − 1.

two-adicity 32 means the Goldilocks field supports NTTs of length up to 2³² (about 4 billion elements). this is far more than needed for any practical STARK proof.

## the primitive root (generator)

a primitive root (or generator) of F_p* is an element g whose powers produce every nonzero element: {g⁰, g¹, ..., g^(p−2)} = F_p*. the generator has order p − 1 — the maximum possible.

for Goldilocks, g = 7. verification: g is a generator if and only if g^((p−1)/q) ≠ 1 for every prime factor q of p − 1. the prime factors are {2, 3, 5, 17, 257, 65537}. checking the most discriminating factor:

```
7^((p−1)/2) = p − 1 ≠ 1   ✓  (7 is a quadratic non-residue)
```

smaller candidates fail: 2, 3, 5, 6 all satisfy a^((p−1)/2) = 1, making them quadratic residues (and therefore not generators).

## constructing n-th roots from the generator

given the generator g of order p − 1, an n-th root of unity is:

```
ω_n = g^((p−1)/n)
```

this element has order exactly n (when n | p − 1). proof: ω_n^n = g^(p−1) = 1, and for any k < n, ω_n^k = g^(k(p−1)/n) ≠ 1 because k(p − 1)/n is not a multiple of p − 1.

for NTT of length N = 2ᵏ:

```
ω_N = 7^((p−1) / N) = 7^((p−1) / 2^k)
```

the inverse root is ω_N^(N−1) = ω_N⁻¹, needed for the inverse NTT.

## the discrete logarithm

given g and y = g^x, finding x is the discrete logarithm problem. this problem is computationally hard for large primes — the basis of Diffie-Hellman key exchange and related cryptographic protocols.

for the Goldilocks field, the discrete log is not directly relevant to its use in hashing and proving. but it explains why the generator g = 7 is special: it is the structural anchor from which all other roots are derived by exponentiation.

## quadratic residues and non-residues

an element a is a quadratic residue if a = b² for some b. the Euler criterion: a is a QR if and only if a^((p−1)/2) = 1. elements with a^((p−1)/2) = p − 1 are non-residues.

exactly half of the nonzero elements are quadratic residues. the QRs form a subgroup of index 2 in F_p*.

this matters for:
- **square roots**: if a is a QR, a^((p+1)/4) is a square root (when p ≡ 3 mod 4). Goldilocks has p ≡ 1 mod 4, so the Tonelli-Shanks algorithm is needed instead — see [[sqrt]] for the full specification.
- **generator verification**: a generator must be a non-residue (since it generates the full group, not just the QR subgroup).

## the subgroup lattice

the divisors of p − 1 form a lattice of subgroups of F_p*. for Goldilocks:

```
p − 1 = 2³² × 3 × 5 × 17 × 257 × 65537

subgroups of order 2^k (k = 0, 1, ..., 32):
  trivial {1} ⊂ {±1} ⊂ ... ⊂ (2^32-th roots) ⊂ F_p*
```

each 2ᵏ-th root of unity generates a subgroup of order 2ᵏ. these nested subgroups are what the NTT's recursive halving exploits: at each stage of the butterfly, the twiddle factors come from the next-larger subgroup.

## twiddle factors

in the NTT, twiddle factors are the roots of unity used in butterfly operations:

```
butterfly(a, b, ω):
  return (a + ω·b, a − ω·b)
```

at stage s of a 2ᵏ-point NTT, the twiddle factors are powers of ω_{2^(s+1)} — the (2^(s+1))-th root of unity. there are 2^s distinct twiddle factors at each stage, for a total of N/2 = 2^(k−1) across all stages.

precomputing twiddle factors into a table eliminates repeated exponentiations during the transform. the table has N/2 entries, each a field element (8 bytes for Goldilocks), so a 2²⁰-point NTT needs 4 MB of twiddle storage.

## roots of unity and polynomial evaluation

evaluating a polynomial f(x) of degree < N at the N-th roots of unity {1, ω, ω², ..., ω^(N−1)} is exactly what the NTT computes. the key property that makes this efficient:

```
ω^(N/2) = −1
```

this means half the evaluation points are negations of the other half. the polynomial f(x) = f_even(x²) + x · f_odd(x²) splits into even and odd parts that are each evaluated at N/2 points — the (N/2)-th roots of unity. this is the Cooley-Tukey decomposition.

without roots of unity, polynomial evaluation at N arbitrary points costs O(N²). with roots of unity, the NTT computes it in O(N log N).

## see also

- [[finite-fields]] — the multiplicative group structure
- [[ntt-theory]] — where roots of unity drive the transform
- [[modular-arithmetic]] — Fermat's little theorem and cyclic groups