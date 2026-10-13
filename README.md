---
tags: cyber, core
alias: lens, lenses, polynomial commitment, PCS
crystal-type: entity
crystal-domain: crypto
---
# lens

polynomial commitment for [[cyber]]. commit to a polynomial, prove
evaluations, verify without seeing the polynomial.

the PCS zheng ships is Reed–Solomon over Goldilocks in `cyber-lens-rspcs`:
[[whir|WHIR]] (the bake-off winner) and [[tensor-rs|TensorRs]] (Ligero
geometry), both with hemera Merkle trees and Fp3 challenges. the five
per-algebra constructions below are the older family; Brakedown's expander
code has no proven distance (lens#6) and loses the bake-off, so it is legacy.

```
algebra (five fields) → hemera (hash) → lens (commitment) → nox → zheng → bbg
```

## how it works

a multilinear polynomial f over ν variables has 2^ν evaluations. the prover
encodes it with a Reed–Solomon code and commits the codeword as a hemera
Merkle tree (32-byte root). later, the prover can open the
commitment at any point — proving f(r) = y without revealing f. the verifier
checks the proof with no access to the polynomial, reading only the queried
codeword positions and their Merkle authentication paths.

the example below uses the legacy Brakedown `Lens` API; the shipped RS path
is `TensorRs` / `Whir` behind the `MultilinearPcs` trait (see
[specs/whir.md](specs/whir.md), [specs/tensor-rs.md](specs/tensor-rs.md)).

```rust
use cyb_lens_brakedown::*;
use nebu::Goldilocks;

// polynomial: f(x₁, x₂) with 4 evaluations
let poly = MultilinearPoly::new(vec![
    Goldilocks::new(1), Goldilocks::new(2),
    Goldilocks::new(3), Goldilocks::new(4),
]);

// commit → 32 bytes
let commitment = Brakedown::commit(&poly);

// open at (0, 1) → f(0,1) = 3
let point = vec![Goldilocks::ZERO, Goldilocks::ONE];
let value = poly.evaluate(&point);

// prover generates proof
let mut pt = Transcript::new(b"example");
let proof = Brakedown::open(&poly, &point, &mut pt);

// verifier checks (no access to polynomial)
let mut vt = Transcript::new(b"example");
assert!(Brakedown::verify(&commitment, &point, value, &proof, &mut vt));
```

the commitment is binding (can't change the polynomial after committing) and
the proof is sound (can't fake an evaluation). all security reduces to hemera
collision resistance — no trusted setup, no pairings, post-quantum.

## constructions

| construction | crate | algebra | encoding |
|-------------|-------|---------|----------|
| WHIR | [cyber-lens-rspcs](rspcs/) | Goldilocks, Fp3 points | RS codeword, hemera Merkle tree, folding rounds |
| TensorRs | [cyber-lens-rspcs](rspcs/) | Goldilocks, Fp3 points | RS rows (Ligero geometry), hemera Merkle tree |
| Brakedown (legacy) | [cyber-lens-brakedown](brakedown/) | Goldilocks (nebu) | Margulis-derived code (distance unproven) + tensor Merkle |
| Binius | [cyber-lens-binius](binius/) | F₂ tower (kuro) | binary folding + hemera Merkle tree |
| Ikat | [cyber-lens-ikat](ikat/) | R_q NTT slots (jali) | ring elements → NTT → batched Brakedown |
| Assayer | [cyber-lens-assayer](assayer/) | tropical → F_p | witness + dual certificate → Brakedown |
| Porphyry | [cyber-lens-porphyry](porphyry/) | F_q 512-bit (genies) | Brakedown over deep isogeny field |

four implement `Lens<F: Field>`. Assayer wraps Brakedown — the tropical semiring
has no subtraction, so the optimization witness is packed as Goldilocks elements.

## the trait

from [cyber-lens-core](core/) (depends on [strata-core](https://github.com/cyberia-to/strata) for the Field trait):

```rust
pub trait Lens<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;
    fn open(poly: &MultilinearPoly<F>, point: &[F], transcript: &mut Transcript) -> Opening;
    fn verify(commitment: &Commitment, point: &[F], value: F, proof: &Opening, transcript: &mut Transcript) -> bool;
    fn batch_open(poly: &MultilinearPoly<F>, points: &[(Vec<F>, F)], transcript: &mut Transcript) -> Opening;
    fn batch_verify(commitment: &Commitment, points: &[(Vec<F>, F)], proof: &Opening, transcript: &mut Transcript) -> bool;
}
```

## inside each construction

### WHIR and TensorRs

the shipped Goldilocks PCS. both encode with a Reed–Solomon code over
Goldilocks, commit the codeword as a hemera Merkle tree, draw challenges in
Fp3 and open queried positions with authentication paths. WHIR folds the
codeword k variables per round and wins the zheng bake-off (rate 1/64, k = 4,
24 grinding bits); TensorRs stays decodable until phase 5 of the proof-system
repair. security bits are computed from proven round-by-round soundness, no
conjectures. measurements: [audit/rs-whir-pcs-2026-10.md](audit/rs-whir-pcs-2026-10.md).

### Brakedown (legacy)

a sparse code derived from the Margulis expander graph encodes each row of a
tensor layout; encoded columns are committed in a hemera Merkle tree (tensor
v2) and opened column by column (opening v3). the code is injective but its
distance is not proven (lens#6), so Brakedown openings carry no proven
soundness bound. it is kept for existing consumers and is not the PCS zheng
ships.

### Binius

binary-native. kuro's F₂¹²⁸ elements pack 128 bits per machine word. folding
halves the polynomial using F₂¹²⁸ challenges. each round is committed via hemera
Merkle tree with authentication paths. AND/XOR cost 1 constraint each (vs ~32 in
F_p) — this is why binary workloads (quantized AI, comparison circuits) use Binius.

### Ikat

ring-aware. jali's R_q = F_p[x]/(x^n+1) decomposes via NTT into n independent
Goldilocks slots. Ikat converts ring elements to NTT form, batches the slots into
one multilinear polynomial, and commits via Brakedown. multiple ring polynomial
multiplies share one commitment.

```rust
use cyb_lens_ikat::Ikat;
use jali::ring::RingElement;

let elem = RingElement::new(1024);
let (commitment, poly) = Ikat::commit_rings(&[elem]);
```

### Assayer

tropical witness-verify. optimization problems (shortest path, assignment, Viterbi,
transport) produce a witness (the optimal solution) and a dual certificate (LP dual
variables proving no cheaper alternative exists). Assayer packs both as Goldilocks
elements and commits via Brakedown. verification checks three properties:

1. structural validity (assignment is a legal path/matching)
2. cost correctness (claimed cost = sum of assigned weights)
3. dual feasibility (distance labels satisfy triangle inequality)

```rust
use cyb_lens_assayer::*;

let witness = TropicalWitness { /* shortest path solution */ };
let cert = DualCertificate { /* distance labels */ };

assert!(Assayer::verify_tropical(&witness, &cert));
let (commitment, poly) = Assayer::commit_witness(&witness, &cert);
```

### Porphyry

Brakedown over genies' F_q (CSIDH-512 prime, 512 bits), legacy like Brakedown (code distance unproven). same expander + tensor
structure, wider field elements (64 bytes each). privacy workloads (stealth
addresses, VDF, blind signatures) prove natively in F_q without the 64× penalty
of encoding 512-bit operations as Goldilocks constraints.

## three consumers

| consumer | what it uses lens for |
|----------|---------------------|
| [[nox]] | noun identity: hemera(Lens.commit(noun_poly) ‖ tag) → 32 bytes |
| [[zheng]] | proof commitment: SuperSpartan queries Lens.open at random points |
| [[bbg]] | state root: BBG_root = hemera(Lens.commit(state) ‖ ...) |

## crates

```toml
# everything
[dependencies]
cyber-lens = "0.2.0"

# just the trait (for consumers)
[dependencies]
cyber-lens-core = "0.1.2"

# one construction
[dependencies]
cyber-lens-brakedown = "0.2.0"
```

| crate | what |
|-------|------|
| [cyber-lens-core](core/) | Lens trait, Commitment, Opening, Transcript, MultilinearPoly |
| [cyber-lens-rspcs](rspcs/) | WHIR and TensorRs: RS multilinear PCS over Goldilocks, hemera Merkle trees |
| [cyber-lens-brakedown](brakedown/) | legacy: Margulis-derived code (distance unproven) + tensor Merkle over F_p |
| [cyber-lens-binius](binius/) | binary folding + Merkle tree over F₂ |
| [cyber-lens-ikat](ikat/) | NTT batching → Brakedown over R_q slots |
| [cyber-lens-assayer](assayer/) | tropical witness-verify → Brakedown delegation |
| [cyber-lens-porphyry](porphyry/) | Brakedown over F_q (512-bit) |
| [cyber-lens](src/) | facade: re-exports core + all five |

## workspace

```
lens/
├── core/           cyber-lens-core         trait + types + transcript
├── brakedown/      cyber-lens-brakedown    20 tests
├── binius/         cyber-lens-binius       6 tests
├── ikat/           cyber-lens-ikat         5 tests
├── assayer/        cyber-lens-assayer      9 tests
├── porphyry/       cyber-lens-porphyry     6 tests
├── rspcs/          cyber-lens-rspcs        WHIR + TensorRs
├── src/            cyber-lens            28 integration tests
└── specs/          commitment layer spec
```

algebraic backends live in [strata](https://github.com/cyberia-to/strata)
(352 tests: nebu, kuro, jali, trop, genies).

## 76 tests

```bash
cargo test --workspace
```

## specs

- [commitment.md](specs/commitment.md) — types, trait, opening protocol, composition interface
- [trait.md](specs/trait.md) — Lens trait, naming conventions
- RS PCS: [whir](specs/whir.md), [tensor-rs](specs/tensor-rs.md)
- per-construction: [scalar-field](specs/scalar-field.md), [binary-tower](specs/binary-tower.md), [polynomial-ring](specs/polynomial-ring.md), [tropical-semiring](specs/tropical-semiring.md), [isogeny-curves](specs/isogeny-curves.md)

## license

cyber license: don't trust. don't fear. don't beg.
