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

## five lenses

| algebra | field | construction | spec |
|---------|-------|-------------|------|
| F_p (+ extensions) | [[Goldilocks field]] | expander-graph codes (Brakedown) | [[field]] |
| F₂ tower | binary | binary Reed-Solomon (Binius) | [[binary]] |
| R_q | polynomial ring | NTT-batched expander codes | [[ring]] |
| (min,+) | tropical semiring | witness-verify via field lens | [[tropical]] |
| F_q | isogeny curves | expander codes over F_q | [[isogeny]] |

files named by algebra (the math), not by implementation repo (nebu, kuro, etc.).

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

lens depends on hemera for commitment binding (Merkle trees, Fiat-Shamir). lens depends on arithmetic repos (nebu, kuro, jali, trop, genies) for field operations per backend.

## consumers

| consumer | what it uses | how |
|----------|-------------|-----|
| nox | Lens.commit for noun identity | identity = hemera(Lens.commit(noun_poly) ‖ tag) |
| zheng | Lens.commit/open/verify for proof commitment | SuperSpartan + sumcheck queries lens |
| bbg | Lens.commit for state root | BBG_root = hemera(Lens.commit(BBG_poly) ‖ ...) |
