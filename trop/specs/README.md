# trop specification

canonical reference for tropical semiring arithmetic: the (min, +) semiring, its matrix algebra, and dual certificate verification.

## spec pages

| page | defines |
|------|---------|
| [semiring](semiring.md) | tropical semiring axioms, (min, +) definition, identity elements, idempotency |
| [matrix](matrix.md) | tropical matrix multiplication, power, eigenvalue, determinant, rank |
| [kleene](kleene.md) | Kleene star (tropical closure), A* = I ⊕ A ⊕ ... ⊕ A^(n-1) |
| [eigenvalue](eigenvalue.md) | tropical eigenvalue, critical cycle, Karp's characterization |
| [determinant](determinant.md) | tropical determinant, optimal assignment value, tropical rank |
| [dual](dual.md) | (max, +) dual semiring, negation duality |
| [verification](verification.md) | LP duality certificates for optimality proofs over F_p |
| [encoding](encoding.md) | F_p ↔ tropical element conversion, +∞ sentinel, matrix serialization |
| [vectors](vectors.md) | known-answer test values, edge cases |

## design boundary

trop defines arithmetic. algorithms that use trop arithmetic (Dijkstra, Hungarian, Viterbi, Sinkhorn) are programs that execute on [[nox]] with optional jet acceleration. this separation mirrors nebu: nebu defines field ops, not FFT — NTT is a nox program that calls nebu arithmetic.

## see also

- [nebu](https://github.com/cyberia-to/nebu) — Goldilocks field (verification backbone)
- [nox](https://github.com/cyberia-to/nox) — VM (tropical algorithms execute as nox programs)
