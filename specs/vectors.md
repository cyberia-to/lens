---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: test vectors, known-answer tests, field vectors
diffusion: 0.00010722364868599256
springs: 0.0007414137239001096
heat: 0.0005436091679597342
focus: 0.0003847577751049711
gravity: 0
density: 0.14
---

# test vectors

known-answer tests for Goldilocks field arithmetic. generated from the [[hemera]] reference implementation. any conforming implementation must produce identical results.

```
p = 0xFFFFFFFF00000001
ε = 0x00000000FFFFFFFF
```

## canonical reduction

| input | canonical |
|---|---|
| `0x0000000000000000` | `0x0000000000000000` |
| `0x0000000000000001` | `0x0000000000000001` |
| `0xFFFFFFFF00000000` | `0xFFFFFFFF00000000` |
| `0xFFFFFFFF00000001` | `0x0000000000000000` |
| `0xFFFFFFFF00000002` | `0x0000000000000001` |
| `0xFFFFFFFFFFFFFFFF` | `0x00000000FFFFFFFE` |

p itself reduces to 0. values in [p, 2⁶⁴) subtract p once.

## addition

| a | b | a + b mod p |
|---|---|---|
| `0x0000000000000000` | `0x0000000000000000` | `0x0000000000000000` |
| `0x0000000000000001` | `0x0000000000000002` | `0x0000000000000003` |
| `0xFFFFFFFF00000000` | `0x0000000000000001` | `0x0000000000000000` |
| `0xFFFFFFFF00000000` | `0xFFFFFFFF00000000` | `0xFFFFFFFEFFFFFFFF` |
| `0x8000000000000000` | `0x8000000000000000` | `0x00000000FFFFFFFF` |
| `0x00000000FFFFFFFF` | `0x00000000FFFFFFFF` | `0x00000001FFFFFFFE` |

row 3: (p−1) + 1 = 0. row 4: (p−1) + (p−1) = p − 2. row 5: triggers u64 overflow, correction adds ε.

## subtraction

| a | b | a − b mod p |
|---|---|---|
| `0x0000000000000005` | `0x0000000000000003` | `0x0000000000000002` |
| `0x0000000000000000` | `0x0000000000000001` | `0xFFFFFFFF00000000` |
| `0x0000000000000000` | `0x0000000000000000` | `0x0000000000000000` |
| `0x0000000000000001` | `0xFFFFFFFF00000000` | `0x0000000000000002` |
| `0xFFFFFFFF00000000` | `0xFFFFFFFF00000000` | `0x0000000000000000` |

row 2: 0 − 1 = p − 1. row 4: 1 − (p−1) = 2.

## multiplication

| a | b | a × b mod p |
|---|---|---|
| `0x0000000000000003` | `0x0000000000000007` | `0x0000000000000015` |
| `0x0000000000000000` | `0x000000000000002A` | `0x0000000000000000` |
| `0x0000000000000001` | `0xFFFFFFFF00000000` | `0xFFFFFFFF00000000` |
| `0xFFFFFFFF00000000` | `0xFFFFFFFF00000000` | `0x0000000000000001` |
| `0xFFFFFFFF00000000` | `0x0000000000000002` | `0xFFFFFFFEFFFFFFFF` |
| `0x0000000012345678` | `0x000000009ABCDEF0` | `0x0B00EA4E242D2080` |

row 4: (p−1)² = 1. this is the key identity — the negative unit squares to the unit.

## S-box (x⁷)

| x | x⁷ mod p |
|---|---|
| `0x0000000000000000` | `0x0000000000000000` |
| `0x0000000000000001` | `0x0000000000000001` |
| `0x0000000000000002` | `0x0000000000000080` |
| `0x0000000000000007` | `0x00000000000C90F7` |
| `0xFFFFFFFF00000000` | `0xFFFFFFFF00000000` |
| `0x00000000DEADBEEF` | `0xF49CB716AE41CF92` |
| `0x123456789ABCDEF0` | `0xA480968CDE68DB72` |

row 3: 2⁷ = 128. row 5: (p−1)⁷ = p−1 (odd power of −1 is −1).

## negation

| x | −x mod p |
|---|---|
| `0x0000000000000000` | `0x0000000000000000` |
| `0x0000000000000001` | `0xFFFFFFFF00000000` |
| `0xFFFFFFFF00000000` | `0x0000000000000001` |
| `0x000000000000002A` | `0xFFFFFFFEFFFFFFD7` |
| `0x8000000000000000` | `0x7FFFFFFF00000001` |

neg is self-inverse: −(−x) = x. row 2 and row 3 demonstrate this.

## primitive root

| expression | value |
|---|---|
| 7^(p−1) mod p | `0x0000000000000001` |
| 7^((p−1)/2) mod p | `0xFFFFFFFF00000000` |
| 2^((p−1)/2) mod p | `0x0000000000000001` |
| 3^((p−1)/2) mod p | `0x0000000000000001` |
| 5^((p−1)/2) mod p | `0x0000000000000001` |
| 6^((p−1)/2) mod p | `0x0000000000000001` |

7^((p−1)/2) = p−1 ≠ 1, confirming 7 is a quadratic non-residue and thus a generator. candidates 2, 3, 5, 6 all yield 1, confirming they are quadratic residues (not generators).

## inversion

| a | a⁻¹ mod p |
|---|---|
| `0x0000000000000001` | `0x0000000000000001` |
| `0x0000000000000002` | `0x7FFFFFFF80000001` |
| `0xFFFFFFFF00000000` | `0xFFFFFFFF00000000` |

row 1: 1⁻¹ = 1. row 2: 2⁻¹ = (p+1)/2. row 3: (p−1)⁻¹ = p−1, since (p−1)² = 1.

verification: a · a⁻¹ mod p = 1 for each row.

## roots of unity

| expression | value | note |
|---|---|---|
| ω₂ = 7^((p−1)/2) | `0xFFFFFFFF00000000` | = p−1 = −1. ω₂² = 1 ✓ |
| ω₂⁻¹ = ω₂^(2−1) | `0xFFFFFFFF00000000` | −1 is its own inverse |
| 2⁻¹ mod p | `0x7FFFFFFF80000001` | N⁻¹ scaling factor for length-2 INTT |

the 2-nd root of unity is −1. this is the base case for every NTT: the length-2 butterfly is (a+b, a−b).

## square root

| a | legendre(a) | sqrt(a) |
|---|---|---|
| `0x0000000000000000` | `0x0000000000000000` | `0x0000000000000000` |
| `0x0000000000000001` | `0x0000000000000001` | `0x0000000000000001` |
| `0x0000000000000004` | `0x0000000000000001` | `0x0000000000000002` |
| `0x0000000000000009` | `0x0000000000000001` | `0x0000000000000003` |
| `0x0000000000000002` | `0x0000000000000001` | `0x000000FFFEFFFF00` |
| `0x0000000000000007` | `0xFFFFFFFF00000000` | ⊥ (no root) |

row 3: sqrt(4) = 2. row 5: sqrt(2) exists because 2 is a QR. row 6: 7 is a QNR — no square root.

verification: r² mod p = a for each row with a root.

## batch inversion

| input | output |
|---|---|
| `[3, 5, 7]` | `[0xAAAAAAAA00000001, 0xCCCCCCCC00000001, 0x249249246DB6DB6E]` |

verification: a[i] · result[i] mod p = 1 for each element. the batch result matches individual inversions (see § inversion for inv(1), inv(2); these extend the pattern).

## extension field

F_{p²} = F_p[u] / (u² − 7). elements written as (re, im) representing re + im·u.

### extension multiplication

| x | y | x · y |
|---|---|---|
| (2, 3) | (4, 5) | (`0x71`, `0x16`) |
| (`0x123456789ABCDEF0`, `0xFEDCBA9876543210`) | (`0xAAAAAAAA`, `0x55555555`) | (`0x25ED096D7B425EDC`, `0xD7CC6BAE7839A5C3`) |

row 1: (2+3u)(4+5u) = (8+105) + (10+12)u = 113 + 22u = (0x71, 0x16).

### extension inversion

| x | x⁻¹ |
|---|---|
| (2, 3) | (`0x49C341156822B63D`, `0x115B1E5F63CBEEA5`) |

verification: x · x⁻¹ = (1, 0).

### extension conjugate and norm

| x | conj(x) | norm(x) |
|---|---|---|
| (1, 1) | (1, p−1) | `0xFFFFFFFEFFFFFFFB` |

norm(1+u) = 1 − 7 = p − 6 = 0xFFFFFFFEFFFFFFFB.

## cubic extension field

F_{p³} = F_p[t] / (t³ − t − 1). elements written as (c0, c1, c2) representing c0 + c1·t + c2·t².

### cubic multiplication

| x | y | x · y |
|---|---|---|
| (2, 3, 5) | (4, 7, 11) | (76, 149, 118) |

expansion: d=[8,26,63,68,55], reduce via t³=t+1: c0=8+68=76, c1=26+68+55=149, c2=63+55=118.

### cubic inversion

| x | x⁻¹ |
|---|---|
| (2, 3, 5) | roundtrip: x · x⁻¹ = (1, 0, 0) |

norm(2, 3, 5) = 67. adjugate/67 gives the inverse.

## quartic extension field

F_{p⁴} = F_p[w] / (w⁴ − 7). elements written as (c0, c1, c2, c3) representing c0 + c1·w + c2·w² + c3·w³.

### quartic multiplication

| x | y | x · y |
|---|---|---|
| (2, 3, 5, 11) | (4, 7, 13, 17) | (1359, 1622, 1376, 152) |

expansion: d=[8,26,67,152,193,228,187], reduce via w⁴=7: c0=8+7·193=1359, c1=26+7·228=1622, c2=67+7·187=1376, c3=152.

### quartic inversion

| x | x⁻¹ |
|---|---|
| (2, 3, 5, 11) | roundtrip: x · x⁻¹ = (1, 0, 0, 0) |

tower inversion: Fp2-norm, invert in Fp2, scale conjugate.

### Fp2 embedding

| Fp2 input | Fp4 embedding | Fp2 product | Fp4 product |
|---|---|---|---|
| (2, 3) × (4, 5) | (2, 0, 3, 0) × (4, 0, 5, 0) | (113, 22) | (113, 0, 22, 0) |

Fp2 arithmetic is a special case of Fp4 arithmetic.

### quartic Frobenius

σ⁴(x) = x for all x ∈ F_{p⁴}. Frobenius constant: 7^((p−1)/4) = 2⁴⁸.