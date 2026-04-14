# why extension fields

nebu provides three extension fields over Goldilocks. each exists for a specific reason in the [[cyber]] proof stack.

## the tower

```
F_p                    64-bit     base field, all VM arithmetic
  ↑
F_{p²}  = F_p[u]/(u²−7)     128-bit    STARK verification challenges (FRI, WHIR)
  ↑
F_{p⁴}  = F_p[w]/(w⁴−7)     256-bit    deep recursion, long-lived commitments

F_{p³}  = F_p[t]/(t³−t−1)   192-bit    recursive composition (degree coprime to 2)
```

Fp2 and Fp4 form a tower: Fp4 = Fp2[v]/(v²−u). Fp3 stands alone — its degree is coprime to all powers of 2, which is exactly why it exists.

## Fp2: 128-bit STARK security

the base field F_p has ~64 bits. when a STARK verifier samples a random challenge, an attacker who guesses correctly can forge proofs. challenges from F_p give 64-bit security — too low.

F_{p²} doubles the security margin. FRI and WHIR sample challenges from F_{p²}, giving 128-bit soundness. this is the standard security target for interactive-to-non-interactive compiled proofs.

construction: u² = 7. since 7 is a quadratic non-residue in Goldilocks (7^((p−1)/2) = −1), x² − 7 is irreducible. every element is (re, im) with Karatsuba multiplication at 3 base muls.

## Fp3: recursive proof composition

when a STARK verifier runs inside another STARK, the inner proof's evaluation domain (powers of 2 — using the field's 2³² two-adicity) must not collide with the outer proof's challenge space. if both use extensions of degree 2^k, their roots of unity overlap.

Fp3 solves this: degree 3 is coprime to all powers of 2. inner-field roots of unity cannot be outer-field challenges. the algebraic separation is structural, not probabilistic.

construction: t³ = t + 1. the polynomial x³ − x − 1 has no roots in F_p (verified computationally), making it irreducible over F_p. schoolbook multiplication at 9 base muls with simple reduction (t³ → t + 1, t⁴ → t² + t).

## Fp4: deep recursion and 256-bit security

two use cases push beyond Fp2:

**recursion towers.** when multiple levels of recursive verification are stacked, each level needs fresh algebraic separation. the tower Fp → Fp2 → Fp4 provides this: Fp4 = Fp2[v]/(v²−u), so each layer extends the previous cleanly. inversion in Fp4 reduces to Fp2 operations via the tower norm.

**long-lived commitments.** polynomial commitments in [[bbg]] that anchor state for years need security margins beyond 128 bits. challenges from F_{p⁴} give 256-bit security — sufficient for commitments that must resist quantum-era attack budgets.

construction: w⁴ = 7. since 7 is a QNR, x⁴ − 7 is irreducible. the Frobenius endomorphism has a beautiful form: σ(w) = 2⁴⁸·w, making Frobenius evaluation nearly free (multiply odd coefficients by 2⁴⁸, negate even-degree terms).

## cost summary

| extension | mul cost | inv cost | use case |
|-----------|----------|----------|----------|
| Fp2 | 3 base muls (Karatsuba) | 1 base inv + 4 muls | FRI/WHIR challenges |
| Fp3 | 9 base muls (schoolbook) | 1 base inv + ~15 muls | recursive composition |
| Fp4 | 16 base muls (schoolbook) | 1 base inv + ~18 muls | deep recursion, 256-bit security |

all operations are constant-time with no secret-dependent branching. Rust and WGSL implementations share identical algorithms.

## see also

- [[fp2]] — Fp2 specification
- [[fp3]] — Fp3 specification
- [[fp4]] — Fp4 specification
- [[vectors]] — known-answer test vectors for all extensions
