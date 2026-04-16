---
tags: cyber, core
alias: lens, lenses, polynomial commitment, PCS
crystal-type: entity
crystal-domain: crypto
---
# lens

five algebraic backends for polynomial commitment in [[cyber]].
the arithmetic layer between [[hemera]] (identity) and [[nox]] (execution).

```
hemera → lens → nox → zheng → bbg
```

## what exists

five field arithmetic crates — one per algebra. each provides the scalar operations
that a polynomial commitment scheme operates over. 352 tests across all crates.

| crate | algebra | what it computes | crates.io |
|-------|---------|-----------------|-----------|
| [[nebu]] | F_p (Goldilocks) | scalar field arithmetic, NTT, extensions (Fp2–Fp4) | cyb-nebu |
| [[kuro]] | F₂ tower | binary tower F₂ → F₂¹²⁸, Karatsuba mul, packed 128-element SIMD | cyb-kuro |
| [[jali]] | R_q = F_p[x]/(x^n+1) | polynomial ring arithmetic, negacyclic NTT, noise tracking | cyb-jali |
| [[trop]] | (min, +) semiring | tropical matrix ops, Kleene star, determinant, eigenvalue | cyb-trop |
| [[genies]] | F_q (CSIDH-512) | 512-bit multi-limb arithmetic, Montgomery curves, isogeny walks | cyb-genies |

the meta-crate `cyber-lens` re-exports all five.

each crate also has a `wgsl/` GPU backend (wgpu compute shaders) and a `cli/` tool.

## what is specified

the `specs/` directory contains the full specification for five polynomial commitment
constructions — one per algebra. these are the commitment schemes that will be built
on top of the arithmetic crates above.

| construction | algebra | encoding | spec |
|-------------|---------|----------|------|
| Brakedown | nebu (F_p) | expander-graph linear code | [[scalar-field]] |
| Binius | kuro (F₂) | binary Reed-Solomon | [[binary-tower]] |
| Ikat | jali (R_q) | NTT-batched Brakedown | [[polynomial-ring]] |
| Assayer | trop (min,+) | witness-verify via dual certificate | [[tropical-semiring]] |
| Porphyry | genies (F_q) | expander codes over deep field | [[isogeny-curves]] |

all five implement the same trait:

```rust
trait Lens<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;
    fn open(poly: &MultilinearPoly<F>, point: &[F]) -> Opening;
    fn verify(commitment: &Commitment, point: &[F], value: F, proof: &Opening) -> bool;
}
```

see [[trait]] for the full interface specification.

## usage

```toml
# all five algebras
[dependencies]
cyber-lens = "0.1"

# or individual crates
cyb-nebu = "0.1"    # Goldilocks field
cyb-kuro = "0.1"    # binary tower
cyb-jali = "0.1"    # polynomial ring
cyb-trop = "0.1"    # tropical semiring
cyb-genies = "0.1"  # isogeny curves
```

```rust
use nebu::Goldilocks;

let a = Goldilocks::new(42);
let b = Goldilocks::new(7);
let c = a * b;
let inv = c.inv();
assert_eq!(c * inv, Goldilocks::ONE);
```

## workspace structure

```
lens/
├── src/           cyber-lens meta-crate (re-exports all five)
├── nebu/          Goldilocks field (F_p)
│   ├── rs/        core library (cyb-nebu)
│   ├── wgsl/      GPU backend
│   ├── cli/       command-line tool
│   └── specs/
├── kuro/          binary tower (F₂)
├── jali/          polynomial ring (R_q)
├── trop/          tropical semiring (min,+)
├── genies/        isogeny curves (F_q)
└── specs/         lens-level specs (trait, constructions)
```

## license

cyber license: don't trust. don't fear. don't beg.
