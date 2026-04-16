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
    /// Brakedown, Ikat, Porphyry: recursive tensor decomposition
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

### 3.1 tensor decomposition (Brakedown, Ikat, Porphyry)

```
OPEN(f, r = (r₁, ..., r_ν)):
    q₀ = tensor_reduce(f, r)        // √N elements
    C₁ = commit(q₀)
    α₁ = transcript.squeeze_field()

    q₁ = tensor_reduce(q₀, α₁)     // N^{1/4} elements
    C₂ = commit(q₁)
    ...

    q_d has ≤ λ elements             // d = log log N rounds
    send q_d directly

    return Opening::Tensor { [C₁, ..., C_d], q_d }

tensor_reduce(g, r):
    for i in 0..len(g)/2:
        g'[i] = g[2i] + r · (g[2i+1] - g[2i])
    return g'
```

each round squares the compression ratio. prover: O(N). proof: O(log N + λ) elements.

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

```
VERIFY(C₀, r, y, proof):
    reconstruct transcript
    for round i = 0..d-1:
        r_i = transcript.squeeze_field()
        check C_{i+1} consistent with tensor reduction at r_i
    check q_d evaluates to y at composed point
```

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
| Brakedown | F_p (nebu) | expander-graph linear code | cyber-lens-brakedown | [[scalar-field]] |
| Binius | F₂ tower (kuro) | binary Reed-Solomon, packed rows | cyber-lens-binius | [[binary-tower]] |
| Ikat | F_p (jali NTT slots) | NTT-batched Brakedown | cyber-lens-ikat | [[polynomial-ring]] |
| Assayer | F_p (Brakedown delegation) | tropical witness + dual certificate | cyber-lens-assayer | [[tropical-semiring]] |
| Porphyry | F_q (genies) | Brakedown over deep field | cyber-lens-porphyry | [[isogeny-curves]] |

### 4.1 Brakedown

the primary construction. expander-graph linear code over Goldilocks.
O(N) commit, ~1.3 KiB proof, ~660 field ops to verify. see [[scalar-field]].

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
(HyperNova accumulator, CCS selectors, folding).

### 5.1 what lens guarantees

- commitment is always 32 bytes (hemera hash)
- opening protocol uses hemera Fiat-Shamir transcript
- verification is a pure function: (commitment, point, value, proof) → bool

lens does not know about folding — it produces and verifies individual commitments.

### 5.2 recursion cost per algebra

when zheng verifies a lens opening inside a circuit:

| algebra | hemera in-circuit cost | notes |
|---------|----------------------|-------|
| F_p (nebu) | ~736 F_p constraints | native — hemera is Goldilocks Poseidon2 |
| F₂ (kuro) | ~142K binary constraints | simulating Goldilocks mul in bits |
| R_q (jali) | ~736 F_p constraints | NTT slots are F_p elements |
| (min,+) (trop) | ~736 F_p constraints | witness committed via Brakedown/F_p |
| F_q (genies) | prohibitive | simulating Goldilocks in 512-bit field |

the recursion boundary is always Goldilocks. folding protocol is specified
in [[zheng/specs/recursion]].

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
| expander degree d | 20-30 | encoding density for 128-bit security |
| expansion factor c | 2-4 | codeword blowup |
| Fiat-Shamir | hemera sponge | non-interactive proof derivation |

all security reduces to hemera collision resistance.

## 10. implementation status

| component | crate | status |
|-----------|-------|--------|
| nebu (F_p arithmetic) | cyb-nebu | implemented, 73 tests |
| kuro (F₂ arithmetic) | cyb-kuro | implemented, 77 tests |
| jali (R_q arithmetic) | cyb-jali | implemented, 70 tests |
| trop (tropical arithmetic) | cyb-trop | implemented, 77 tests |
| genies (F_q arithmetic) | cyb-genies | implemented, 55 tests |
| Lens trait + types | cyber-lens-core | specified, not implemented |
| Brakedown | cyber-lens-brakedown | specified, not implemented |
| Binius | cyber-lens-binius | specified, not implemented |
| Ikat | cyber-lens-ikat | specified, not implemented |
| Assayer | cyber-lens-assayer | specified, not implemented |
| Porphyry | cyber-lens-porphyry | specified, not implemented |
| Transcript | cyber-lens-core | specified, not implemented |
