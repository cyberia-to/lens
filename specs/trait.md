---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: polynomial commitments, polynomial commitment scheme, PCS, lens, lenses
---
# polynomial commitment scheme (lens)

the universal cryptographic primitive. commit to a polynomial, prove evaluations, verify without seeing the polynomial. one primitive serves proof commitment, state authentication, noun identity, and data availability.

a lens is how an algebra presents its work for verification. each algebra computes in its own structure — scalars, binary, rings, semirings, isogenies. the lens makes that computation verifiable. different structures need different optics. same laws of verification ([[SuperSpartan]] + [[sumcheck]]), different lenses.

## the interface

```
trait Lens<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;     // 32 bytes
    fn open(poly: &MultilinearPoly<F>, point: &[F]) -> Opening;
    fn verify(commitment: &Commitment, point: &[F], value: F, proof: &Opening) -> bool;
}
```

three operations. commit is O(N). open produces a proof. verify checks the proof. all transparent (no trusted setup), all post-quantum.

## five lenses

| lens | algebra | field | encoding | proof size | verify cost |
|------|---------|-------|----------|------------|-------------|
| [[expander-pcs\|Brakedown]] | [[nebu]] (+ nebu², nebu³, nebu⁴) | F_p (+ extensions) | expander-graph linear code | ~1.3 KiB | ~660 F_p ops |
| [[binary-pcs\|Binius]] | [[kuro]] | F₂ tower | binary Reed-Solomon | workload-dependent | ~660 F₂ ops |
| [[ring-pcs\|Ikat]] | [[jali]] | R_q | NTT-batched Brakedown | ~1.3 KiB (batched) | ring-structured |
| [[isogeny-pcs\|Isogeny]] | [[genies]] | F_q | Brakedown over F_q | ~1.3 KiB (wider) | ~660 F_q ops |
| [[tropical-pcs\|Tropical]] | [[trop]] | (min,+) | witness-verify via Brakedown | witness-proportional | dual certificate |

all five implement the same Lens trait. all fold into the same [[HyperNova]] accumulator via universal CCS with algebra selectors.

## three roles

the lens serves three roles — all using the same interface:

**1. proof commitment.** commit to [[nox]] execution trace. [[SuperSpartan]] verifies constraints via [[sumcheck]]. [[HyperNova]] folds into accumulator.

**2. state commitment.** BBG_poly (10 public dimensions), A(x) (private commitments), N(x) (nullifiers). BBG_root = H(commit(BBG_poly) ‖ commit(A) ‖ commit(N)).

**3. noun identity.** every [[nox]] noun is a multilinear polynomial. noun identity = hemera(Lens.commit(noun) ‖ domain_tag) — 32 bytes. axis = Lens opening. DAS = Lens opening at random positions.

one interface. five lenses. three roles.

see [[expander-pcs]] for the Brakedown lens. see [[binary-pcs]] for the Binius lens. see [[ring-pcs]] for the Ikat lens. see [[isogeny-pcs]] for the Isogeny lens. see [[tropical-pcs]] for the Tropical lens. see [[accumulator]] for how all five fold into one object.
