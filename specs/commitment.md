---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: commitment layer, lens commitment, PCS layer
---
# commitment layer specification

polynomial commitment over five algebras. this document defines types, trait,
opening protocol, composition interface, and crate structure. per-construction
details live in the individual spec files: [[scalar-field]], [[binary-tower]],
[[polynomial-ring]], [[tropical-semiring]], [[isogeny-curves]].

the PCS zheng ships is Reed–Solomon over Goldilocks behind the
`MultilinearPcs` trait (`core/src/pcs.rs`, crate `cyber-lens-rspcs`):
[[whir]] (bake-off winner) and [[tensor-rs]] (Ligero geometry), hemera
Merkle trees, challenges and points in Fp3. Brakedown's expander-derived code
has no proven distance (lens#6); Brakedown and the constructions built on it
(Ikat, Assayer, Porphyry) are legacy and carry no proven opening soundness.

## 1. types

all types live in the `cyber-lens-core` crate (depends only on hemera).

### 1.1 scalar types

```rust
/// field scalar — supports full arithmetic including subtraction and inversion
trait Field: Copy + Eq + Add + Sub + Mul + Neg {
    const ZERO: Self;
    const ONE: Self;
    fn inv(self) -> Self;
}
```

the Lens trait requires `Field`. four algebras satisfy it directly:

| type | crate | size | identity |
|------|-------|------|----------|
| `Goldilocks` | nebu | 8 bytes | F_p, p = 2^64 - 2^32 + 1 |
| `F2_128` | kuro | 16 bytes | F₂ tower, char 2 |
| `Fq` | genies | 64 bytes | F_q, q = CSIDH-512 prime |
| `Goldilocks` | nebu (via jali NTT slots) | 8 bytes | Ikat operates on F_p scalars within the ring |

`Tropical` (trop, 8 bytes) is a semiring — it supports min (addition) and
saturating addition (multiplication) with identities +inf and 0. it has
no subtraction or inversion. Assayer does not implement `Lens<Tropical>`;
it is a wrapper protocol that accepts tropical inputs and delegates
commitment to `Lens<Goldilocks>` via Brakedown. see §5.4.

### 1.2 multilinear polynomial

the fundamental object being committed. a multilinear polynomial over ν variables
is defined by its evaluation table — 2^ν field elements.

```rust
struct MultilinearPoly<F: Field> {
    evals: Vec<F>,    // length 2^num_vars
    num_vars: usize,  // ν
}
```

evaluation at point r = (r₁, ..., r_ν):

$$f(r_1, \ldots, r_\nu) = \sum_{x \in \{0,1\}^\nu} f(x) \prod_{i=1}^{\nu} \big(x_i r_i + (1 - x_i)(1 - r_i)\big)$$

the multilinear extension — the unique degree-1-per-variable polynomial
that agrees with the evaluation table on the boolean hypercube.

### 1.3 commitment

a 32-byte binding digest of a polynomial. produced by `commit`, consumed by `verify`.

```rust
struct Commitment([u8; 32]);
```

binding property: computationally infeasible to find two distinct polynomials with
the same commitment. achieved via hemera hash of the encoded polynomial.

the format is identical across all constructions — always 32 bytes, always a
hemera hash. this uniformity allows commitments from different algebras to
compose in the same accumulator (see §6).

### 1.4 opening

a proof that a committed polynomial evaluates to a claimed value at a given point.
each construction produces structurally different proofs:

```rust
enum Opening {
    /// Brakedown, Ikat, Porphyry (legacy): tensor layout, column Merkle
    /// openings; shipped wire format in [[scalar-field]]
    Tensor {
        round_commitments: Vec<Commitment>,
        final_poly: Vec<u8>,
    },
    /// Binius: folding with Merkle authentication paths
    Folding {
        round_commitments: Vec<Commitment>,
        merkle_paths: Vec<Vec<[u8; 32]>>,
        final_value: Vec<u8>,
    },
    /// Assayer: tropical witness committed via Brakedown + dual certificate
    Witness {
        witness_commitment: Commitment,
        witness_opening: Box<Opening>,  // Tensor variant (Brakedown over F_p)
        certificate: Vec<u8>,           // LP dual feasibility data
    },
}
```

### 1.5 transcript

Fiat-Shamir transcript for non-interactive proofs. hemera sponge.

```rust
struct Transcript {
    state: [u8; 32],
}

impl Transcript {
    fn new(domain: &[u8]) -> Self;
    fn absorb(&mut self, data: &[u8]);
    fn squeeze(&mut self) -> [u8; 32];
    fn squeeze_field<F: Field>(&mut self) -> F;
}
```

all challenges derive from hemera. no other hash function in the protocol.

## 2. the lens trait

```rust
trait Lens<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;

    fn open(
        poly: &MultilinearPoly<F>,
        point: &[F],
        transcript: &mut Transcript,
    ) -> Opening;

    fn verify(
        commitment: &Commitment,
        point: &[F],
        value: F,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool;

    fn batch_open(
        poly: &MultilinearPoly<F>,
        points: &[(Vec<F>, F)],
        transcript: &mut Transcript,
    ) -> Opening;

    fn batch_verify(
        commitment: &Commitment,
        points: &[(Vec<F>, F)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool;
}
```

four constructions implement `Lens<F>` directly:

| construction | F | crate |
|-------------|---|-------|
| Brakedown | `Goldilocks` | cyber-lens-brakedown |
| Binius | `F2_128` | cyber-lens-binius |
| Porphyry | `Fq` | cyber-lens-porphyry |
| Ikat | `Goldilocks` | cyber-lens-ikat |

Assayer does not implement `Lens`. it is a separate protocol that accepts
tropical inputs and produces `Lens<Goldilocks>` artifacts via Brakedown
delegation. see §5.4.

### 2.1 security properties

binding: computationally infeasible to find two polynomials with the same
commitment. reduces to hemera collision resistance.

evaluation binding: the committed polynomial is fixed at commit time.

soundness: if poly(point) ≠ value, no efficient prover can produce a
proof that verify accepts, except with probability ≤ 2^(-λ).

transparency: no trusted setup. all parameters are public and deterministic.

post-quantum: relies on hash collision resistance only. no pairings,
no discrete log, no elliptic-curve assumptions.

### 2.2 complexity targets

| operation | prover cost | proof size | verifier cost |
|-----------|-------------|------------|---------------|
| commit | O(N) field ops | 32 bytes | — |
| open | O(N) field ops | O(log N + λ) elements | O(log N + λ) field ops |
| verify | — | — | O(log N + λ) field ops |
| batch_open (m points) | O(N + m·ν) | O(log N + λ) | O(log N + λ + m·ν) |

N = 2^ν (evaluation table size), λ = 128 (security parameter).

## 3. opening protocol

### 3.0 Reed–Solomon PCS (WHIR, TensorRs)

the shipped Goldilocks path. the evaluation table is encoded with an RS code,
the codeword is committed as a hemera Merkle tree, and an opening at an Fp3
point is proven by sumcheck-style folding rounds (WHIR) or one tensor row
combination (TensorRs), each checked at Fiat–Shamir query positions with
Merkle authentication paths. full protocols: [[whir]], [[tensor-rs]];
measurements: `audit/rs-whir-pcs-2026-10.md`.

### 3.1 tensor layout (Brakedown, Ikat, Porphyry — legacy)

the evaluation table is laid out as a matrix, each row encoded with the
expander-derived code, encoded columns committed in a hemera Merkle tree, and
an opening answers queried columns with their paths plus a weighted row
equation. the original sketch of this section (recursive tensor reduction with
unchecked round commitments) was not a binding opening and is withdrawn. the
code's distance is unproven (lens#6), so no soundness bound is claimed. wire
format: [[scalar-field]].

### 3.2 binary folding (Binius)

structurally similar to tensor decomposition but operates on packed binary matrices
with Merkle authentication paths instead of direct commitments. each round combines
rows over F₂¹²⁸ and produces Merkle auth paths for verification.

see [[binary-tower]] for the full folding protocol.

### 3.3 witness-verify (Assayer)

the prover runs the tropical optimization, packs the witness and dual certificate
as a Goldilocks polynomial, and opens via Brakedown. the verifier checks structural
validity, cost correctness, and optimality via the dual certificate.

see [[tropical-semiring]] for per-workload verification details.

### 3.4 verification

the verifier replays the transcript, recomputes every query position, checks
each queried symbol against the committed Merkle root with its authentication
path, and checks the round consistency equations of the construction down to
the claimed value. per-construction rules: [[whir]], [[tensor-rs]],
[[scalar-field]], [[binary-tower]].

### 3.5 batch opening

multiple openings amortized into one proof via the multilinear equality polynomial:

```
BATCH_OPEN(f, [(r₁, y₁), ..., (r_m, y_m)]):
    α = transcript.squeeze_field()
    r* = transcript.squeeze_field()
    combined_value = Σ α^i · y_i · eq(r_i, r*)
    return OPEN(f, r*), combined_value

eq(r, x) = Π_j (r_j x_j + (1 - r_j)(1 - x_j))
```

the verifier checks a single opening at r* and reconstructs the combined
claim from individual points and values. no polynomial division — the
eq polynomial is computable in O(ν) per point.

prover cost: O(N) for one opening + O(m · ν) for combining.

## 4. five constructions

each construction is specified in its own file. summary:

| construction | field | encoding | crate | spec |
|-------------|-------|----------|-------|------|
| WHIR | F_p (nebu), Fp3 points | Reed–Solomon, folding rounds | cyber-lens-rspcs | [[whir]] |
| TensorRs | F_p (nebu), Fp3 points | Reed–Solomon rows, Ligero geometry | cyber-lens-rspcs | [[tensor-rs]] |
| Brakedown (legacy) | F_p (nebu) | expander-derived code, distance unproven | cyber-lens-brakedown | [[scalar-field]] |
| Binius | F₂ tower (kuro) | binary Reed-Solomon, packed rows | cyber-lens-binius | [[binary-tower]] |
| Ikat | F_p (jali NTT slots) | NTT-batched Brakedown | cyber-lens-ikat | [[polynomial-ring]] |
| Assayer | F_p (Brakedown delegation) | tropical witness + dual certificate | cyber-lens-assayer | [[tropical-semiring]] |
| Porphyry | F_q (genies) | Brakedown over deep field | cyber-lens-porphyry | [[isogeny-curves]] |

### 4.1 Brakedown

legacy. expander-derived linear code over Goldilocks, tensor layout, column
Merkle tree. O(N) commit. its distance is unproven (lens#6) and it loses the
bake-off to WHIR, so no proof-size or verifier figure is claimed for it.
see [[scalar-field]].

### 4.2 Binius

binary-native commitment. 128 F₂ elements per u128 word. AND/XOR = 1 constraint
(vs ~32 in F_p). binary proofs verify in F_p circuits. see [[binary-tower]].

### 4.3 Ikat

ring-structured commitment for FHE workloads. the NTT decomposes R_q operations
into n independent Goldilocks operations — Ikat commits the NTT evaluation slots
(which are Goldilocks scalars) via batched Brakedown, exploiting ring structure
for NTT batching, automorphism arguments, and noise tracking. Ikat implements
`Lens<Goldilocks>`, operating on the NTT-slot representation of ring elements.
see [[polynomial-ring]].

### 4.4 Assayer

tropical semiring (min, +) has no subtraction or inversion — classical polynomial
commitment over trop is impossible. Assayer is a protocol wrapper: it accepts
tropical computation, produces an optimal assignment (witness) and LP dual
feasibility certificate, packs both as a Goldilocks polynomial, and commits via
Brakedown. three verification checks: structural validity, cost correctness,
dual optimality.

Assayer does not implement the `Lens` trait. its interface:

```rust
struct Assayer;

impl Assayer {
    /// accept tropical problem + solution, produce Brakedown commitment + certificate
    fn commit_witness(
        witness: &TropicalWitness,
        brakedown: &impl Lens<Goldilocks>,
    ) -> (Commitment, Opening);

    /// verify witness against commitment using dual certificate
    fn verify_witness(
        commitment: &Commitment,
        certificate: &DualCertificate,
        proof: &Opening,
        brakedown: &impl Lens<Goldilocks>,
    ) -> bool;
}
```

see [[tropical-semiring]].

### 4.5 Porphyry

Brakedown instantiated over F_q (512-bit CSIDH prime). same expander-graph
structure, wider field elements. hemera hashes serialized bytes — field-agnostic.
isogeny proofs verify in F_p circuits. see [[isogeny-curves]].

## 5. composition interface

lens provides commitments per-algebra. [[zheng]] handles composition
(Spartan over CCS, accumulation of evaluation claims, the decider's one
WHIR opening).

### 5.1 what lens guarantees

- commitment is always 32 bytes (hemera hash)
- opening protocol uses hemera Fiat-Shamir transcript
- verification is a pure function: (commitment, point, value, proof) → bool

lens does not know about accumulation — it produces and verifies individual commitments and openings.

### 5.2 recursion cost per algebra

when zheng verifies a lens opening inside a circuit:

| algebra | hemera in-circuit cost | notes |
|---------|----------------------|-------|
| F_p (nebu) | ~1,440 F_p constraints | native — hemera is Goldilocks Poseidon2 |
| F₂ (kuro) | ~142K binary constraints | simulating Goldilocks mul in bits |
| R_q (jali) | ~1,440 F_p constraints | NTT slots are F_p elements |
| (min,+) (trop) | ~1,440 F_p constraints | witness committed via Brakedown/F_p |
| F_q (genies) | prohibitive | simulating Goldilocks in 512-bit field |

the recursion boundary is always Goldilocks. accumulation is specified in
zheng `specs/accumulation.md` (landing with phase 3); recursion serves
composition only.

## 6. three roles

### 6.1 proof commitment (consumed by zheng)

```
commitment = Lens.commit(trace_poly)
proof = Lens.open(trace_poly, challenge_point)
```

zheng queries the commitment at random points. lens produces the opening.
the IOP protocol that determines which points to query is [[zheng]]'s domain.

### 6.2 state commitment (consumed by bbg)

```
BBG_root = hemera(Lens.commit(BBG_poly) ‖ Lens.commit(A) ‖ Lens.commit(N))
```

state queries are polynomial openings. batch opening covers N entries in one proof.

### 6.3 noun identity (consumed by nox)

```
noun_id = hemera(Lens.commit(noun_poly) ‖ domain_tag)    // 32 bytes
```

DAS: Lens openings at random positions verify data availability.

## 7. crate structure

```
lens/
├── core/           cyber-lens-core         trait + types + transcript
│                   depends on: hemera
├── brakedown/      cyber-lens-brakedown    impl Lens<Goldilocks>
│                   depends on: core, nebu
├── binius/         cyber-lens-binius       impl Lens<F2_128>
│                   depends on: core, kuro
├── ikat/           cyber-lens-ikat         impl Lens<Goldilocks>
│                   depends on: core, jali
├── assayer/        cyber-lens-assayer      wrapper (not Lens impl)
│                   depends on: core, trop, brakedown
├── porphyry/       cyber-lens-porphyry     impl Lens<Fq>
│                   depends on: core, genies
├── rspcs/          cyber-lens-rspcs        impl MultilinearPcs (WHIR, TensorRs)
│                   depends on: core, nebu, hemera
└── src/            cyber-lens              facade re-exports all
                    depends on: all above
```

consumers (nox, zheng, bbg) depend on `cyber-lens-core` for the trait.
they depend on a specific construction crate only when instantiating it.

## 8. dependency graph

```
         hemera
           ↓
      cyber-lens-core (trait, types, transcript)
      ↓          ↓          ↓          ↓          ↓
  brakedown   binius     ikat      assayer    porphyry
      ↓          ↓          ↓        ↓   ↓        ↓
    nebu       kuro       jali    trop brakedown  genies
                            ↓
                          nebu
```

## 9. security parameters

| parameter | value | meaning |
|-----------|-------|---------|
| λ | 128 | bits of security |
| hash | hemera (Poseidon2 over Goldilocks) | collision resistance |
| RS rate ρ (WHIR) | 1/16 lens default; 1/64 in zheng's shipped profile | codeword blowup |
| folding factor k (WHIR) | 4 | variables folded per round |
| grinding (WHIR) | 16 bits lens default; 24 in zheng's shipped profile | proof-of-work per challenge |
| Brakedown code (legacy) | — | distance unproven (lens#6); no parameter gives a proven bound |
| Fiat-Shamir | hemera sponge | non-interactive proof derivation |

security of the RS path: hemera collision resistance (Merkle binding,
Fiat–Shamir in the random-oracle model) plus proven RS proximity bounds,
computed per proof by `MultilinearPcs::security_bits`.

## 10. implementation status

| component | crate | status |
|-----------|-------|--------|
| nebu (F_p arithmetic) | strata-nebu | implemented, 154 tests |
| kuro (F₂ arithmetic) | strata-kuro | implemented, 87 tests |
| jali (R_q arithmetic) | strata-jali | implemented, 70 tests |
| trop (tropical arithmetic) | strata-trop | implemented, 87 tests |
| genies (F_q arithmetic) | strata-genies | implemented, 74 tests |
| Lens trait + types | cyber-lens-core | implemented |
| MultilinearPcs trait | cyber-lens-core | implemented |
| WHIR, TensorRs | cyber-lens-rspcs | implemented, see `audit/rs-whir-pcs-2026-10.md` |
| Brakedown | cyber-lens-brakedown | implemented, legacy (distance unproven) |
| Binius | cyber-lens-binius | implemented |
| Ikat | cyber-lens-ikat | implemented, legacy (Brakedown-based) |
| Assayer | cyber-lens-assayer | implemented, legacy (Brakedown-based) |
| Porphyry | cyber-lens-porphyry | implemented, legacy (Brakedown-based) |
| Transcript | cyber-lens-core | implemented |
