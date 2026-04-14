---
tags: jali, crypto
crystal-type: entity
crystal-domain: crypto
---
# noise — error distribution and tracking

lattice security comes from noise. jali provides the noise model — generation, tracking, and bound estimation.

## error distributions

```
sample_uniform(n: usize) → RingElement
  each coefficient uniform in [0, p)
  used for: public matrices A

sample_ternary(n: usize) → RingElement
  each coefficient in {-1, 0, 1}
  used for: secret keys, short vectors

sample_gaussian(n: usize, sigma: f64) → RingElement
  each coefficient from discrete Gaussian with standard deviation σ
  used for: LWE error terms

sample_cbd(n: usize, eta: usize) → RingElement
  centered binomial distribution with parameter η
  used for: ML-KEM noise (faster than Gaussian, similar security)
```

## noise tracking

FHE operations increase noise in ciphertexts. when noise exceeds a threshold, decryption fails. tracking noise bounds is essential for correctness.

```
NoiseBudget {
  log_bound: u32     // log₂ of noise bound
}

noise_after_add(a: NoiseBudget, b: NoiseBudget) → NoiseBudget
  log_bound = max(a.log_bound, b.log_bound) + 1

noise_after_mul(a: NoiseBudget, b: NoiseBudget) → NoiseBudget
  log_bound = a.log_bound + b.log_bound + log₂(n)

noise_after_bootstrap() → NoiseBudget
  log_bound = bootstrap_noise    // fixed, determined by parameters

needs_bootstrap(budget: NoiseBudget, max_budget: u32) → bool
  budget.log_bound >= max_budget - safety_margin
```

## noise in zheng proofs

zheng ring-aware proving tracks noise via running accumulator (not per-operation range checks):

```
generic:     64 constraints per coefficient per operation (range check)
             n coefficients × m operations = 64nm constraints

ring-aware:  running accumulator, fold noise bound per step
             check once at bootstrapping boundary
             ~30 field ops per fold
```

## dependencies

- nebu: F_p arithmetic for coefficient generation
- jali::ring: RingElement type
