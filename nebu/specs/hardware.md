---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: GFP, Goldilocks Field Processor, hardware primitives
status: proposal
diffusion: 0.0021435396809766157
springs: 0.00018951670935407302
heat: 0.0008060749147709153
focus: 0.0012898398362486962
gravity: 26
density: 1.74
---

# hardware specification

**status: proposal** — design intent for the [[GFP]] (Goldilocks Field Processor). no silicon exists yet. ISA encodings, timing models, and register file layout are not specified. this page defines the target instruction set for future hardware, not a buildable specification.

the [[GFP]] — four hardware primitives optimized for the Goldilocks field.

## primitives

| primitive | operation | signature | purpose |
|-----------|-----------|-----------|---------|
| `fma` | field multiply-accumulate | (a, b, c) → a + b·c | matrix operations, polynomial evaluation |
| `ntt` | NTT butterfly | (a, b, ω) → (a + ω·b, a − ω·b) | polynomial arithmetic, STARK proving |
| `p2r` | Poseidon2 round | state[16] → state[16] | [[hemera]] hashing |
| `lut` | lookup table | (index, table) → value | cross-domain operations |

## fma — field multiply-accumulate

the fundamental arithmetic primitive. every field operation reduces to `fma`:

```
add(a, b) = fma(a, b, 1)     // a + b·1
mul(a, b) = fma(0, a, b)     // 0 + a·b
```

matrix-vector multiplication (the dominant operation in Poseidon2's linear layers) is a sequence of `fma` calls. the accumulator stays in a register — no intermediate writes to memory.

## ntt — NTT butterfly

see [[ntt]] for the full specification. the `ntt` instruction fuses one multiplication and two additions into a single operation, eliminating the multiply-then-add pipeline stall.

## p2r — Poseidon2 round

a single `p2r` instruction executes one full or partial round of the Poseidon2 permutation:

- read 16-element state from registers
- apply S-box (x⁷) to the appropriate elements
- apply the linear layer (M_E or M_I)
- add round constants
- write 16-element state back to registers

the entire [[hemera]] permutation (72 rounds) is 72 `p2r` instructions. round constants are loaded from a dedicated constant ROM.

## lut — lookup table

a configurable lookup table that serves all four computational domains. one table, one circuit, one verification cost. see [[rosetta stone]] for how a single lookup table over the Goldilocks field serves ZK proofs, AI inference, FHE, and quantum simulation simultaneously.