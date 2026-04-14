---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: Goldilocks, Goldilocks prime, Goldilocks arithmetic, F_p, goldilocks, field choice, reduction, multiplication, inversion
stake: 27174830290765380
diffusion: 0.0012017391670251642
springs: 0.0003233948039147658
heat: 0.0006053419858528769
focus: 0.0008189564218575767
gravity: 13
density: 0.67
---

# the Goldilocks field

the complete story of the Goldilocks prime: why it was chosen, how its arithmetic works, and what makes it fast. every algorithm below runs on p = 2⁶⁴ − 2³² + 1.

## why this prime

three properties make Goldilocks the right field for [[cyber]].

**native u64 arithmetic.** every field element fits in a single machine register. addition is one add + one conditional subtract. multiplication is one u128 multiply + fast reduction. no multi-limb arithmetic, no Montgomery form. compare with BN254 (254-bit, 4-limb) or BLS12-381 (381-bit, 6-limb).

**STARK compatibility.** the two-adicity of 32 enables NTT of length up to 2³² — sufficient for any practical STARK proof. the same field used for hashing is the field used for proving. a [[hemera]] hash output is 8 Goldilocks elements — they enter the STARK prover directly. ~1,200 constraints per hash vs ~15,000 for BLAKE3.

**universal substrate.** the Goldilocks field is the arithmetic home for every domain in [[cyber]]:

| domain | how F_p helps |
|--------|---------------|
| ZK proofs | programs are arithmetic circuits over F_p by construction |
| AI | weights and activations are field elements, no quantization |
| FHE | when ciphertext modulus q = p, proof impedance vanishes |
| quantum | prime dimension eliminates gate decomposition overhead |

one field. no conversion at any boundary.

**the double seven.** the S-box exponent must satisfy gcd(d, p−1) = 1 for bijectivity. d=3 fails, d=5 fails, d=7 is the minimum. the encoding width must fit [0, p) unconditionally: max 7-byte value 2⁵⁶ − 1 < p, but 8 bytes can exceed p. the same prime forces both the nonlinear layer and the encoding layer to 7.

## the reduction identity

```
p = 2⁶⁴ − 2³² + 1
2⁶⁴ = p + ε       where ε = 2³² − 1 = 0xFFFFFFFF

therefore:  2⁶⁴ ≡ ε (mod p)
```

this single identity is the engine of Goldilocks arithmetic. every time a computation produces a multiple of 2⁶⁴, that multiple is replaced by ε. no division. no precomputed constants.

the identity converts positional notation into field arithmetic: bit positions above 63 contribute powers of ε instead of powers of 2⁶⁴. this is a consequence of p being a generalized Fermat prime (form a² − a + 1 with a = 2³²).

## addition

```
add(a, b):
  (sum, carry) = a + b          // u64 overflowing add
  (sum, carry2) = sum + carry · ε
  if carry2: sum = sum + ε
  return sum
```

when a + b overflows u64, the carry discards 2⁶⁴ from the true sum. since 2⁶⁴ ≡ ε, adding ε recovers the correct residue. the second carry handles the rare case where adding ε itself overflows. since a, b < p < 2⁶⁴, the maximum sum is 2(p − 1) < 2⁶⁵, so at most two corrections suffice.

## subtraction

```
sub(a, b):
  (diff, borrow) = a − b        // u64 overflowing sub
  (diff, borrow2) = diff − borrow · ε
  if borrow2: diff = diff − ε
  return diff
```

underflow wraps by adding 2⁶⁴. subtracting ε corrects. the symmetry is exact: overflow adds ε, underflow subtracts ε.

## multiplication

field multiplication is the dominant cost in every application: hash rounds, NTT butterflies, matrix products.

**the widening multiply.** two u64 values produce a u128 product. on x86-64, the `mul` instruction places the 128-bit result in rdx:rax. on AArch64, `umulh`/`mul` produce the halves separately. this single instruction is the only expensive step.

**the reduction pipeline.** the 128-bit product splits and reduces:

```
mul(a, b):
  x = a × b                          // u128
  x_lo = x[0:64]
  x_hi = x[64:128]
  x_hi_hi = x_hi >> 32               // bits 96–127
  x_hi_lo = x_hi & ε                 // bits 64–95

  (t0, borrow) = x_lo − x_hi_hi      // 2⁹⁶ ≡ −1, so subtract
  if borrow: t0 = t0 − ε

  t1 = x_hi_lo × ε                   // 2⁶⁴ ≡ ε, 32×32 → 64 bit

  (result, carry) = t0 + t1
  return result + carry · ε
```

why: x_hi_lo · 2⁶⁴ ≡ x_hi_lo · ε. and x_hi_hi · 2⁹⁶ = x_hi_hi · ε · 2³², where ε · 2³² = 2⁶⁴ − 2³² = p − 1 ≡ −1, so x_hi_hi · 2⁹⁶ ≡ −x_hi_hi.

three 64-bit operations after the u128 multiply. no division. no Montgomery form.

**machine-level pipeline:**

```
mul     rax, a, b          // u128 product → rdx:rax
shr     t, rdx, 32         // x_hi_hi
and     u, rdx, 0xFFFFFFFF // x_hi_lo
sub     rax, rax, t        // x_lo − x_hi_hi (+ borrow correction)
imul    u, u, ε            // x_hi_lo × ε
add     rax, rax, u        // combine (+ carry correction)
```

six instructions. steps 2–3 parallelize, step 5 overlaps with 4. throughput: ~4–5 cycles per multiplication.

## squaring and the S-box

squaring (a × a) uses the same reduction pipeline. on modern x86-64, specialized squaring provides marginal improvement over general multiplication since `mul` is already fast.

the Poseidon2 S-box computes x⁷ in three multiplications:

```
x² = x · x,  x³ = x² · x,  x⁴ = x² · x²,  x⁷ = x³ · x⁴
```

optimal — no addition chain for 7 uses fewer. cost: ~15 cycles.

## multiply-accumulate

```
fma(a, b, c) = a + b · c mod p
```

the fundamental operation for matrix-vector products (Poseidon2 linear layer), polynomial evaluation (Horner's method), and NTT butterflies. a dedicated `fma` instruction (see [[hardware]]) keeps the accumulator in a register, eliminating store-load latency.

## inversion

field inversion computes a⁻¹ such that a · a⁻¹ = 1. roughly 64× the cost of one multiplication.

**Fermat's method.** a^(p−1) = 1, so a⁻¹ = a^(p−2). the exponent:

```
p − 2 = 2⁶⁴ − 2³² − 1 = 0xFFFFFFFEFFFFFFFF
```

binary: 32 ones, one zero, 31 ones. hamming weight 63.

**square-and-multiply** gives 63 squarings + 62 multiplications = 125 muls. but the Mersenne structure of the exponent enables an optimized addition chain:

```
compute a^(2^k − 1) for k = 1, 2, 4, 8, 16, 32:

a^1        = a
a^3        = a^2 · a
a^(2⁴−1)  = (a^3)^(2²) · a^3
a^(2⁸−1)  = (a^(2⁴−1))^(2⁴) · a^(2⁴−1)
a^(2¹⁶−1) = (a^(2⁸−1))^(2⁸) · a^(2⁸−1)
a^(2³²−1) = (a^(2¹⁶−1))^(2¹⁶) · a^(2¹⁶−1)
```

then 32 squarings and a final correction. total: ~64 multiplications.

**batch inversion.** Montgomery's trick inverts n elements with 1 inversion + 3(n−1) multiplications:

```
batch_invert(a[0..n]):
  prefix[0] = a[0]
  for i in 1..n: prefix[i] = prefix[i-1] · a[i]
  inv = prefix[n-1]⁻¹
  for i in (1..n).rev():
    result[i] = inv · prefix[i-1]
    inv = inv · a[i]
  result[0] = inv
```

amortized cost: 3 multiplications per element. critical for NTT twiddle precomputation and polynomial division.

**division** is multiplication by the inverse: div(a, b) = a · b⁻¹.

| method | cost | best when |
|--------|------|-----------|
| Fermat | ~64 muls | fast multiplier (most CPUs) |
| extended GCD | ~64 divisions | no fast multiplier |
| batch (Montgomery) | 3 muls/element | inverting many elements |

## canonicalization

the reduction algorithms produce values in [0, 2⁶⁴) — correct mod p but possibly non-canonical (in [p, 2⁶⁴)).

```
canonicalize(v):
  if v ≥ p: return v − p
  return v
```

canonicalization is deferred in practice. intermediate results tolerate non-canonical form — the next operation's overflow correction handles it. applied only at output boundaries: serialization, comparison, hashing.

## comparison with other strategies

| strategy | reduction cost | applicability |
|----------|---------------|---------------|
| trial subtraction | 1 division (20–90 cycles) | any modulus |
| Barrett | 2 multiplies + correction | any modulus, precomputed |
| Montgomery | 1 multiply + shift + form conversion | any odd modulus |
| Goldilocks | 2–3 adds/subs | only p = 2⁶⁴ − 2³² + 1 |

Montgomery is the standard for arbitrary-prime cryptography. it replaces division by a shift, but requires converting values to Montgomery form (multiply by R = 2⁶⁴ mod p) and back. Goldilocks eliminates this entire pipeline — 3–5× faster than generic 64-bit primes.

## hardware

the [[GFP]] (Goldilocks Field Processor) has four primitives optimized for this field: `fma` (field multiply-accumulate), `ntt` ([[NTT]] butterfly), `p2r` ([[Poseidon2]] round), `lut` (lookup table). see [[hardware]] for the full proposal.

## see also

- [[finite-fields]] — the algebraic structure behind F_p
- [[modular-arithmetic]] — congruence, Fermat's theorem, constant-time
- [[roots-of-unity]] — the cyclic structure enabling NTT
- [[ntt-theory]] — where multiplication meets polynomial arithmetic
- [[applications]] — STARK proofs, Poseidon2, FHE
- [[sqrt]] — square root (Tonelli-Shanks) over Goldilocks
- [[batch]] — batch inversion (Montgomery's trick)
- [[fp2]] — F_{p²} for 128-bit recursive STARK security