---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: ring PCS, R_q PCS, jali PCS, ring-aware PCS
---
# ring PCS

the R_q PCS backend for [[zheng]]. polynomial ring workloads (FHE bootstrapping, lattice KEM, convolution) prove natively through their ring structure. the cyclotomic ring R_q = F_p[x]/(x^n+1) decomposes via NTT into n copies of F_p — ring-aware proving exploits this decomposition for batch commitments, automorphism arguments, and running noise accumulators.

implements the [[pcs|PCS]] trait with ring-structured constraint encodings over [[jali]]'s R_q arithmetic. the IOP layer ([[SuperSpartan]] + [[sumcheck]] + [[HyperNova]]) stays field-generic. [[hemera]] remains the only hash.

## architecture

```
zheng
├── IOP:          SuperSpartan + sumcheck     (field-generic, shared)
├── Composition:  HyperNova folding           (field-generic, shared)
├── Hash:         hemera                      (one hash, universal)
├── PCS₁:         Brakedown (Goldilocks)      (arithmetic workloads)
├── PCS₂:         Binius (F₂ tower)           (binary workloads)
├── PCS₃:         Ring-aware (R_q)            (FHE/lattice workloads) ← this
├── PCS₄:         Isogeny (F_q)              (privacy workloads)
└── PCS₅:         Tropical (min,+)           (optimization workloads)
```

zheng exposes a PCS trait. all five backends implement it:

```
trait PCS<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;
    fn open(poly: &MultilinearPoly<F>, point: &[F]) -> Opening;
    fn verify(commitment: &Commitment, point: &[F], value: F, proof: &Opening) -> bool;
}
```

## why ring-aware PCS

a generic SuperSpartan prover treats each NTT multiply as independent degree-2 constraints. it doesn't see that blind rotation performs n structured polynomial multiplies over the same ring. the 3072× cost gap (one R_q multiply = 3n F_p multiplies at n=1024) makes generic proving prohibitive for FHE workloads.

ring-aware PCS exploits three R_q structural properties:

### NTT batching

blind rotation: n polynomial multiplies. each = NTT → pointwise → INTT.

```
generic:     n independent commitments
             n × O(N log N) hemera hashes for Merkle trees
             n × O(N) constraints for NTT correctness

ring-aware:  1 batch commitment to the entire rotation
             the NTT structure IS the evaluation domain
             commit once, prove n multiplies together

savings:     ~log(n) factor on commitment
             ~n factor on NTT correctness (proved once, not n times)
```

### automorphism exploitation

key switching uses Galois automorphisms of R_q: x → x^{5^k}. these permute NTT evaluation points.

```
generic:     each rotation = n F_p constraints
             (re-prove the permuted polynomial from scratch)

ring-aware:  each rotation = 1 permutation argument
             (prove the permutation, not the result)
             cost: O(log n) constraints via LogUp permutation check

savings:     ~n/log(n) per rotation
```

### noise budget tracking

FHE correctness requires noise below a threshold. generic provers check noise bounds per-operation (range checks on coefficients).

```
generic:     per-operation range check
             ~64 constraints per coefficient per operation
             n coefficients × m operations = 64nm constraints

ring-aware:  running noise accumulator
             fold noise bound through computation
             check once at end (decider)
             cost: ~30 field ops per fold step

savings:     ~64nm / (30m) = ~2n per operation
```

## FHE bootstrapping workload

TFHE bootstrapping refreshes ciphertext noise. four phases, each with a different algebraic character:

| phase | operation | algebra | PCS |
|---|---|---|---|
| gadget decomposition | bit-split coefficients | F₂ (binary) | PCS₂ (Binius) |
| blind rotation | n polynomial multiplies in R_q | R_q (ring) | PCS₃ (this) |
| key switching | matrix × vector over F_p | F_p (matrix) | PCS₁ (Brakedown) |
| modulus switching | rescale coefficients | F_p (field) | PCS₁ (Brakedown) |

bootstrapping is a cross-algebra computation. each phase proves via the appropriate PCS backend. HyperNova folds across boundaries.

## jet library

recognized by formula hash, replace generic constraints with ring-structured ones:

```
ntt_batch(polys, n):
  recognize n polynomial multiplies sharing the same ring
  commit as single structured polynomial
  cost: O(n × N) instead of O(n × N log N)

key_switch(ct, ks_key, k):
  recognize key switching matrix-vector multiply
  exploit automorphism structure as permutation argument
  cost: O(k × log N) instead of O(k × N)

gadget_decomp(ct, base, levels):
  recognize binary decomposition of coefficients
  delegate to Binius PCS for binary sub-trace
  cost: O(N × levels) binary constraints (1 each in F₂)

noise_track(noise_bound, operation):
  fold noise bound into running accumulator
  check at bootstrapping boundary
  cost: ~30 field ops per fold
```

## cross-algebra bootstrapping flow

```
step 1: gadget_decomp(ct)          → PCS₂ (F₂, Binius)
         fold into accumulator       ~766 F_p constraints

step 2: blind_rotation(decomposed)  → PCS₃ (R_q, ring-aware)
         ntt_batch jet               ~n × N constraints (batched)
         fold into accumulator       ~766 F_p constraints

step 3: key_switch(rotated, ks)     → PCS₁ (F_p, Brakedown)
         key_switch jet              ~k × log(N) constraints
         fold into accumulator       ~766 F_p constraints

step 4: mod_switch(switched)        → PCS₁ (F_p, Brakedown)
         ~N constraints              (coefficient rescaling)
         fold into accumulator       ~766 F_p constraints

total composition overhead: ~3,064 F_p constraints (4 boundary crossings)
total computation: dominated by step 2 (blind rotation)
```

## hemera under FHE

computing hemera homomorphically (FHE-friendly hashing):

```
hemera multiplicative depth: 40 (x⁻¹ S-box, 16 partial rounds)
FHE noise growth:               5.4× less per hemera evaluation
bootstraps needed:              fewer → faster overall pipeline
```

ring-aware jets prove that hemera evaluation under FHE was computed correctly. hemera's reduced depth means the noise tracking accumulator triggers bootstrapping less often.

## cross-algebra composition

ring sub-traces fold into the shared F_p accumulator via HyperNova:

```
jali (R_q) computation → ring-aware proof
                              ↓
            HyperNova fold into shared F_p accumulator
            cost: ~766 F_p constraints per boundary crossing
```

universal CCS with selectors:

```
universal_ccs = {
  sel_Fp:   1 for Goldilocks rows
  sel_F2:   1 for binary rows
  sel_ring: 1 for ring-structured rows    ← this
  sel_Fq:   1 for isogeny rows
  sel_trop: 1 for tropical witness-verify rows
}
```

one accumulator covers all five algebras. the decider runs once in F_p.

## jali dependency

a separate repo **jali** (जाली) provides R_q polynomial ring arithmetic:
- ring element type (coefficient and NTT representations)
- polynomial multiply via NTT (3n nebu muls)
- Galois automorphisms (slot permutations)
- noise distributions and tracking
- depends only on nebu

zheng depends on jali for the Ring PCS backend. hemera remains the only hash throughout.

## open questions

1. **ring-structured CCS formalization**: encoding R_q operations as CCS matrices with ring structure — concrete CCS representation for batched NTT
2. **permutation argument for automorphisms**: LogUp permutation check in key switching context — Galois group structure may reduce LogUp cost further
3. **noise model precision**: running accumulator tracks worst-case — adaptive noise tracking (tighten bounds from measured noise) would improve throughput
4. **multi-key FHE**: threshold FHE bootstrapping with multiple keys — each key has separate automorphism group, jet recognition across key boundaries

see [[polynomial-commitment]] for Brakedown PCS, [[binary-pcs]] for F₂ backend, [[isogeny-pcs]] for F_q backend, [[tropical-pcs]] for (min,+) backend, [[recursion]] for cross-algebra folding
