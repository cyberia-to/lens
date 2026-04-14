---
tags: cyber, cip
crystal-type: entity
crystal-domain: crypto
alias: jali, polynomial ring, R_q arithmetic, lattice arithmetic
---
# jali (जाली)

polynomial ring arithmetic for [[cyber]]. R_q = F_p[x]/(x^n+1) over [[Goldilocks field|Goldilocks]]. jali is to polynomial rings what [[nebu]] is to scalars and [[kuro]] is to binary fields.

```
R_q = F_p[x] / (x^n + 1)
```

the fifth execution algebra — structured vectors for lattice cryptography, fully homomorphic encryption, and ring-based protocols.

## the algebra

```
p = 2^64 - 2^32 + 1            Goldilocks prime (nebu)
n = power of 2 (1024, 2048, 4096)
x^n + 1 = Φ_{2n}               cyclotomic polynomial

element: vector of n Goldilocks field elements (coefficients)
addition: coefficient-wise (n parallel nebu adds)
multiplication: NTT → pointwise → INTT (3n nebu muls)
automorphisms: x → x^{5^k mod 2n} (Galois group of Φ_{2n})
```

via NTT, R_q decomposes as F_p^n (n independent copies of Goldilocks). the ring structure (cyclotomic wrapping) is what makes Ring-LWE hard. the NTT decomposition is what makes computation fast.

## why jali exists

one R_q multiply = 3n F_p multiplies via the negacyclic NTT. at n = 1024, that is 3072 scalar multiplications per ring multiply. encoding ring multiplication as 3072 separate nebu constraints in a proof system wastes 3072x. a dedicated ring-aware algebra — with NTT batching, automorphism exploitation, and noise tracking — closes the gap.

the same criterion that separates kuro (32x for binary), trop (10x for optimization), and genies (foreign prime for isogeny): when the cost gap exceeds an order of magnitude, the workload deserves its own algebra.

## operations

| operation | description | complexity |
|-----------|-------------|------------|
| add(a, b) | coefficient-wise addition | n nebu adds |
| sub(a, b) | coefficient-wise subtraction | n nebu subs |
| mul(a, b) | NTT → pointwise → INTT | 3n nebu muls |
| neg(a) | coefficient-wise negation | n nebu negs |
| scalar_mul(a, s) | coefficient-wise multiply by scalar | n nebu muls |
| automorphism(a, k) | x → x^{5^k}: permutes NTT slots | n index ops |
| ntt(a) | coefficient form → NTT form | n nebu muls |
| intt(a) | NTT form → coefficient form | n nebu muls |
| sample_uniform(n) | uniform random ring element | n random F_p |
| sample_ternary(n) | coefficients in {-1, 0, 1} | n ternary draws |
| sample_cbd(n, eta) | centered binomial distribution | n CBD draws |
| noise_after_add | noise bound growth for addition | +1 bit |
| noise_after_mul | noise bound growth for multiplication | log₂(n) bits |

## parameters

| n | LWE security (at q ~ 2^64) | use case |
|------|----------------------------|----------|
| 1024 | ~128 bits | standard FHE |
| 2048 | ~192 bits | high security |
| 4096 | ~256 bits | maximum security |

n must be a power of 2 (NTT requirement) and must divide 2^32 (Goldilocks two-adicity). all three values satisfy both constraints.

## structure

```
jali/
├── rs/              core library (depends on nebu, no_std)
│   └── src/lib.rs   RingElement, NTT, noise, sampling, automorphisms
├── cli/             command-line tool
├── reference/       canonical specifications (7 docs)
│   ├── ring.md      R_q element type, add, sub, mul, automorphisms
│   ├── noise.md     noise tracking, bounds, growth estimation
│   ├── ntt.md       negacyclic NTT transform specification
│   ├── sample.md    error distributions, seeded sampling
│   ├── encoding.md  RingElement to/from bytes
│   └── automorphism.md  Galois group action on R_q
└── docs/explanation/ educational articles (5 docs)
```

## companion repos

| repo | role |
|------|------|
| [[nebu]] | Goldilocks field arithmetic — jali's only dependency |
| [[kuro]] | F₂ tower field (binary regime, complementary algebra) |
| [[trop]] | tropical semiring (optimization regime) |
| [[genies]] | isogeny group action (post-quantum regime) |
| [[mudra]] | protocols built on jali: seal (ML-KEM), veil (TFHE) |
| [[zheng]] | proof system: ring-aware PCS, NTT batching |
| [[nox]] | VM: jets for gadget_decompose, ntt_batch, key_switch |

## dependency

```
nebu (F_p scalar arithmetic + NTT roots of unity)
  ↓
jali (R_q polynomial ring arithmetic)
  ↓
mudra (seal, veil — protocols over jali)
zheng (PCS₃ — ring-aware proving)
nox (jets — accelerated execution)
```

jali depends only on nebu. no hemera dependency (hashing is the consumer's job). pure arithmetic.

## license

cyber license: don't trust. don't fear. don't beg.
