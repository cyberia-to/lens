---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: nebu explanation, field encyclopedia
diffusion: 0.00010722364868599256
springs: 0.0003718815264935351
heat: 0.00030996176623186564
focus: 0.00022716863553742702
gravity: 0
density: 2.03
---

# finite field arithmetic

an encyclopedia of the mathematics behind the Goldilocks field — from first principles to applications. every concept is grounded in the field we implement: p = 2⁶⁴ − 2³² + 1.

## foundations

- [[finite-fields]] — field axioms, existence and uniqueness, GF(p), characteristic, the multiplicative group
- [[modular-arithmetic]] — congruence, residue classes, Fermat's little theorem, constant-time arithmetic

## the Goldilocks field

- [[goldilocks]] — why this prime, the ε reduction identity, add/sub/mul/inversion algorithms, S-box, batch inversion, comparison with Barrett and Montgomery
- [[sqrt]] — square root and Legendre symbol (Tonelli-Shanks, sign convention)
- [[batch]] — batch inversion (Montgomery's trick, amortized 3 muls/element)
- [[fp2]] — quadratic extension F_{p²} = F_p[u]/(u²−7) for 128-bit security

## algebraic structure

- [[roots-of-unity]] — primitive roots, generators, quadratic residues, subgroup lattice, twiddle factors

## transforms and polynomials

- [[ntt-theory]] — the NTT as finite-field FFT, butterfly decomposition, Cooley-Tukey, Gentleman-Sande, complexity
- [[polynomial-arithmetic]] — evaluation, interpolation, convolution, Reed-Solomon codes, Schwartz-Zippel lemma

## applications

- [[applications]] — STARK proofs, Poseidon2 hashing, FHE, polynomial commitments, verifiable computation