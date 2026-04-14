# kuro specification

canonical reference for the F₂ tower field, its arithmetic, packed operations, and hardware targets.

## spec pages

| page | defines |
|------|---------|
| [field](field.md) | tower levels, all field operations, properties, cost model vs Goldilocks |
| [tower](tower.md) | tower construction, irreducible polynomials, representation choices, embeddings |
| [inversion](inversion.md) | Fermat inversion in GF(2^n), addition chains, tower-recursive formula |
| [packed](packed.md) | Packed128 operations, inner product kernel, BitNet and SpMV applications |
| [encoding](encoding.md) | bytes to tower elements, tower elements to bytes, canonical representation |
| [batch](batch.md) | batch inversion via Montgomery's trick for binary tower fields |
| [vectors](vectors.md) | known-answer test values for each tower level, edge cases |
| [hardware](hardware.md) | binary field processor design, SIMD utilization, GPU compute shaders |

## see also

- [nebu](https://github.com/mastercyb/nebu) — Goldilocks field arithmetic (the prime field counterpart)
- kuro is to F₂ what nebu is to Goldilocks
