# tower construction specification

how and why the F₂ tower is built from seven successive quadratic extensions.

## why quadratic extensions

the tower uses exclusively degree-2 extensions for three reasons:

1. **Karatsuba multiplication.** a degree-2 extension requires 3 sub-multiplications (Karatsuba). degree-3 would require 6 (Toom-Cook). degree-4 as two nested degree-2 requires 9 = 3². the quadratic tower minimizes multiplication cost at every level.

2. **Binary alignment.** quadratic extensions double the bit width: 1 -> 2 -> 4 -> 8 -> 16 -> 32 -> 64 -> 128. each level fits exactly in a power-of-two machine word. no wasted bits, no packing overhead.

3. **Recursive structure.** every algorithm (mul, inv, frobenius, trace, norm) decomposes identically at every level. one implementation pattern, seven instantiations.

## irreducible polynomial selection

every extension uses the same form:

```
f_k(x) = x² + x + alpha_k
```

where alpha_k is the canonical generator of level k (the element `0b10`).

### why x² + x + alpha (not x² + alpha)

in characteristic 2, x² + c is *never* irreducible -- it always factors as (x + sqrt(c))² since squaring is the Frobenius. the form x² + x + c is irreducible over GF(2^n) if and only if Tr(c) = 1, where Tr is the absolute trace to F₂.

### irreducibility verification

| level | extension | alpha_k | Tr(alpha_k) | irreducible? |
|-------|-----------|---------|-------------|-------------|
| 0->1 | F₂ -> F₂² | 1 (= alpha_0) | 1 | yes |
| 1->2 | F₂² -> F₂⁴ | 0b10 (= x in F₂²) | 1 | yes |
| 2->3 | F₂⁴ -> F₂⁸ | 0b0010 (= x in F₂⁴) | 1 | yes |
| 3->4 | F₂⁸ -> F₂¹⁶ | 0x02 (= x in F₂⁸) | 1 | yes |
| 4->5 | F₂¹⁶ -> F₂³² | 0x0002 | 1 | yes |
| 5->6 | F₂³² -> F₂⁶⁴ | 0x00000002 | 1 | yes |
| 6->7 | F₂⁶⁴ -> F₂¹²⁸ | 0x0000000000000002 | 1 | yes |

the trace of the canonical generator `0b10` at each level equals 1 because the tower is constructed so that this invariant holds recursively.

## representation

each element of F_{2^{2^k}} is stored as 2^k bits in the smallest fitting unsigned integer type:

```
level  bits  rust type   mask
-----  ----  ---------   ----
0      1     u8          0x01
1      2     u8          0x03
2      4     u8          0x0F
3      8     u8          0xFF
4      16    u16         0xFFFF
5      32    u32         0xFFFFFFFF
6      64    u64         0xFFFFFFFFFFFFFFFF
7      128   u128        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
```

### bit layout

an element a = a_lo + a_hi * x is stored with a_lo in the low bits and a_hi in the high bits:

```
F₂⁴ element a = a_lo + a_hi * x, where a_lo, a_hi in F₂²:
  bits [0:2) = a_lo    (F₂² element, 2 bits)
  bits [2:4) = a_hi    (F₂² element, 2 bits)

F₂¹²⁸ element a = a_lo + a_hi * x, where a_lo, a_hi in F₂⁶⁴:
  bits [0:64)   = a_lo  (F₂⁶⁴ element, 64 bits)
  bits [64:128) = a_hi  (F₂⁶⁴ element, 64 bits)
```

this convention is consistent at every level. extraction uses shift-and-mask; recombination uses shift-and-or.

## embedding maps

every subfield embeds canonically into every superfield. the embedding is the identity on bit representations:

```
embed: F_{2^k} -> F_{2^{2k}}
embed(a) = a    (same bit pattern, zero-extended into a_lo position)
```

concretely, embed(a) places a in the low half and zero in the high half. this means the element `1` has representation `0x01` at every level, and the element `0` has representation `0x00` at every level.

### projection (relative trace)

the relative trace from F_{2^{2k}} to F_{2^k} extracts the high component:

```
Tr_{2^{2k}/2^k}(a_lo + a_hi * x) = a_hi
```

this is because x + x^{2^k} = 1 in the extension (from x² + x + alpha), so Tr(a_lo + a_hi * x) = a_hi * Tr(x) = a_hi * 1 = a_hi.

### norm map

the relative norm from F_{2^{2k}} to F_{2^k}:

```
N(a) = a * a^{2^k}
N(a_lo + a_hi * x) = a_lo * (a_lo + a_hi) + a_hi² * alpha
```

this decomposes into subfield operations: 1 squaring, 1 multiplication, 2 additions, 1 multiplication by alpha.

## see also

- [field](field.md) -- all operations defined over the tower
- [inversion](inversion.md) -- tower-recursive inversion using the norm
