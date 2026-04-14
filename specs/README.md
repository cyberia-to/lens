---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: nebu reference, nebu specification
diffusion: 0.00010722364868599256
springs: 0.0003405697993500444
heat: 0.00028171152949721216
focus: 0.0002121250700474493
gravity: 0
density: 7.29
---

# nebu specification

canonical reference for the Goldilocks prime field, its arithmetic, and its hardware.

## spec pages

| page | defines |
|------|---------|
| [[field]] | prime, elements, arithmetic, properties, why Goldilocks |
| [[ntt]] | Number Theoretic Transform, roots of unity, butterfly, Cooley-Tukey |
| [[encoding]] | 7-byte input encoding, 8-byte output encoding, padding, throughput |
| [[vectors]] | known-answer test vectors for all field operations |
| [[sqrt]] | square root, Legendre symbol, Tonelli-Shanks |
| [[batch]] | batch inversion, Montgomery's trick |
| [[fp2]] | quadratic extension F_{p²}, 128-bit security |
| [[fp3]] | cubic extension F_{p³}, recursive composition |
| [[fp4]] | quartic extension F_{p⁴}, 256-bit security, recursion tower |
| [[hardware]] | GFP primitives: fma, ntt, p2r, lut (proposal) |

## see also

- [[hemera]] — hash function over this field
- [[trident]] — language compiled to circuits over this field
- [[nox]] — VM executing over this field