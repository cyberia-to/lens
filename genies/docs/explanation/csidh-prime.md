# the CSIDH prime: why not Goldilocks

genies is the one module in the cyber stack with a foreign prime. every other module — nebu, kuro, hemera, zheng, nox — works over Goldilocks (p = 2^64 - 2^32 + 1) or the binary field F_2. genies cannot. this article explains why, and why it is a mathematical impossibility rather than a design limitation.

## what CSIDH needs

the CSIDH construction requires a prime q such that:

```
q + 1 = 4 * l_1 * l_2 * ... * l_n
```

where l_1 < l_2 < ... < l_n are distinct small odd primes. the factorization of q+1 must be smooth — composed entirely of small primes.

why smooth q+1? because each small prime l_i enables an l_i-isogeny, and the cost of computing an l_i-isogeny is O(l_i). if q+1 had large prime factors, the corresponding isogenies would be computationally infeasible, and the class group action could not be computed efficiently.

## why Goldilocks fails

Goldilocks: p = 2^64 - 2^32 + 1.

```
p + 1 = 2^64 - 2^32 + 2
      = 2 * (2^63 - 2^31 + 1)
      = 2 * 9223372034707292161
```

the factor 2^63 - 2^31 + 1 = 9223372034707292161 is prime. there are zero small odd prime factors in p+1. not one.

this means no efficient l-isogenies exist over F_p for any small odd l. the class group action cannot be computed. CSIDH over Goldilocks is impossible.

## the incompatibility is structural

Goldilocks was chosen specifically because p-1 has large powers of 2:

```
p - 1 = 2^64 - 2^32 = 2^32 * (2^32 - 1)
```

the 2-adic structure in p-1 enables the Number Theoretic Transform (NTT) — fast polynomial multiplication that zheng and nox rely on.

CSIDH needs smooth p+1. NTT needs smooth p-1. for a random prime, p-1 and p+1 cannot both be smooth simultaneously (by heuristic arguments from analytic number theory, the probability vanishes as the prime grows). the two requirements are in direct tension.

this is not a matter of searching harder. it is an algebraic fact: no prime can simultaneously be NTT-friendly (smooth p-1) and CSIDH-friendly (smooth p+1) at cryptographic sizes.

## the CSIDH-512 prime

the standard CSIDH-512 prime uses the first 74 odd primes:

```
q = 4 * 3 * 5 * 7 * 11 * 13 * 17 * 19 * 23 * ... * 587 - 1
```

the first 74 odd primes are: 3, 5, 7, 11, 13, ..., 587.

```
q + 1 = 4 * product(l_i)
```

by construction, q+1 is maximally smooth. every prime up to 587 divides q+1, giving 74 distinct isogeny degrees.

q itself is prime (verified by primality testing). q is approximately 2^511, making it a 512-bit prime.

## arithmetic cost

since q is 512 bits, F_q arithmetic requires 8 x 64-bit limbs:

| operation | method | relative cost vs Goldilocks |
|-----------|--------|-----------------------------|
| add | 8-limb carry chain | ~8x |
| mul | 8x8 schoolbook + Barrett reduction | ~64x |
| inv | Fermat (a^(q-2)) via ~512 sqr + ~256 mul | ~512 * 64x |

F_q multiplication is roughly 64 times more expensive than a Goldilocks multiplication. this is acceptable because genies operations are infrequent — a key exchange is one action (~50-100 ms), not a bulk computation like hashing or proving.

## Barrett reduction

after multiplying two 512-bit values, we get a 1024-bit product that must be reduced modulo q. genies uses Barrett reduction:

1. precompute mu = floor(2^1024 / q) once
2. for product z: estimate quotient as q_hat = floor(z * mu / 2^1024)
3. compute remainder r = z - q_hat * q
4. at most one conditional subtraction: if r >= q, subtract q

Barrett reduction avoids expensive division at runtime. the precomputed constant mu is fixed for the CSIDH-512 prime.

## constant-time arithmetic

all F_q operations in genies are constant-time:
- no branches that depend on the value of operands
- no variable-time multiplication (schoolbook, not Karatsuba with early termination)
- no data-dependent memory access patterns
- conditional subtraction uses constant-time selection (cmov), not if/else

this is mandatory because F_q operations occur on secret data (the exponent vector and intermediate curve computations). any timing variation leaks information about the secret key.

## the verification bridge

genies produces 512-bit F_q values. zheng operates over 64-bit Goldilocks values. the bridge is limb decomposition:

```
x in F_q  ->  (x_0, x_1, ..., x_7)  in F_p^8
```

each 64-bit limb x_i fits in one Goldilocks element. zheng verifies that the limbs reassemble correctly and that all F_q arithmetic was performed correctly by checking constraints over F_p. genies provides the folding functions; zheng provides the proof system.

## see also

- [[class-group]] — what the smooth factorization enables
- [[supersingular-curves]] — the curves over F_q
- [[verification]] — how F_q folds into F_p for proofs
