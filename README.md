# nebu

Goldilocks field arithmetic for [[cyber]]. the single prime that unifies the entire stack — from virtual machine execution to hash functions to polynomial commitments to proof generation.

```
p = 2⁶⁴ - 2³² + 1 = 18,446,744,069,414,584,321
```

## why Goldilocks

**native 64-bit arithmetic.** every field element is a u64. multiplication produces a u128, reduced back to u64 without division — just shifts and adds. the reduction identity `2⁶⁴ ≡ 2³² - 1 (mod p)` makes this three operations after the multiply. branchless, constant-time, no secret-dependent control flow.

**massive two-adicity.** `p - 1` has factor `2³²`, giving NTT (Number Theoretic Transform) over domains up to 4 billion points. polynomial evaluation and interpolation — the core operations of STARK proving — run at native speed on roots of unity that exist naturally in the field.

**7-byte absorption.** field elements encode unconditionally in 7 bytes, giving [[hemera]] a 56-byte absorption rate at sponge width 8. no rejection sampling. no variable-length encoding. constant-time stream processing from bytes to field elements.

**one field everywhere.** every layer of [[cyber]] speaks Goldilocks: [[nox]], [[hemera]], [[zheng]], [[bbg]], [[mudra]]. one field means one implementation to audit, one set of test vectors, one hardware target.

## what nebu provides

### field operations

six core operations matching [[nox]] Layer 1 field patterns:

| op | what | cost |
|----|------|------|
| add | modular addition | 1 add + conditional subtract |
| sub | modular subtraction | 1 sub + conditional add |
| mul | modular multiplication | 1 u128 multiply + reduction |
| inv | field inverse via Fermat | ~96 multiplications via addition chain |
| eq | equality | 1 comparison |
| lt | ordering | 1 comparison |

### transforms

| operation | method |
|-----------|--------|
| NTT forward | Cooley-Tukey, decimation-in-time |
| NTT inverse | Gentleman-Sande, decimation-in-frequency |
| batch inversion | Montgomery's trick, `n-1` multiplications for `n` elements |

radix-2 with bit-reversal permutation. primitive root g = 7. typical proving domains: 2¹⁸–2²⁴ points.

### extension fields

| extension | construction | security | use case |
|-----------|-------------|----------|----------|
| F_{p²} | F_p[u]/(u²−7) | 128-bit | STARK verification challenges |
| F_{p³} | F_p[t]/(t³−t−1) | 192-bit | recursive proof composition |
| F_{p⁴} | F_p[w]/(w⁴−7) | 256-bit | deep recursion, long-lived commitments |

F_{p²} and F_{p⁴} form a tower: Fp4 = Fp2[v]/(v²−u). F_{p³} has degree coprime to 2, separating inner/outer evaluation domains in recursive STARKs.

### encoding and roots

| operation | method |
|-----------|--------|
| encoding | 7-byte LE input → field element, 8-byte canonical output |
| square roots | Tonelli-Shanks with Legendre symbol |

## the stack

every butterfly in an NTT, every round of [[hemera]], every field operation in a [[nox]] reduction, every polynomial evaluation in a [[zheng]] proof — all of it is nebu arithmetic over the same prime.

| repo | role | depends on nebu via | github |
|------|------|---------------------|--------|
| [[hemera]] | hash function | Poseidon2 permutation over field elements | [hemera](https://github.com/cyberia-to/hemera) |
| [[nox]] | virtual machine | 6 field patterns map directly to nebu ops | [nox](https://github.com/cyberia-to/nox) |
| [[zheng]] | proof system | NTT, polynomial evaluation, commitment arithmetic | [zheng](https://github.com/cyberia-to/zheng) |
| [[bbg]] | authenticated state | NMT hashing, polynomial commitments | [bbg](https://github.com/cyberia-to/bbg) |
| [[mudra]] | communication primitives | NTT for TFHE, field arithmetic for key exchange | [mudra](https://github.com/cyberia-to/mudra) |
| [[trident]] | language compiler | compiles to nox field patterns | [trident](https://github.com/cyberia-to/trident) |

## license

Cyber License: Don't trust. Don't fear. Don't beg.
