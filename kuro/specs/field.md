# field specification

the F₂ tower field. arithmetic substrate for binary proving, quantized inference, and every binary-domain computation in cyber. kuro is to F₂ what nebu is to Goldilocks.

version: 0.1
status: canonical

## the tower

seven successive quadratic extensions from F₂ to F₂¹²⁸:

```
level  field    bits  representation  irreducible polynomial
-----  -----    ----  --------------  ----------------------
0      F₂       1     u8 (bit 0)     --
1      F₂²      2     u8 (bits 0-1)  x² + x + 1
2      F₂⁴      4     u8 (bits 0-3)  x² + x + alpha_1
3      F₂⁸      8     u8             x² + x + alpha_2
4      F₂¹⁶     16    u16            x² + x + alpha_3
5      F₂³²     32    u32            x² + x + alpha_4
6      F₂⁶⁴     64    u64            x² + x + alpha_5
7      F₂¹²⁸    128   u128           x² + x + alpha_6
```

each alpha_k is the canonical generator of the previous level -- the element with representation `0b10` at that level (i.e., `x` in the polynomial ring).

## formal definitions

**Level 0.** F₂ = GF(2) = {0, 1}. characteristic 2. the base field.

**Level k+1.** F_{2^{2^{k+1}}} = F_{2^{2^k}}[x] / (x² + x + alpha_k). a degree-2 extension. the irreducible polynomial x² + x + alpha_k has no roots in the subfield because Tr(alpha_k) = 1, where Tr is the absolute trace to F₂.

**Top level.** F₂¹²⁸ = GF(2^128). this is a field with 2^128 elements. it is simultaneously the top of the tower and a 128-dimensional vector space over F₂.

## operations

all operations are defined over characteristic 2. there is no distinction between addition and subtraction.

### addition

```
add(a, b) = a XOR b
```

cost: 1 machine instruction at every level. addition is its own inverse: a + a = 0.

### multiplication

tower Karatsuba at every level above F₂:

```
mul(a, b) where a = a_lo + a_hi * x, b = b_lo + b_hi * x:
  ll = mul(a_lo, b_lo)
  hh = mul(a_hi, b_hi)
  cross = mul(a_lo + a_hi, b_lo + b_hi) + ll + hh

  result_lo = ll + hh * alpha
  result_hi = cross + hh
```

note: in characteristic 2, subtraction = addition = XOR. the reduction uses x² = x + alpha from the extension polynomial.

at F₂ (level 0): mul(a, b) = a AND b.

### squaring

squaring in characteristic 2 is linear (the Frobenius endomorphism):

```
sq(a) = a²
sq(a_lo + a_hi * x) = a_lo² + a_hi² * x²
                     = a_lo² + a_hi² * (x + alpha)
                     = (a_lo² + a_hi² * alpha) + a_hi² * x
```

cost: 1 recursive squaring per component (no cross term). squaring is cheaper than general multiplication.

### inversion

Fermat's little theorem: a^{-1} = a^{2^n - 2} in GF(2^n).

see [inversion](inversion.md) for the full algorithm and addition chains.

### frobenius endomorphism

the Frobenius map phi: a -> a² is a field automorphism of GF(2^n) over F₂. it generates the Galois group Gal(GF(2^n) / F₂) = Z/nZ.

```
phi(a) = a^2
phi^k(a) = a^{2^k}
```

at level k of the tower, the Frobenius restricted to F_{2^{2^k}} has order 2^k.

### trace

the absolute trace from GF(2^n) to F₂:

```
Tr(a) = a + a^2 + a^4 + ... + a^{2^{n-1}}
```

Tr(a) is always in F₂ = {0, 1}. for the tower, the trace decomposes:

```
Tr_{2^{2k}/2}(a) = Tr_{2^k/2}(Tr_{2^{2k}/2^k}(a))
Tr_{2^{2k}/2^k}(a_lo + a_hi * x) = a_hi        (relative trace)
```

### norm

the relative norm from F_{2^{2k}} to F_{2^k}:

```
N(a) = a * a^{2^k} = a * phi^k(a)
N(a_lo + a_hi * x) = a_lo² + a_lo * a_hi + a_hi² * alpha
                    = (a_lo + a_hi)^{-1 check} ...
```

the norm is multiplicative: N(ab) = N(a) * N(b).

### square root

in GF(2^n), every element has a unique square root (squaring is a bijection):

```
sqrt(a) = a^{2^{n-1}}
```

this is because (a^{2^{n-1}})^2 = a^{2^n} = a (by Fermat).

## properties and invariants

| property | value |
|----------|-------|
| characteristic | 2 |
| tower height | 7 levels (F₂ through F₂¹²⁸) |
| top field order | 2^128 |
| multiplicative group | cyclic of order 2^128 - 1 |
| every element self-inverse under addition | a + a = 0 for all a |
| squaring is linear | (a + b)² = a² + b² |
| Frobenius generates Galois group | phi has order n in GF(2^n) |
| unique square roots | every element has exactly one sqrt |

## cost model

| operation | F_p (Goldilocks, nebu) | F₂ tower (kuro) | ratio |
|-----------|------------------------|------------------|-------|
| add | 1 field add (~3 cycles) | 1 XOR (~1 cycle) | 3x faster |
| mul | 1 field mul (~5 cycles) | tower Karatsuba (~20 cycles) | 0.25x |
| sq | same as mul (~5 cycles) | linear, no cross term (~10 cycles) | 0.5x |
| inv | ~96 multiplications | ~2^k additions chains (see [inversion](inversion.md)) | varies |
| AND gate in circuit | ~32 constraints | 1 constraint | 32x faster |
| XOR gate in circuit | ~32 constraints | 1 constraint | 32x faster |
| 128 parallel AND | 128 * 32 = ~4,096 constraints | 1 AND instruction | 4,096x faster |

kuro wins for bitwise operations (32-4,096x). nebu wins for field multiplication (4x). the proof system chooses: Goldilocks for arithmetic, F₂ for binary. cross-algebra boundary: ~766 F_p constraints per crossing.

## packed operations

at the top of the tower, F₂¹²⁸ = 128 F₂ elements packed in one u128:

```
packed_add(a, b) = a XOR b         128 parallel additions, 1 instruction
packed_mul(a, b) = a AND b         128 parallel multiplications, 1 instruction
packed_not(a)    = NOT a            128 parallel complements, 1 instruction
popcount(a)      = count_ones(a)    sum of 128 elements, hardware instruction
inner_product(a, b) = popcount(a AND b)    binary inner product, 2 instructions
```

see [packed](packed.md) for the full Packed128 specification.

## see also

- [tower](tower.md) -- tower construction and irreducible polynomial selection
- [inversion](inversion.md) -- inversion algorithm and addition chains
- [vectors](vectors.md) -- known-answer test vectors
- [encoding](encoding.md) -- byte encoding
