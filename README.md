---
tags: cyber, core
alias: lens, lenses, polynomial commitment, PCS
crystal-type: entity
crystal-domain: crypto
---
# lens

polynomial commitment for [[cyber]]. five constructions over five algebras.
commit to a polynomial, prove evaluations, verify without seeing the polynomial.

```
hemera (hash) → lens (commitment) → nox (execution) → zheng (proof) → bbg (state)
```

lens sits between [[hemera]] (the hash function) and the three consumers:
[[nox]] uses it for noun identity, [[zheng]] for proof commitment, [[bbg]] for state authentication.

## how it works

a polynomial commitment scheme (PCS) has three operations:

```rust
// commit: seal a polynomial into a 32-byte digest
let commitment = Brakedown::commit(&poly);

// open: prove that poly(point) = value
let proof = Brakedown::open(&poly, &point, &mut transcript);

// verify: check the proof without seeing the polynomial
assert!(Brakedown::verify(&commitment, &point, value, &proof, &mut transcript));
```

the polynomial is multilinear — ν variables, 2^ν evaluations. the commitment is a hemera hash
of the expander-encoded polynomial. the proof is a chain of tensor reductions with proximity
queries at Fiat-Shamir-derived codeword positions.

all five constructions share one trait (`Lens<F: Field>`) and one commitment format (hemera Hash).
this allows [[zheng]] to fold proofs from different algebras into one accumulator.

## five constructions

| crate | construction | algebra | field | what it commits |
|-------|-------------|---------|-------|----------------|
| [cyb-lens-brakedown](brakedown/) | Brakedown | [[nebu]] | Goldilocks F_p | Margulis expander encoding + tensor decomposition |
| [cyb-lens-binius](binius/) | Binius | [[kuro]] | F₂ tower | binary folding with hemera Merkle tree |
| [cyb-lens-ikat](ikat/) | Ikat | [[jali]] | R_q NTT slots | ring elements → NTT → batched Brakedown |
| [cyb-lens-assayer](assayer/) | Assayer | [[trop]] | (wrapper) | tropical witness + dual certificate → Brakedown |
| [cyb-lens-porphyry](porphyry/) | Porphyry | [[genies]] | F_q (512-bit) | Brakedown over deep isogeny field |

four implement `Lens<F>` directly. Assayer is a wrapper protocol — the tropical semiring
has no subtraction, so it packs the optimization witness as Goldilocks elements and delegates
commitment to Brakedown.

## the trait

defined in [cyb-lens-core](core/):

```rust
pub trait Lens<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;
    fn open(poly: &MultilinearPoly<F>, point: &[F], transcript: &mut Transcript) -> Opening;
    fn verify(commitment: &Commitment, point: &[F], value: F, proof: &Opening, transcript: &mut Transcript) -> bool;
    fn batch_open(poly: &MultilinearPoly<F>, points: &[(Vec<F>, F)], transcript: &mut Transcript) -> Opening;
    fn batch_verify(commitment: &Commitment, points: &[(Vec<F>, F)], proof: &Opening, transcript: &mut Transcript) -> bool;
}
```

types: `Field` (scalar trait), `MultilinearPoly<F>` (evaluation table), `Commitment` (hemera Hash),
`Opening` (Tensor / Folding / Witness variants), `Transcript` (hemera sponge for Fiat-Shamir).

## five algebras

| crate | algebra | what it computes | tests |
|-------|---------|-----------------|-------|
| [cyb-nebu](nebu/rs/) | F_p (Goldilocks, p = 2^64 - 2^32 + 1) | field arithmetic, NTT, extensions Fp2–Fp4 | 73 |
| [cyb-kuro](kuro/rs/) | F₂ tower (F₂ → F₂¹²⁸) | Karatsuba mul, packed 128-element SIMD | 77 |
| [cyb-jali](jali/rs/) | R_q = F_p[x]/(x^n+1) | negacyclic NTT, noise tracking, automorphisms | 70 |
| [cyb-trop](trop/rs/) | (min, +) semiring | matrix ops, Kleene star, determinant, eigenvalue | 77 |
| [cyb-genies](genies/rs/) | F_q (CSIDH-512, 512-bit) | Montgomery curves, isogeny walks, class group action | 55 |

each algebra also has a `wgsl/` GPU backend (wgpu compute shaders) and a `cli/` tool.

## usage

```toml
# everything — trait + all five constructions + all five algebras
[dependencies]
cyber-lens = "0.1"

# just the trait (for consumers like nox, zheng, bbg)
[dependencies]
cyb-lens-core = "0.1"

# one specific construction
[dependencies]
cyb-lens-brakedown = "0.1"  # pulls in cyb-lens-core + cyb-nebu

# one specific algebra (no commitment, just field arithmetic)
[dependencies]
cyb-nebu = "0.1"
```

### commit → open → verify

```rust
use cyb_lens_brakedown::*;
use nebu::Goldilocks;

// create a 2-variable polynomial: f(0,0)=1, f(1,0)=2, f(0,1)=3, f(1,1)=4
let poly = MultilinearPoly::new(vec![
    Goldilocks::new(1), Goldilocks::new(2),
    Goldilocks::new(3), Goldilocks::new(4),
]);

// commit
let commitment = Brakedown::commit(&poly);

// evaluate at (0, 1) → f(0,1) = 3
let point = vec![Goldilocks::ZERO, Goldilocks::ONE];
let value = poly.evaluate(&point);

// open: prover generates proof
let mut prover_transcript = Transcript::new(b"example");
let proof = Brakedown::open(&poly, &point, &mut prover_transcript);

// verify: verifier checks proof (no access to polynomial)
let mut verifier_transcript = Transcript::new(b"example");
assert!(Brakedown::verify(&commitment, &point, value, &proof, &mut verifier_transcript));
```

### Ikat ring commitment

```rust
use cyb_lens_ikat::Ikat;
use jali::ring::RingElement;

// create ring elements (R_q polynomials)
let mut elem = RingElement::new(1024);
elem.coeffs[0] = nebu::Goldilocks::new(42);

// batch ring elements → NTT → single multilinear polynomial
let (commitment, poly) = Ikat::commit_rings(&[elem]);
```

### Assayer tropical witness

```rust
use cyb_lens_assayer::*;
use trop::Tropical;

// shortest path: 0 →3→ 1 →2→ 2, cost = 5
let witness = TropicalWitness {
    num_vertices: 3,
    edges: vec![
        Edge { from: 0, to: 1, weight: Tropical::from_u64(3) },
        Edge { from: 1, to: 2, weight: Tropical::from_u64(2) },
    ],
    assignment: vec![0, 1],
    cost: Tropical::from_u64(5),
    source: 0, target: 2,
};

// dual certificate: distance labels proving optimality
let cert = DualCertificate {
    dual_vars: vec![Goldilocks::new(0), Goldilocks::new(3), Goldilocks::new(5)],
    dual_objective: Goldilocks::new(5),
};

// verify: structural validity + cost correctness + dual feasibility
assert!(Assayer::verify_tropical(&witness, &cert));

// commit via Brakedown delegation
let (commitment, poly) = Assayer::commit_witness(&witness, &cert);
```

## workspace structure

```
lens/
├── core/           cyb-lens-core         Lens trait, Field, types, Transcript
├── brakedown/      cyb-lens-brakedown    Margulis expander + tensor decomposition
├── binius/         cyb-lens-binius       binary folding + Merkle tree
├── ikat/           cyb-lens-ikat         NTT batching → Brakedown
├── assayer/        cyb-lens-assayer      tropical witness → Brakedown
├── porphyry/       cyb-lens-porphyry     Brakedown over F_q (512-bit)
├── src/            cyber-lens            facade re-exports everything
├── nebu/           cyb-nebu              Goldilocks F_p arithmetic
│   ├── rs/         core library
│   ├── wgsl/       GPU backend
│   └── cli/        command-line tool
├── kuro/           cyb-kuro              F₂ binary tower
├── jali/           cyb-jali              polynomial ring R_q
├── trop/           cyb-trop              tropical semiring
├── genies/         cyb-genies            isogeny curves F_q
└── specs/          specifications
    ├── commitment.md    complete commitment layer spec
    ├── trait.md         Lens trait + naming conventions
    ├── scalar-field.md  Brakedown spec
    ├── binary-tower.md  Binius spec
    ├── polynomial-ring.md  Ikat spec
    ├── tropical-semiring.md  Assayer spec
    └── isogeny-curves.md  Porphyry spec
```

## dependency graph

```
         hemera
           ↓
      cyb-lens-core ←─── nox, zheng, bbg (consumers)
      ↓     ↓     ↓     ↓        ↓
  brakedown binius ikat assayer porphyry
      ↓       ↓     ↓    ↓  ↓      ↓
    nebu    kuro   jali trop brakedown genies
                     ↓
                   nebu
```

## tests

426 tests across the workspace:

| layer | tests |
|-------|-------|
| algebras (nebu, kuro, jali, trop, genies) | 352 |
| constructions (brakedown, binius, ikat, assayer, porphyry) | 46 |
| integration (cross-construction roundtrips, soundness) | 28 |
| total | 426 |

```bash
cargo test --workspace \
  --exclude nebu-wgsl --exclude kuro-wgsl \
  --exclude jali-wgsl --exclude trop-wgsl \
  --exclude genies-wgsl
```

## specs

the `specs/` directory contains the canonical specification:

- [commitment.md](specs/commitment.md) — types, trait, opening protocol, composition interface, crate structure, security parameters
- [trait.md](specs/trait.md) — Lens trait definition, naming conventions, dependency graph
- per-construction specs: [scalar-field](specs/scalar-field.md), [binary-tower](specs/binary-tower.md), [polynomial-ring](specs/polynomial-ring.md), [tropical-semiring](specs/tropical-semiring.md), [isogeny-curves](specs/isogeny-curves.md)

security: all security reduces to hemera (Poseidon2) collision resistance. no trusted setup, no pairings, no discrete log. post-quantum.

## license

cyber license: don't trust. don't fear. don't beg.
