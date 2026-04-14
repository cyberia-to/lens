# polynomial ring arithmetic

an encyclopedia of the mathematics behind jali's polynomial ring R_q = F_p[x]/(x^n+1) — from first principles to applications. every concept is grounded in the ring we implement: structured vectors of Goldilocks field elements with negacyclic convolution.

## foundations

- [[polynomial-rings]] — what is F_p[x]/(x^n+1), coefficients as vectors, multiplication as convolution, why cyclotomic polynomials

## algorithms

- [[negacyclic-ntt]] — standard NTT vs negacyclic NTT, the twisting trick, why Goldilocks is perfect, cost model

## security

- [[lattice-security]] — Ring-LWE assumption, noise as security mechanism, noise budget tracking, bootstrapping

## context

- [[fhe-overview]] — what FHE enables, ciphertexts as ring elements + noise, why jali matters for privacy
- [[five-algebras]] — the complete arithmetic stack: nebu, kuro, trop, genies, jali
