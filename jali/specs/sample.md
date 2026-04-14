---
tags: jali, crypto
crystal-type: entity
crystal-domain: crypto
---
# sample — error distributions and sampling

lattice cryptography requires random ring elements drawn from specific distributions. the distribution choice determines security level, noise growth, and performance.

## distributions

```
sample_uniform(n: usize, seed: [u8; 32]) → RingElement
  each coefficient uniform in [0, p)
  used for: public matrices A, random oracles
  entropy: 64n bits

sample_ternary(n: usize, seed: [u8; 32]) → RingElement
  each coefficient in {-1, 0, 1} with equal probability 1/3
  used for: secret keys, short vectors
  entropy: ~1.58n bits
  infinity norm: 1

sample_cbd(n: usize, eta: usize, seed: [u8; 32]) → RingElement
  centered binomial distribution with parameter η
  coefficient = Σ(a_i) - Σ(b_i) where a_i, b_i uniform bits
  range: [-η, η]
  used for: ML-KEM noise (NIST standard), TFHE errors
  variance: η/2
  entropy: 4ηn bits (consumes 2η random bits per coefficient)

sample_gaussian(n: usize, sigma: f64, seed: [u8; 32]) → RingElement
  discrete Gaussian with standard deviation σ
  used for: Ring-LWE error terms (theoretical constructions)
  requires rejection sampling or CDT (cumulative distribution table)
```

## deterministic seeded sampling

all sampling functions take a 32-byte seed. the same seed always produces the same ring element. this is essential for:

1. **reproducibility**: test vectors are deterministic
2. **compressed randomness**: a ciphertext can transmit seeds instead of full ring elements
3. **verifiable sampling**: a prover can demonstrate that a ring element was sampled correctly by revealing the seed

the seed is expanded via a PRF (the consumer's choice — typically hemera/Poseidon2 or SHAKE). jali specifies the distribution, not the PRF.

## security parameters

| scheme | distribution | η or σ | security claim |
|--------|-------------|--------|----------------|
| ML-KEM-768 (NIST) | CBD | η = 2 | 128-bit (NIST Level 1) |
| ML-KEM-1024 (NIST) | CBD | η = 2 | 192-bit (NIST Level 3) |
| TFHE bootstrapping | Gaussian | σ ~ 2^{-15} | 128-bit |
| Ring-LWE (generic) | Gaussian | σ = α * q/√(2π) | depends on α |

the noise parameter controls the hardness vs correctness trade-off. larger noise = harder lattice problem = more security, but faster noise budget exhaustion during FHE computation.

## coefficient bounds

| distribution | max |coeff| | expected ||a||₂ |
|-------------|-----------|----------------|
| uniform | p - 1 | ~p/√3 * √n |
| ternary | 1 | ~√(2n/3) |
| CBD(η) | η | ~√(ηn/2) |
| Gaussian(σ) | ~6σ (tail bound) | ~σ√n |

these bounds feed into jali's noise tracking system. see [noise](noise.md).

## see also

- [noise](noise.md) — how sampling distributions affect noise budgets
- [ring](ring.md) — the RingElement type that sampling produces
