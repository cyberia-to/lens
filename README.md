---
tags: cyber, core
alias: lens, lenses, polynomial commitment, PCS
crystal-type: entity
crystal-domain: crypto
---
# lens

polynomial commitment for [[cyber]]. five constructions over five
[[algebra|algebras]]. commit to a polynomial, prove evaluations,
verify without seeing the polynomial.

```
algebra (five fields) → hemera (hash) → lens (commitment) → nox → zheng → bbg
```

## how it works

a multilinear polynomial f over ν variables has 2^ν evaluations. the prover
commits the polynomial (one hemera hash). later, the prover can open the
commitment at any point — proving f(r) = y without revealing f. the verifier
checks the proof with no access to the polynomial.

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

## five constructions

each algebra has its own optimal commitment scheme:

| construction | crate | algebra | encoding |
|-------------|-------|---------|----------|
| Brakedown | [cyb-lens-brakedown](brakedown/) | Goldilocks (nebu) | Margulis expander + tensor decomposition |
| Binius | [cyb-lens-binius](binius/) | F₂ tower (kuro) | binary folding + hemera Merkle tree |
| Ikat | [cyb-lens-ikat](ikat/) | R_q NTT slots (jali) | ring elements → NTT → batched Brakedown |
| Assayer | [cyb-lens-assayer](assayer/) | tropical → F_p | witness + dual certificate → Brakedown |
| Porphyry | [cyb-lens-porphyry](porphyry/) | F_q 512-bit (genies) | Brakedown over deep isogeny field |

four implement `Lens<F: Field>`. Assayer wraps Brakedown — the tropical semiring
has no subtraction, so the optimization witness is packed as Goldilocks elements.

## the trait

from [cyb-lens-core](core/) (depends on [strata-core](https://github.com/cyberia-to/strata) for the Field trait):

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

### Brakedown

the primary construction. Margulis expander graph (algebraic, proven expansion
via Kazhdan's property T) encodes the polynomial into a codeword. hemera hashes
the codeword → 32-byte commitment. opening is recursive tensor decomposition:
each round halves the polynomial using an evaluation-point coordinate. proximity
testing at each round: 20 codeword positions (Fiat-Shamir derived) prove the
prover has the actual codeword, not a fake.

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

Brakedown over genies' F_q (CSIDH-512 prime, 512 bits). same expander + tensor
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
cyber-lens = "0.1"

# just the trait (for consumers)
[dependencies]
cyb-lens-core = "0.1"

# one construction
[dependencies]
cyb-lens-brakedown = "0.1"
```

| crate | what |
|-------|------|
| [cyb-lens-core](core/) | Lens trait, Commitment, Opening, Transcript, MultilinearPoly |
| [cyb-lens-brakedown](brakedown/) | Margulis expander + tensor decomposition over F_p |
| [cyb-lens-binius](binius/) | binary folding + Merkle tree over F₂ |
| [cyb-lens-ikat](ikat/) | NTT batching → Brakedown over R_q slots |
| [cyb-lens-assayer](assayer/) | tropical witness-verify → Brakedown delegation |
| [cyb-lens-porphyry](porphyry/) | Brakedown over F_q (512-bit) |
| [cyber-lens](src/) | facade: re-exports core + all five |

## workspace

```
lens/
├── core/           cyb-lens-core         trait + types + transcript
├── brakedown/      cyb-lens-brakedown    20 tests
├── binius/         cyb-lens-binius       6 tests
├── ikat/           cyb-lens-ikat         5 tests
├── assayer/        cyb-lens-assayer      9 tests
├── porphyry/       cyb-lens-porphyry     6 tests
├── src/            cyber-lens            28 integration tests
└── specs/          commitment layer spec
```

algebraic backends live in [strata](https://github.com/cyberia-to/strata)
(352 tests: nebu, kuro, jali, trop, genies).

## 74 tests

```bash
cargo test --workspace
```

## specs

- [commitment.md](specs/commitment.md) — types, trait, opening protocol, composition interface
- [trait.md](specs/trait.md) — Lens trait, naming conventions
- per-construction: [scalar-field](specs/scalar-field.md), [binary-tower](specs/binary-tower.md), [polynomial-ring](specs/polynomial-ring.md), [tropical-semiring](specs/tropical-semiring.md), [isogeny-curves](specs/isogeny-curves.md)

## license

cyber license: don't trust. don't fear. don't beg.
