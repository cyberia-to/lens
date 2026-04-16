---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: commitment layer, lens commitment, PCS layer
---
# commitment layer specification

the complete specification for lens — polynomial commitment over five algebras.
this document defines types, trait, constructions, composition, and integration
with the cyber stack.

## 1. types

### 1.1 scalar types

each algebra defines its own scalar type. two trait levels:

```rust
/// any algebra's scalar — sufficient for commitment
trait Algebra: Copy + Eq {
    const ZERO: Self;
    const ONE: Self;
    fn add(self, rhs: Self) -> Self;
    fn mul(self, rhs: Self) -> Self;
}

/// field scalars — adds subtraction and inversion
trait Field: Algebra + Sub + Neg {
    fn inv(self) -> Self;
}
```

four algebras satisfy `Field`. the tropical semiring satisfies only `Algebra`
(no additive inverse — see §5.4). the Lens trait is generic over `Algebra`,
not `Field`, so that Assayer can participate.

concrete implementations:

| type | crate | size | trait | identity |
|------|-------|------|-------|----------|
| `Goldilocks` | nebu | 8 bytes | Field | F_p, p = 2^64 - 2^32 + 1 |
| `F2_128` | kuro | 16 bytes | Field | F₂ tower, char 2 |
| `RingElement` | jali | 32 KiB (fixed MAX_N=4096 array, n active) | Field | R_q = F_p[x]/(x^n+1) |
| `Tropical` | trop | 8 bytes | Algebra | (min, +) semiring |
| `Fq` | genies | 64 bytes | Field | F_q, q = CSIDH-512 prime |

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

this is the multilinear extension — the unique degree-1-per-variable polynomial
that agrees with the evaluation table on the boolean hypercube.

### 1.3 commitment

a 32-byte binding digest of a polynomial. produced by `commit`, consumed by `verify`.

```rust
struct Commitment([u8; 32]);
```

binding property: computationally infeasible to find two distinct polynomials with
the same commitment. achieved via hemera hash of the encoded polynomial.

the commitment format is identical across all five constructions — always 32 bytes,
always a hemera hash. this uniformity is what allows commitments from different
algebras to compose in the same accumulator.

### 1.4 opening

a proof that a committed polynomial evaluates to a claimed value at a given point.
structure varies by construction.

```rust
enum Opening {
    /// Brakedown, Porphyry: recursive tensor decomposition
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
    /// Assayer: tropical witness + dual certificate, committed via Brakedown
    Witness {
        witness_commitment: Commitment,
        witness_opening: Box<Opening>,  // Tensor variant for the witness poly
        certificate: Vec<u8>,           // LP dual feasibility data
    },
}
```

the enum reflects that different constructions produce structurally different proofs.
the Commitment type (§1.3) is uniform across all variants — always 32 bytes.

### 1.5 transcript

Fiat-Shamir transcript for non-interactive proofs. uses hemera as the hash.

```rust
struct Transcript {
    state: [u8; 32],  // hemera sponge state
}

impl Transcript {
    fn new(domain: &[u8]) -> Self;
    fn absorb(&mut self, data: &[u8]);             // feed data in
    fn squeeze(&mut self) -> [u8; 32];             // extract challenge
    fn squeeze_field<F: Field>(&mut self) -> F;    // extract field challenge
}
```

all challenges derive from hemera. no other hash function appears anywhere
in the protocol.

## 2. the lens trait

```rust
trait Lens<F: Algebra> {
    /// commit to a multilinear polynomial.
    /// returns a 32-byte hemera digest.
    /// cost: O(N) field operations where N = 2^num_vars.
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;

    /// produce a proof that poly(point) = value.
    /// the verifier will check this against the commitment.
    fn open(
        poly: &MultilinearPoly<F>,
        point: &[F],
        transcript: &mut Transcript,
    ) -> Opening;

    /// check a proof that a committed polynomial evaluates to value at point.
    /// returns true iff the proof is valid.
    fn verify(
        commitment: &Commitment,
        point: &[F],
        value: F,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool;

    /// amortize multiple openings into one proof.
    /// critical for batch verification of nox traces and bbg state queries.
    fn batch_open(
        poly: &MultilinearPoly<F>,
        points: &[(Vec<F>, F)],  // (point, claimed_value) pairs
        transcript: &mut Transcript,
    ) -> Opening;

    /// verify a batch opening.
    fn batch_verify(
        commitment: &Commitment,
        points: &[(Vec<F>, F)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool;
}
```

### 2.1 security properties

binding: given commitment C, it is computationally infeasible to find
(point, value₁, proof₁) and (point, value₂, proof₂) with value₁ ≠ value₂
where both proofs verify. security reduces to hemera collision resistance.

evaluation binding: the committed polynomial is fixed at commit time.
the prover cannot change which polynomial the commitment refers to.

soundness: if poly(point) ≠ value, no efficient prover can produce a
proof that verify accepts, except with negligible probability 2^(-λ).

transparency: no trusted setup. all parameters are public and deterministic.
the expander graph is explicitly constructed, not sampled from a trapdoor.

post-quantum: security relies on hash collision resistance (hemera),
not discrete log or factoring. no pairing-based or elliptic-curve assumptions.

### 2.2 complexity targets

| operation | prover cost | proof size | verifier cost |
|-----------|-------------|------------|---------------|
| commit | O(N) field ops | 32 bytes | — |
| open | O(N) field ops | O(log N + λ) elements | O(log N + λ) field ops |
| verify | — | — | O(log N + λ) field ops |
| batch_open (m points) | O(N + m·ν) | O(log N + λ) | O(log N + λ + m·ν) |

where N = 2^ν (evaluation table size), λ = 128 (security parameter).

## 3. encoding layer

the encoding is what differentiates the five constructions. it sits between
the raw polynomial and the commitment.

```
polynomial → ENCODE → codeword → hemera(codeword) → commitment
```

### 3.1 expander-graph code (Brakedown — for nebu, genies)

```
encode(v ∈ F^N):
    w = E · v           // sparse matrix-vector multiply
    return w ∈ F^{cN}   // expansion factor c ≈ 2-4

E: bipartite expander graph
   |L| = N, |R| = cN, left-degree d ≈ 20-30
   expansion property: ∀ S ⊆ L with |S| ≤ δN,
     |Γ(S)| ≥ (1-ε)d|S|
```

commit = hemera(w). the expander property guarantees that any sufficiently
large subset of codeword positions determines the original polynomial.

used by: Brakedown (over F_p via nebu), Porphyry (over F_q via genies).

### 3.2 binary Reed-Solomon (Binius — for kuro)

```
encode(v ∈ F₂^N):
    arrange v as √N × √N matrix
    pack rows into u128 words (128 F₂ elements per word)
    Reed-Solomon extend each row over F₂¹²⁸
    return extended matrix
```

commit = hemera Merkle tree over extended rows.

the packing is the key insight: 128 F₂ elements fit in one machine word,
giving 128× data parallelism over scalar field approaches.

### 3.3 NTT-batched code (Ikat — for jali)

```
encode(ring_polys ∈ R_q^m):
    apply NTT to each polynomial (shared twiddle factors)
    batch all NTT evaluations into one commitment
    return batched NTT matrix
```

commit = hemera(batched NTT evaluations). the NTT structure IS the evaluation
domain — ring multiplication decomposes into n independent F_p multiplications,
and the commitment covers all n simultaneously.

### 3.4 witness encoding (Assayer — for trop)

trop is not a field. polynomial commitment in the classical sense is impossible.
the resolution: tropical computation produces a witness, and the witness is
committed via Brakedown (§3.1) over F_p.

```
encode(tropical_computation):
    1. run optimization (Dijkstra, Hungarian, Viterbi, etc.)
    2. extract WITNESS: optimal assignment + cost
    3. construct CERTIFICATE: LP dual feasibility proof
    4. pack (witness, certificate) as F_p polynomial
    5. commit via Brakedown
```

verification checks three properties in F_p:
- structural validity (assignment is legal)
- cost correctness (claimed cost matches assignment)
- optimality (dual certificate proves no cheaper alternative)

### 3.5 deep-field code (Porphyry — for genies)

Brakedown instantiated over F_q (512-bit prime) instead of F_p (64-bit).
same expander-graph structure, wider field elements.

```
encode(v ∈ F_q^N):
    w = E · v            // same expander, F_q arithmetic
    serialize to bytes    // 64 bytes per F_q element
    return w
```

commit = hemera(serialized codeword). hemera operates on bytes — it is
field-agnostic. the commitment format is identical to Brakedown.

## 4. opening protocol

### 4.1 recursive tensor decomposition

Brakedown, Ikat, and Porphyry use the same opening structure (expander-graph
encoding + tensor reduction). Binius uses a structurally similar protocol
(recursive halving) but with binary-specific folding and Merkle authentication
paths — see [[binary-tower]] for details. Assayer uses witness-verify (§3.4).

the generic tensor decomposition protocol (Brakedown/Ikat/Porphyry):

```
OPEN(f, r = (r₁, ..., r_ν)):
    // round 0
    q₀ = tensor_reduce(f, r)
    C₁ = commit(q₀)
    absorb C₁ into transcript
    α₁ = transcript.squeeze_field()

    // round 1
    q₁ = tensor_reduce(q₀, α₁)
    C₂ = commit(q₁)
    ...

    // round d = log log N
    q_d has ≤ λ elements
    send q_d directly

    return Opening { [C₁, ..., C_d], q_d, transcript }

tensor_reduce(g, r):
    // halve the polynomial using challenge r
    for i in 0..len(g)/2:
        g'[i] = g[2i] + r · (g[2i+1] - g[2i])
    return g'
```

each round squares the compression ratio. after d = log log N rounds,
the polynomial is small enough to send directly.

### 4.2 verification

```
VERIFY(C₀, r, y, proof):
    reconstruct transcript from proof
    for round i = 0..d-1:
        r_i = transcript.squeeze_field()
        check C_{i+1} is consistent with tensor reduction of C_i at r_i
    check q_d evaluates to y at the composed point
    return all checks passed
```

verifier cost: d hash checks + final evaluation = O(log log N × hash_cost + λ × field_ops).

### 4.3 batch opening

multiple openings amortized into one proof via random linear combination:

```
BATCH_OPEN(f, [(r₁, y₁), ..., (r_m, y_m)]):
    α = transcript.squeeze_field()

    // reduce m opening claims to 1 via random evaluation
    // claim: f(r_i) = y_i for all i
    // combined claim: g(r*) = Σ α^i · y_i
    //   where g = Σ α^i · eq(r_i, ·) · f(·)
    //   and eq(r, x) = Π_j (r_j x_j + (1-r_j)(1-x_j))  (multilinear equality)

    r* = transcript.squeeze_field()
    combined_value = Σ_{i=1}^{m} α^i · y_i · eq(r_i, r*)
    return OPEN(f, r*), combined_value
```

the verifier checks a single opening at r* and reconstructs the combined
claim from the individual points and values. no polynomial division required —
the reduction uses the multilinear equality polynomial eq(r, ·) which is
computable in O(ν) per point.

prover cost: O(N) for the single opening + O(m · ν) for combining claims.
when m << N, this is dominated by O(N).

critical for:
- nox trace verification (thousands of constraint checks → 1 proof)
- bbg state queries (N entries → 1 proof)
- DAS (20 random samples → 1 proof)

## 5. five constructions

### 5.1 Brakedown (scalar field — nebu)

the primary construction. arithmetic workloads.

| parameter | value |
|-----------|-------|
| field | F_p, p = 2^64 - 2^32 + 1 (Goldilocks) |
| encoding | expander-graph linear code, d ≈ 20-30 |
| commit cost | O(N) nebu muls, ~40 ms at N = 2²⁰ |
| proof size | ~1.3 KiB |
| verify cost | ~660 nebu muls, ~5 μs |
| binding | 1 hemera call |
| extensions | Fp2, Fp3, Fp4 via nebu extension towers |

primary workloads: tri-kernel SpMV, state transitions, arithmetic circuits.

### 5.2 Binius (binary tower — kuro)

binary-native commitment. bitwise workloads.

| parameter | value |
|-----------|-------|
| field | F₂ tower (F₂ → F₂¹²⁸) |
| encoding | binary Reed-Solomon over F₂¹²⁸ |
| commit cost | hemera Merkle over packed rows |
| proof size | workload-dependent (folding rounds vary) |
| verify cost | ~660 kuro ops + hemera Merkle checks |
| packing | 128 F₂ elements per u128 word |

cost advantage: AND = 1 constraint (vs ~32 in F_p), XOR = 1 constraint,
comparison = ~1 constraint (vs ~64 in F_p).

primary workloads: quantized AI inference (BitNet 1-bit models),
tri-kernel SpMV with quantized weights.

recursion boundary: binary proofs verify in F_p circuits (hemera is F_p native).

### 5.3 Ikat (polynomial ring — jali)

ring-structured commitment. FHE and lattice workloads.

| parameter | value |
|-----------|-------|
| field | R_q = F_p[x]/(x^n+1), n up to 4096 |
| encoding | NTT-batched Brakedown |
| commit cost | O(n × N) (batched, not n × O(N log N)) |
| proof size | ~1.3 KiB (batched) |
| verify cost | ring-structured (amortized across n slots) |

three structural optimizations:
- NTT batching: n polynomial multiplies share one commitment (~log(n) savings)
- automorphism exploitation: Galois rotations as permutation arguments (~n/log(n) savings)
- noise budget tracking: running accumulator checked once at boundary (~2n savings per op)

primary workloads: TFHE bootstrapping (blind rotation, key switching, gadget decomposition).

cross-algebra: FHE bootstrapping spans F₂ (gadget decomp → Binius), R_q (blind rotation → Ikat),
F_p (key switching → Brakedown). composition across algebra boundaries is handled by [[zheng]].

### 5.4 Assayer (tropical semiring — trop)

witness-verify commitment. optimization workloads.

trop is not a field. the tropical semiring (min, +) has no additive inverse —
min(a, b) cannot be undone. classical polynomial commitment is impossible.

resolution: the prover runs the optimization, extracts a witness (optimal assignment + cost),
constructs a dual feasibility certificate, and commits the witness via Brakedown over F_p.

| parameter | value |
|-----------|-------|
| computation field | (min, +) via trop |
| commitment field | F_p via Brakedown (delegated) |
| witness | optimal assignment + cost |
| certificate | LP dual variables satisfying strong duality |
| verify cost | O(\|problem\|) F_p constraints |

verification checks:
1. structural validity: assignment is a legal solution
2. cost correctness: claimed cost = sum of assigned weights
3. optimality: dual certificate proves no cheaper alternative exists

per-workload specs:

| workload | tropical cost | verification cost (F_p) |
|----------|--------------|------------------------|
| shortest path | O(\|E\| log \|V\|) | O(\|V\| + \|E\|) |
| assignment (n×n) | O(n³) | O(n²) |
| Viterbi (S states, T steps) | O(S² × T) | O(S² × T) |
| optimal transport (n×n) | O(n³ log n) | O(n²) |

### 5.5 Porphyry (isogeny curves — genies)

deep-field commitment. privacy workloads.

| parameter | value |
|-----------|-------|
| field | F_q, q = CSIDH-512 prime (512 bits) |
| encoding | Brakedown over F_q |
| commit cost | O(N) genies muls |
| proof size | ~1.3 KiB (wider field elements) |
| verify cost | ~660 genies muls |
| element size | 64 bytes (8 × u64 limbs) |

why not non-native: encoding F_q ops as F_p constraints costs (512/64)² = 64× per op.
isogeny walks involve thousands of F_q ops. 64× × thousands is prohibitive.

primary workloads: CSIDH key exchange, VDF verification, blind signatures,
ring signatures, verifiable random functions.

recursion boundary: isogeny proofs verify in F_p circuits (hemera is F_p native).

## 6. composition interface

a single nox program may use multiple algebras. lens provides commitments
per-algebra; [[zheng]] handles composition into a single verifiable object
(HyperNova accumulator, CCS selectors, folding).

### 6.1 what lens guarantees to the composer

lens provides a uniform interface across all five constructions:
- commitment is always 32 bytes (hemera hash)
- opening protocol uses hemera Fiat-Shamir transcript
- verification is a pure function: (commitment, point, value, proof) → bool

this uniformity is what allows zheng to fold commitments from different
algebras into one accumulator. lens does not know about folding — it only
produces and verifies individual commitments.

### 6.2 recursion constraint: hemera cost per algebra

when zheng verifies a lens opening inside a circuit (for recursive composition),
the cost depends on the algebra:

| algebra | hemera inside circuit | notes |
|---------|----------------------|-------|
| F_p (nebu) | ~736 F_p constraints | native — hemera is Goldilocks Poseidon2 |
| F₂ (kuro) | ~142K binary constraints | simulating Goldilocks mul in bits |
| R_q (jali) | ~736 F_p constraints | NTT slots are F_p elements |
| (min,+) (trop) | ~736 F_p constraints | witness is committed via Brakedown/F_p |
| F_q (genies) | prohibitive | simulating Goldilocks in 512-bit field |

consequence: non-Goldilocks openings verify in F_p circuits. the recursion
boundary is always Goldilocks. this is a lens property that zheng relies on,
but the folding protocol itself is specified in [[zheng/specs/recursion]].

## 7. three roles

the same trait serves three consumers. each role uses commit/open/verify
with different polynomial content.

### 7.1 proof commitment (consumed by zheng)

commit to a nox execution trace encoded as a multilinear polynomial.

```
trace: 16 registers × 2^ν rows → multilinear polynomial
commitment = Lens.commit(trace_poly)
proof = Lens.open(trace_poly, challenge_point)
```

zheng queries the commitment at random points. lens produces the opening proof.
the IOP protocol (SuperSpartan, sumcheck) that determines which points to query
is specified in [[zheng]] — lens only commits and opens.

### 7.2 state commitment (consumed by bbg)

commit to authenticated state. three polynomials:

```
BBG_poly:  10 public dimensions (balance, karma, focus, ...)
A(x):      private commitment accumulator
N(x):      nullifier set

BBG_root = hemera(
    Lens.commit(BBG_poly) ‖
    Lens.commit(A) ‖
    Lens.commit(N)
)
```

state queries are polynomial openings: prove that BBG_poly(account_index) = balance
without revealing the full state. batch opening covers N entries in one proof.

### 7.3 noun identity (consumed by nox)

every nox noun is a multilinear polynomial. noun identity is derived from its commitment:

```
noun_id = hemera(Lens.commit(noun_poly) ‖ domain_tag)    // 32 bytes
```

axis (noun decomposition): a Lens opening at a structural point.
DAS (data availability sampling): Lens openings at random positions — if k random
openings verify, the full polynomial is available with high probability.

## 8. dependency graph

```
         hemera (hash — binding, Fiat-Shamir, Merkle)
           ↓
         lens (this spec)
        / | | \  \
    nebu kuro jali trop genies    (arithmetic backends)
           ↓
    ┌──────┼──────┐
   nox   zheng   bbg              (consumers)
```

lens depends on:
- hemera: hash function (commitment binding, Fiat-Shamir challenges, Merkle trees)
- nebu: F_p field arithmetic
- kuro: F₂ tower arithmetic
- jali: R_q ring arithmetic (depends on nebu)
- trop: tropical semiring arithmetic
- genies: F_q isogeny arithmetic

lens is consumed by:
- nox: noun identity via Lens.commit
- zheng: proof verification via Lens.commit/open/verify (zheng adds SuperSpartan + sumcheck on top)
- bbg: state authentication via Lens.commit

## 9. security parameters

| parameter | value | meaning |
|-----------|-------|---------|
| λ | 128 | bits of security |
| hash | hemera (Poseidon2 over Goldilocks) | collision resistance |
| expander degree d | 20-30 | encoding density for 128-bit security |
| expansion factor c | 2-4 | codeword blowup |
| Fiat-Shamir | hemera sponge | non-interactive proof derivation |

all security reduces to hemera collision resistance. no structured reference strings,
no pairing assumptions, no discrete log.

## 10. implementation status

| component | status |
|-----------|--------|
| nebu (F_p arithmetic) | implemented, 73 tests |
| kuro (F₂ arithmetic) | implemented, 77 tests |
| jali (R_q arithmetic) | implemented, 70 tests |
| trop (tropical arithmetic) | implemented, 77 tests |
| genies (F_q arithmetic) | implemented, 55 tests |
| Lens trait | specified, not implemented |
| Brakedown (expander encoding) | specified, not implemented |
| Binius (binary RS encoding) | specified, not implemented |
| Ikat (NTT-batched encoding) | specified, not implemented |
| Assayer (witness-verify) | specified, not implemented |
| Porphyry (deep-field encoding) | specified, not implemented |
| Transcript (Fiat-Shamir) | specified, not implemented |
