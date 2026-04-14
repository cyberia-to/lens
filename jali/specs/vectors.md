---
tags: jali, crypto
crystal-type: entity
crystal-domain: crypto
---
# vectors — test vectors for ring operations

known-answer tests for R_q polynomial ring arithmetic. any conforming implementation must produce identical results.

placeholder: test vectors will be generated from the reference implementation once rs/src/lib.rs is complete. the following structure defines what vectors are needed.

## required test vectors

### ring arithmetic (small n = 4 for readability)

| test | inputs | expected output |
|------|--------|----------------|
| add | a = [1, 2, 3, 4], b = [5, 6, 7, 8] | [6, 8, 10, 12] mod p |
| sub | a = [5, 6, 7, 8], b = [1, 2, 3, 4] | [4, 4, 4, 4] |
| neg | a = [1, 2, 3, 4] | [p-1, p-2, p-3, p-4] |
| scalar_mul | a = [1, 2, 3, 4], s = 3 | [3, 6, 9, 12] |
| mul | a = [1, 1, 0, 0], b = [1, 1, 0, 0] | verify via naive convolution mod x^4+1 |

### NTT round-trip

| test | description |
|------|-------------|
| ntt then intt | for random a, intt(ntt(a)) == a |
| ntt mul vs naive | ntt_mul(a, b) == naive_poly_mul(a, b) mod x^n+1 |
| zero | ntt([0, 0, ..., 0]) == [0, 0, ..., 0] |
| one | ntt([1, 0, ..., 0]) == twist factors |

### automorphism

| test | description |
|------|-------------|
| identity | automorphism(a, 1) == a |
| involution | automorphism(automorphism(a, k), k_inv) == a |
| ntt consistency | ntt(automorphism_coeff(a, k)) == automorphism_ntt(ntt(a), k) |

### sampling (deterministic)

| test | description |
|------|-------------|
| ternary | seed 0x00..00 → fixed element, all coefficients in {-1, 0, 1} |
| cbd(2) | seed 0x00..00 → fixed element, all coefficients in {-2, -1, 0, 1, 2} |
| uniform | seed 0x00..00 → fixed element, all coefficients in [0, p) |

## edge cases

| operation | result |
|-----------|--------|
| add(zero, a) | a |
| mul(zero, a) | zero |
| mul(one, a) | a (where one = [1, 0, ..., 0]) |
| a + (-a) | zero |

## see also

- [ring](ring.md) — operation definitions
- [ntt](ntt.md) — NTT specification
- [encoding](encoding.md) — byte encoding of test values
