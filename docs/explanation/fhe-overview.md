# FHE: computing on encrypted data

fully homomorphic encryption allows computation on ciphertexts without decryption. the server never sees the data. the client never reveals the query. the result is correct. this is the cryptographic primitive that makes privacy compatible with computation — and jali is the arithmetic engine underneath.

## what FHE enables

traditional encryption protects data at rest and in transit. FHE protects data _during computation_:

```
client:  encrypt(data) → ciphertext
server:  f(ciphertext) → encrypted_result    // no decryption key needed
client:  decrypt(encrypted_result) → f(data)  // correct result
```

the server computes an arbitrary function f on encrypted data and returns the encrypted result. the server learns nothing about the data. the client learns nothing beyond f(data).

applications in the cyber network: private smart contract execution, confidential token transfers, encrypted machine learning inference, private governance voting. every computation that touches user data benefits from FHE.

## ciphertexts are ring elements

an FHE ciphertext is a pair of ring elements (c_0, c_1) in R_q x R_q, satisfying:

```
c_0 + c_1 * s ≈ m    (mod R_q)
```

where s is the secret key (a ring element with small coefficients), m is the plaintext (encoded as a ring element), and the approximation hides a small error e:

```
c_0 + c_1 * s = m + e
```

decryption computes c_0 + c_1 * s, then rounds away the error to recover m. this works only when |e| is below the rounding threshold.

## homomorphic addition

adding two ciphertexts:

```
(c_0, c_1) + (c_0', c_1') = (c_0 + c_0', c_1 + c_1')
```

decryption of the sum:

```
(c_0 + c_0') + (c_1 + c_1') * s = (m + e) + (m' + e') = (m + m') + (e + e')
```

the plaintexts add. the errors add. cost: 2 ring additions (2n field adds). noise grows by ~1 bit.

## homomorphic multiplication

multiplying two ciphertexts is harder. the naive product of two degree-1 ciphertexts is degree 2:

```
(c_0 + c_1 * s)(c_0' + c_1' * s) = d_0 + d_1 * s + d_2 * s^2
```

where:
```
d_0 = c_0 * c_0'
d_1 = c_0 * c_1' + c_1 * c_0'
d_2 = c_1 * c_1'
```

three ring multiplications to compute (d_0, d_1, d_2). then a relinearization step reduces the degree-2 ciphertext back to degree 1, using a precomputed evaluation key. relinearization costs additional ring multiplications and adds key-switching noise.

total cost per ciphertext multiply: ~5 ring multiplications + key switching. at n = 1024, that is ~15,000 field multiplications. noise grows significantly — the product of the two input noise levels plus a log₂(n) term.

## the noise wall

every multiplication increases noise exponentially:

```
depth 0: noise = e_0                      (fresh ciphertext)
depth 1: noise ~ e_0^2 * n               (one multiplication)
depth 2: noise ~ e_0^4 * n^3             (two multiplications)
depth L: noise ~ e_0^{2^L} * n^{2^L - 1} (L multiplications)
```

at some depth L_max, the noise exceeds the decryption threshold. for typical parameters (n = 1024, initial noise ~ 2^{-15}), L_max is around 10-20 without bootstrapping.

## bootstrapping

bootstrapping resets the noise to a fixed level by homomorphically evaluating the decryption circuit:

```
1. encrypt the secret key under a bootstrapping key
2. homomorphically compute: c_0 + c_1 * s mod q, then round
3. output: fresh ciphertext encrypting the same plaintext, with reset noise
```

the bootstrap circuit is dominated by:
- ring multiplications (for the modular arithmetic)
- automorphisms (for the blind rotation / accumulator step)
- key switches (to change keys during the computation)

all three operations are jali primitives. bootstrapping is where jali's performance directly determines FHE throughput.

## TFHE and the blind rotation

TFHE (Torus FHE) is the bootstrapping-centric FHE scheme. every gate (AND, OR, NOT) is followed by a bootstrap. the core subroutine is the blind rotation:

```
blind_rotate(acc, LWE_sample, BSK):
  for each bit of the LWE sample:
    conditionally apply automorphism σ_k to acc
    key-switch back using BSK[k]
```

the blind rotation performs ~n automorphisms and ~n key switches. each key switch involves ring multiplications. at n = 1024, one bootstrap is ~10^6 field multiplications — entirely within jali.

## jali's role in the stack

```
mudra::veil   TFHE scheme: encrypt, bootstrap, gate evaluation
    ↓
jali          ring arithmetic: R_q mul, NTT, automorphisms, noise tracking
    ↓
nebu          scalar arithmetic: F_p add, mul, inv (Goldilocks)
```

mudra defines the cryptographic protocol. jali provides the ring algebra. nebu provides the scalar field. each layer does what it does best.

the 3072x cost gap (one ring multiply = 3072 scalar multiplies) is why jali exists as a separate layer. encoding ring operations as scalar constraints would waste three orders of magnitude. a ring-aware algebra — with NTT batching, automorphism permutations, and noise tracking — recovers this performance.

## why FHE matters for cyber

the cyber network's purpose is provable intelligence. users generate data. models process data. proofs verify correctness. but without FHE, either the data is exposed (no privacy) or the computation cannot be verified (no proof).

FHE closes the loop: compute on encrypted data, prove the computation was correct (via zheng's ring-aware proofs), and reveal only the result. the user's data stays private. the computation stays verifiable. the model stays honest.

jali is the arithmetic foundation that makes this possible.

## see also

- [[polynomial-rings]] — the ring R_q underlying FHE ciphertexts
- [[lattice-security]] — the hardness assumption that makes FHE secure
- [[negacyclic-ntt]] — the fast multiplication that bootstrapping requires
- [[five-algebras]] — how jali fits in the complete arithmetic stack
