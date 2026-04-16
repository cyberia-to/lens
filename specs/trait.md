---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: polynomial commitments, polynomial commitment scheme, lens, lenses
---
# lens trait

the universal cryptographic primitive. commit to a polynomial, prove evaluations, verify without seeing the polynomial. one primitive serves proof commitment, state authentication, noun identity, and data availability.

a lens is how an algebra presents its work for verification. each algebra computes in its own structure — scalars, binary, rings, isogenies. the lens makes that computation verifiable. different structures need different optics.

## the interface

```rust
trait Lens<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;     // 32 bytes
    fn open(poly: &MultilinearPoly<F>, point: &[F], transcript: &mut Transcript) -> Opening;
    fn verify(commitment: &Commitment, point: &[F], value: F, proof: &Opening, transcript: &mut Transcript) -> bool;
    fn batch_open(poly: &MultilinearPoly<F>, points: &[(Vec<F>, F)], transcript: &mut Transcript) -> Opening;
    fn batch_verify(commitment: &Commitment, points: &[(Vec<F>, F)], proof: &Opening, transcript: &mut Transcript) -> bool;
}
```

five operations. commit is O(N). open produces a proof. verify checks the proof. batch_open amortizes m openings into one. all transparent (no trusted setup), all post-quantum.

see [[commitment]] for the complete specification including types, security properties, and crate structure.

## naming convention

every algebra has five names at different layers:

| layer | meaning | example |
|-------|---------|---------|
| domain (adjective) | mathematical modifier | scalar, binary, polynomial, tropical, isogeny |
| algebra (noun) | mathematical object | field, tower, ring, semiring, curves |
| impl (repo) | concrete library | nebu, kuro, jali, trop, genies |
| construction (scheme) | commitment construction | Brakedown, Binius, Ikat, Assayer, Porphyry |
| full name | domain + algebra | scalar field, binary tower, polynomial ring, tropical semiring, isogeny curves |

lens files are named `domain-algebra.md` (full name). construction names are inside files. impl repos are dependencies.

## five lenses

| construction | algebra | impl | field | spec |
|-------------|---------|------|-------|------|
| Brakedown | scalar field | [[nebu]] | F_p | [[scalar-field]] |
| Binius | binary tower | [[kuro]] | F₂ | [[binary-tower]] |
| Ikat | polynomial ring | [[jali]] | F_p (NTT slots) | [[polynomial-ring]] |
| Assayer | tropical semiring | [[trop]] | F_p (delegation) | [[tropical-semiring]] |
| Porphyry | isogeny curves | [[genies]] | F_q | [[isogeny-curves]] |

four constructions implement `Lens<F>` directly. Assayer is a wrapper protocol that delegates commitment to Brakedown over F_p — see [[tropical-semiring]].

## three roles

proof commitment — commit [[nox]] execution trace for [[zheng]] verification
state commitment — commit [[bbg]] polynomial state (BBG_poly, A(x), N(x))
noun identity — commit [[nox]] noun polynomial for content addressing

one trait. five lenses. three roles.

## dependency

```
hemera (hash — commitment binding, Fiat-Shamir)
  ↓
lens (polynomial commitment — this repo)
  ↓
nox (noun identity via Lens.commit)
zheng (proof commitment via Lens.commit/open/verify)
bbg (state commitment via Lens.commit)
```

### arithmetics (lens depends on — one per algebra)

| impl | algebra | provides |
|------|---------|----------|
| nebu | scalar field | F_p arithmetic + extensions (Fp2, Fp3, Fp4) |
| kuro | binary tower | F₂ tower arithmetic (F₂ → F₂¹²⁸) |
| jali | polynomial ring | R_q = F_p[x]/(x^n+1) arithmetic |
| trop | tropical semiring | (min,+) semiring arithmetic |
| genies | isogeny curves | F_q arithmetic + group action |

### consumers

| consumer | what it uses | how |
|----------|-------------|-----|
| nox | Lens.commit for noun identity | identity = hemera(Lens.commit(noun_poly) ‖ tag) |
| zheng | Lens.commit/open/verify for proof commitment | zheng adds SuperSpartan + sumcheck on top |
| bbg | Lens.commit for state root | BBG_root = hemera(Lens.commit(BBG_poly) ‖ ...) |
