---
tags: cyber, cip
crystal-type: entity
crystal-domain: crypto
alias: genies, isogeny arithmetic, class group action
---
# genies

isogeny group action arithmetic for [[cyber]]. genies provides the algebraic primitives for supersingular isogeny computation over a 512-bit CSIDH prime.

```
action: cl(O) × Ell(O, π) → Ell(O, π)
```

the class group cl(O) acts on the set of supersingular elliptic curves with endomorphism ring O. the action is commutative: [a] then [b] equals [b] then [a]. genies computes this action.

## why genies exists

three properties simultaneously:

1. **post-quantum security** — no known quantum algorithm breaks the class group action (Kuperberg is subexponential, not polynomial)
2. **commutative group action** — enables non-interactive protocols without pairings
3. **compact representation** — public keys are single curve coefficients (~64 bytes)

no known construction achieves all three over [[Goldilocks field|Goldilocks]]. the CSIDH prime q = 4 * l_1 * l_2 * ... * l_n - 1 requires smooth q+1, which is algebraically incompatible with NTT-friendly primes. see [[reference/prime|prime]] for details.

## operations

| operation | description | complexity |
|-----------|-------------|------------|
| fq_mul(a, b) | F_q multiplication (512-bit) | 8x8 limb schoolbook + Barrett |
| fq_inv(a) | F_q inversion | Fermat: ~512 sqr + ~256 mul |
| point_add(P, Q) | elliptic curve point addition | 6 fq_mul (projective) |
| isogeny(E, P, l) | l-isogeny with kernel P via Velu | O(l) fq operations |
| action(secret, E) | class group action [a] * E | n isogeny steps |
| dh(secret, peer) | action(secret, peer) | 1 action |
| batch_action(secrets, E) | multiple actions sharing computation | amortized |
| encode(E) | curve to 64 bytes | x-coordinate + sign |
| fold(x) | F_q element to 8 Goldilocks limbs | for zheng proofs |

## structure

```
genies/
├── rs/              core library (no_std, zero deps)
│   └── src/lib.rs   F_q arithmetic, curve ops, isogeny, action
├── cli/             command-line tool
├── reference/       canonical specifications (8 docs)
└── docs/            documentation
```

## the prime

CSIDH-512: q = 4 * 3 * 5 * 7 * 11 * ... * 587 - 1 (first 74 odd primes). q ~ 2^511.

this is the one module in the [[cyber]] stack with a foreign prime. not because the design is incomplete, but because mathematics does not permit the three properties over Goldilocks.

## verification pathway

isogeny computations produce witnesses (the action path). [[zheng]] verifies correctness by folding F_q witnesses into Goldilocks:

```
F_q element (512-bit) → 8 Goldilocks limbs (8 × 64-bit) → zheng constraint
```

genies provides the folding arithmetic. zheng provides the proof system.

## companion repos

| repo | role |
|------|------|
| [[mudra]] | protocols built on genies (CSIDH DH, VRF, VDF, threshold, stealth, blind) |
| [[nebu]] | Goldilocks field arithmetic (proof backbone) |
| [[kuro]] | F_2 tower arithmetic (binary regime) |
| [[hemera]] | hash function (commitment, trust anchor) |
| [[nox]] | VM (jet dispatch for accelerated isogeny ops) |
| [[zheng]] | proof system (verifies isogeny computation via folding) |

protocols built on genies (CSIDH key exchange, VRF, VDF, threshold, stealth addresses, blind signatures) live in [[mudra]].

## license

cyber license: don't trust. don't fear. don't beg.
