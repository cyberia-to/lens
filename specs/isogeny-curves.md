---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: isogeny lens, Porphyry, genies lens, isogeny-pcs
---
# isogeny lens (Porphyry)

## Current implementation boundary

Porphyry remains an experimental legacy single-point backend. It has not adopted
Brakedown's versioned systematic tensor authentication and does not inherit that
backend's binding or verification arguments. Empty and multi-point batch proving
are rejected, and batch verification returns false for those requests. The former
random-point batch path ignored the caller's claimed values and also passed only
32 bytes to the Fq reduction function, which requires at least 64. That path is
removed; expanding a random challenge alone would not repair its authentication.
The general Lens transcript's single-hash `squeeze_field` API remains unsuitable
for Fq; a future Fq protocol must specify a domain-separated 64-byte-or-wider
challenge derivation together with its actual proof checks.


the F_q lens. dedicated polynomial commitment over [[genies]]'s isogeny field F_q where q = 4·ℓ₁·ℓ₂·...·ℓₙ - 1. privacy primitives (stealth addresses, VDF verification, blind signatures, ring signatures) prove natively in their own field — no non-native arithmetic penalty.

implements the [[trait|Lens]] trait for F_q. the IOP layer ([[SuperSpartan]] + [[sumcheck]]) stays field-generic. only the polynomial commitment scheme changes. [[hemera]] remains the only hash.

part of the five-lens architecture — see [[commitment]] for the full picture.

see [[commitment]] §2 for the Lens trait definition.

## why dedicated lens, not non-native

genies uses primes q that may be 512-bit or larger (CSIDH-512: 512-bit prime). encoding F_q operations as F_p constraints costs (512/64)² = 64× per operation. a single isogeny walk involves thousands of F_q operations. 64× × thousands = unacceptable.

dedicated Porphyry lens over F_q: each F_q operation = 1 constraint. same approach as Binius for F₂ — match the lens to the algebra.

| approach | cost per F_q mul | isogeny walk (1000 muls) |
|----------|-----------------|------------------------|
| non-native in F_p | ~64 F_p constraints | ~64,000 constraints |
| dedicated Porphyry | 1 F_q constraint | ~1,000 constraints |

## F_q field structure

```
q = 4 · ℓ₁ · ℓ₂ · ... · ℓₙ - 1

  ℓᵢ: small distinct primes (for efficient isogeny computation)
  q ≡ 3 (mod 4): enables supersingular curve y² = x³ + x over F_q

CSIDH parameter sets:
  CSIDH-512:   q = 512-bit prime, 74 small primes ℓᵢ
  CSIDH-1024:  q = 1024-bit prime
  CSIDH-2048:  q = 2048-bit prime
```

F_q elements are multi-limb integers (8-32 Goldilocks-sized limbs). arithmetic uses Montgomery multiplication.

## commitment

Brakedown instantiated over F_q instead of F_p. same expander-graph linear code construction, different field:

```
commit(poly over F_q):
  1. encode evaluations via expander-graph linear code over F_q
  2. build hemera Merkle tree over encoded rows
     (hemera hashes bytes — field element serialization is transparent)
  3. root = hemera hash (same 32-byte format as all lens backends)
```

hemera operates on byte sequences. F_q elements serialize to bytes. same Merkle tree, same commitment format, same Fiat-Shamir transcript.

## opening and verification

the Brakedown tensor protocol carried over to F_q (legacy; see the implementation boundary above):

```
open(poly, point):
  tensor decompose point into F_q vectors
  compute inner products over F_q
  tensor opening over F_q (no proven soundness bound: code distance unproven)

verify(commitment, point, value, proof):
  check tensor consistency (F_q arithmetic)
  check Merkle auth paths (hemera)
  verifier cost: not measured
```

## cost comparison

| operation | F_p (Goldilocks) | F_q (Porphyry) | notes |
|---|---|---|---|
| F_p multiply | 1 constraint | N/A | nebu workload |
| F_q multiply | ~64 constraints (non-native) | 1 constraint | genies workload |
| isogeny walk step | ~64 constraints (non-native) | 1 constraint | privacy primitive |
| hemera hash | ~736 constraints | ~736 constraints | same (hemera is F_p native) |
| VDF verification | ~64K constraints (non-native) | ~1K constraints | delay primitive |

## primary workloads

| workload | what it proves | constraints |
|----------|---------------|-------------|
| stealth key exchange | isogeny walk correctness | ~1000 F_q |
| VDF verification | sequential squaring proof | ~1000 F_q |
| blind signatures | group action on blinded message | ~500 F_q |
| ring signatures | one-of-n group action proof | ~2000 F_q |
| VRF | deterministic randomness from group action | ~500 F_q |

## cross-algebra composition

```
nox<F_q> sub-trace → Porphyry commitment over F_q (legacy: Brakedown-based, distance unproven)
                           ↓
            composition with the Goldilocks accumulator: open research
```

phase 3 of the proof-system repair (soft3 `proposals/proof-system-repair.md`, landing in this release) accumulates Reed–Solomon evaluation claims over Goldilocks: each step's claims are batched by a sumcheck into one root plus a fixed number of claims, and the decider is one WHIR opening of the final accumulator (zheng `specs/accumulation.md`, `specs/machine.md`). the earlier fold of every algebra into a shared F_p accumulator through a universal CCS with per-algebra selectors, and its per-fold and per-crossing costs, belonged to the retired design (legacy, unsound) and are withdrawn.

F_q claims are not Goldilocks evaluation claims, so they do not enter that accumulator directly; simulating F_q in Goldilocks or hemera in F_q is costly either way. how an isogeny sub-trace joins the accumulator is open research; no cost per crossing is claimed.

## recursion boundary

isogeny proofs are NOT verified inside F_q circuits. verification crosses to Goldilocks:

```
isogeny execution → isogeny proof (hemera Merkle + FS externally)
                        ↓
    verify in F_p circuit (hemera native, ~736 constraints)
                        ↓
    fold into F_p accumulator
```

hemera inside F_q circuit: prohibitively expensive (simulating Goldilocks in F_q). hemera inside F_p circuit: ~736 constraints. recursion boundary is always Goldilocks.

## genies dependency

a separate repo **genies** provides F_q arithmetic:
- multi-limb integer arithmetic (Montgomery multiplication)
- supersingular curve operations over F_q
- class group action computation (isogeny walks)
- no hemera dependency, no nebu dependency
- pure isogeny field algebra

zheng depends on genies for the Porphyry backend. hemera remains the only hash throughout.

## open questions

1. **F_q element size**: CSIDH-512 uses 512-bit field elements (8 × 64-bit limbs). optimal limb representation for Brakedown tensor decomposition
2. **expander graph over F_q**: Brakedown expander construction generalized to multi-limb fields — concrete expansion parameters
3. **proof size**: Brakedown over F_q produces larger proofs (field elements are bigger). target: < 10 KiB per isogeny proof
4. **cross-field composition**: how F_q claims join the Goldilocks accumulation of phase 3 — construction and security reduction, open research

see [[scalar-field]] for Brakedown, [[binary-tower]] for Binius, [[tropical-semiring]] for Assayer, [[commitment]] for the full lens architecture
