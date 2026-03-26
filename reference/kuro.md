---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: binary PCS, F2 tower PCS, Binius PCS
---
# binary PCS

the F₂ tower PCS backend for [[zheng]]. binary-native polynomial commitment over [[kuro]]'s tower: F₂ → F₂² → F₂⁴ → ... → F₂¹²⁸. bitwise operations (XOR, AND, NOT, SHL, LT) cost 1 constraint each — 32-64× cheaper than encoding them in [[Goldilocks field|Goldilocks]] where bit decomposition forces 32-64 constraints per operation.

implements the [[pcs|PCS]] trait for F₂ tower fields. the IOP layer ([[SuperSpartan]] + [[sumcheck]] + [[HyperNova]]) stays field-generic. only the polynomial commitment scheme changes. [[hemera]] remains the only hash — used externally for commitment binding and Fiat-Shamir, never proved inside binary circuits.

## architecture

```
zheng
├── IOP:          SuperSpartan + sumcheck     (field-generic, shared)
├── Composition:  HyperNova folding           (field-generic, shared)
├── Hash:         hemera                      (one hash, universal)
├── PCS₁:         Brakedown (Goldilocks)      (arithmetic workloads)
├── PCS₂:         Binius (F₂ tower)           (binary workloads)
├── PCS₃:         Ring-aware (R_q)            (FHE/lattice workloads)
├── PCS₄:         Isogeny (F_q)              (privacy workloads)
└── PCS₅:         Tropical (min,+)           (optimization workloads)
```

zheng exposes a PCS trait. Brakedown and Binius both implement it:

```
trait PCS<F: Field> {
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;
    fn open(poly: &MultilinearPoly<F>, point: &[F]) -> Opening;
    fn verify(commitment: &Commitment, point: &[F], value: F, proof: &Opening) -> bool;
}
```

## binary tower fields

F₂ → F₂² → F₂⁴ → F₂⁸ → F₂¹⁶ → F₂³² → F₂⁶⁴ → F₂¹²⁸

each extension defined by irreducible polynomial over the previous level. tower structure enables:
- 128 F₂ elements packed in one u128 machine word
- SIMD-native operations (64x data parallelism vs Goldilocks)
- Karatsuba multiplication over tower levels

## commitment

polynomial evaluations arranged in a matrix. rows packed into machine words. hemera Merkle tree over packed rows.

```
commit(poly over F₂):
  1. arrange evaluations in √n × √n matrix
  2. pack each row into machine words (128 F₂ elements per u128)
  3. Reed-Solomon extend rows over F₂¹²⁸
  4. build hemera Merkle tree over extended rows
  5. root = hemera hash (same as Brakedown commitment format)
```

hemera operates on bytes — it doesn't care that the bytes represent binary field elements. same hemera::hash_node, same tree primitives, same 32-byte root.

## opening

Binius folding: each round halves the polynomial by combining rows using a random F₂¹²⁸ challenge.

```
open(poly, point):
  for each folding round:
    squeeze challenge from hemera Fiat-Shamir transcript
    fold polynomial (halve rows)
    commit folded polynomial (hemera Merkle)
  return final value + all round commitments + Merkle auth paths
```

## verification

```
verify(commitment, point, value, proof):
  replay Fiat-Shamir (hemera sponge)
  check folding consistency (F₂ field arithmetic)
  check Merkle auth paths (hemera)
  check final evaluation
```

the verifier uses hemera for hashing and F₂ for field ops. no binary-native hash needed.

## cost comparison

| operation | F_p (Brakedown) | F₂ (Binius) | ratio |
|---|---|---|---|
| AND | ~32 constraints | 1 constraint | 32x |
| XOR | ~32 constraints | 1 constraint | 32x |
| lt (comparison) | ~64 constraints | ~1 constraint | 64x |
| field multiply | 1 constraint | ~64 constraints | 0.016x |
| hemera hash | ~736 constraints | N/A (external) | — |

binary wins for bitwise. Goldilocks wins for arithmetic. the prover chooses based on workload.

## primary workloads

two workloads dominate the binary regime:
- **quantized AI inference**: BitNet-style 1-bit models. matrix-vector multiply = XOR + popcount. a 4096x4096 layer = ~16M binary constraints (vs ~512M F_p constraints)
- **tri-kernel SpMV**: quantized axon weights for π iteration over the cybergraph. each of 5-8 iterations is a massive binary workload

## cross-algebra composition

a mixed nox program (field + bitwise) partitions into sub-traces:

```
nox<F_p> sub-trace → zheng<Brakedown> proof
nox<F₂>  sub-trace → zheng<Binius> proof
                           ↓
            HyperNova fold into shared F_p accumulator
            cost: ~30 F_p field ops + 1 hemera hash per fold
```

universal CCS with selectors enables heterogeneous folding:

```
universal_ccs = {
  sel_Fp:  1 for Goldilocks rows, 0 otherwise
  sel_F2:  1 for binary rows, 0 otherwise
}
```

one accumulator covers both algebras. the decider runs once in F_p (hemera-native).

## recursion boundary

binary proofs are NOT verified inside binary circuits. verification crosses to Goldilocks:

```
binary execution → binary proof (hemera Merkle + FS externally)
                        ↓
    verify in F_p circuit (hemera native, ~736 constraints)
                        ↓
    fold into F_p accumulator
```

hemera inside F₂ circuit: ~142K binary constraints per hash (simulating Goldilocks mul in bits). unacceptable for recursion. hemera inside F_p circuit: ~736 constraints. the recursion boundary is Goldilocks.

## kuro dependency

a separate repo **kuro** (黒) provides F₂ tower arithmetic:
- F₂ base field operations (XOR = add, AND = mul)
- tower extension arithmetic (F₂² through F₂¹²⁸)
- packed SIMD operations (128 elements per u128)
- no hemera dependency, no nebu dependency
- pure binary algebra

zheng depends on kuro for the Binius PCS backend. hemera remains the only hash throughout.

## open questions

1. **packed trace layout**: nox trace spec (16 registers × 2ⁿ rows) needs binary variant with 128-bit packing
2. **cross-algebra value transfer**: quantize/dequantize at F_p ↔ F₂ boundary — concrete constraint cost
3. **optimal partition granularity**: how large should binary sub-traces be before boundary overhead dominates
4. **kuro SIMD strategy**: AVX-512 (512 elements), AVX2 (256 elements), or portable u128

see [[polynomial-commitment]] for the Brakedown PCS, [[recursion]] for cross-algebra folding, [[sumcheck]] for the shared IOP
