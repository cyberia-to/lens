---
tags: lens
crystal-type: entity
crystal-domain: crypto
---
# lens reference

canonical specification for polynomial commitment — five lenses for five algebras.

## the trait

```
trait Lens<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;     // 32 bytes
    fn open(poly: &MultilinearPoly<F>, point: &[F]) -> Opening;
    fn verify(commitment: &Commitment, point: &[F], value: F, proof: &Opening) -> bool;
}
```

three operations. commit is O(N). open produces a proof. verify checks the proof. all transparent (no trusted setup), all post-quantum. see [[trait]] for the full specification.

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

| domain | algebra | full name | construction | impl | spec |
|--------|---------|-----------|-------------|------|------|
| scalar | field | scalar field | Brakedown (expander-graph codes over F_p) | [[nebu]] | [[scalar-field]] |
| binary | tower | binary tower | Binius (binary Reed-Solomon over F₂) | [[kuro]] | [[binary-tower]] |
| polynomial | ring | polynomial ring | Ikat (NTT-batched, structure IS the pattern) | [[jali]] | [[polynomial-ring]] |
| tropical | semiring | tropical semiring | Assayer (witness-verify via dual certificate) | [[trop]] | [[tropical-semiring]] |
| isogeny | curves | isogeny curves | Porphyry (expander codes over deep field F_q) | [[genies]] | [[isogeny-curves]] |

## three roles

**proof commitment** — commit [[nox]] execution trace for [[zheng]] verification
**state commitment** — commit [[bbg]] polynomial state (BBG_poly, A(x), N(x))
**noun identity** — commit [[nox]] noun polynomial for content addressing

one trait. five lenses. three roles.

## dependency

```
hemera (hash — commitment binding, Fiat-Shamir)
  ↓
lens (polynomial commitment — this repo)
  ↓
nox (noun identity via Lens.commit)
zheng (proof commitment via Lens.commit)
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
| zheng | Lens.commit/open/verify for proof commitment | SuperSpartan + sumcheck queries lens |
| bbg | Lens.commit for state root | BBG_root = hemera(Lens.commit(BBG_poly) ‖ ...) |
