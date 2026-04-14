# genies specification

canonical reference for isogeny group action arithmetic: F_q field operations, supersingular curves, isogeny computation, and class group action.

## spec pages

| page | defines |
|------|---------|
| [prime](prime.md) | CSIDH prime form, selection criteria, concrete parameters, F_q arithmetic |
| [curves](curves.md) | supersingular elliptic curves over F_q, point operations, j-invariant |
| [isogeny](isogeny.md) | l-isogeny computation, Velu formulas, kernel polynomials |
| [class-group](class-group.md) | class group cl(O), ideal representation, mathematical structure |
| [action](action.md) | class group action algorithm, cost model, constant-time considerations |
| [encoding](encoding.md) | F_q element serialization, curve point encoding, secret key format |
| [verification](verification.md) | F_q to F_p folding for zheng proofs |
| [vectors](vectors.md) | known-answer test values for all operations |

## see also

- [nebu](https://github.com/cyberia-to/nebu) — Goldilocks field (proof accumulator)
- [kuro](https://github.com/cyberia-to/kuro) — binary field (complementary regime)
- [mudra](https://github.com/cyberia-to/mudra) — protocols built on genies (CSIDH, VRF, VDF, threshold, stealth, blind)
