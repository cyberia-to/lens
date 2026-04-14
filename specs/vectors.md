# test vectors

known-answer values for genies operations. placeholder — vectors will be generated from the reference implementation.

## planned vectors

| test | input | output |
|------|-------|--------|
| fq_mul | two F_q elements | product mod q |
| fq_inv | F_q element | multiplicative inverse |
| fq_sqrt | quadratic residue in F_q | square root |
| point_add | two points on E_0 | sum point |
| scalar_mul | scalar k, point P | [k]P |
| isogeny_3 | E_0, kernel point of order 3 | codomain curve A' |
| isogeny_5 | E_0, kernel point of order 5 | codomain curve A' |
| action | exponent vector, E_0 | result curve A' |
| dh | two secret keys, E_0 | shared curve (must match) |
| encode_fq | F_q element | 64-byte LE encoding |
| encode_curve | Montgomery coefficient A | 64-byte encoding |
| fold | F_q element | 8 Goldilocks limbs |

## cross-verification

all vectors will be cross-verified against:
- sage reference implementation (SageMath CSIDH)
- the genies Rust implementation
- the nox jet implementation (when available)
