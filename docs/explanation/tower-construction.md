# tower construction: doubling the field

kuro builds GF(2¹²⁸) not as a single monolithic extension, but as seven successive quadratic extensions — each level doubles the previous. this tower structure is the architectural decision that makes everything else work: Karatsuba multiplication, recursive inversion, and SIMD-native packed operations.

## the complex numbers analogy

the simplest tower construction in all of mathematics: the real numbers R are a field. adjoining i (a root of x² + 1) produces C = R[i], the complex numbers. every complex number is a + bi with a, b real. multiplication uses the rule i² = −1.

kuro does the same thing, but starting from GF(2) and repeating seven times.

## quadratic extensions over GF(2)

to extend a field F, pick an irreducible polynomial of degree 2 over F — a polynomial with no roots in F. adjoin a root of that polynomial. the result is a field with |F|² elements.

for binary fields, the irreducible polynomial is x² + x + α for a suitable α. why this form?

**why not x² + α?** in characteristic 2, x² + α = (x + √α)² whenever α has a square root. since every element of GF(2^n) has a square root (the Frobenius map x → x² is a bijection), x² + α always factors. it is never irreducible.

**why x² + x + α works.** the polynomial x² + x + α is irreducible over F when α cannot be written as t² + t for any t in F. at each tower level, the canonical generator α (the element 0b10) satisfies this condition.

## the canonical generator

at every tower level, α is the element whose binary representation is 0b10 — the second basis element. concretely:

```
level 1: F₂ → F₂²     irreducible: x² + x + 1        α = F2(1) = 0b1
level 2: F₂² → F₂⁴    irreducible: x² + x + α₁       α₁ = F2_2(0b10)
level 3: F₂⁴ → F₂⁸    irreducible: x² + x + α₂       α₂ = F2_4(0b0010)
level 4: F₂⁸ → F₂¹⁶   irreducible: x² + x + α₃       α₃ = F2_8(0x02)
level 5: F₂¹⁶ → F₂³²  irreducible: x² + x + α₄       α₄ = F2_16(0x0002)
level 6: F₂³² → F₂⁶⁴  irreducible: x² + x + α₅       α₅ = F2_32(0x0000_0002)
level 7: F₂⁶⁴ → F₂¹²⁸ irreducible: x² + x + α₆       α₆ = F2_64(0x02)
```

the first extension is special: over GF(2), the polynomial x² + x + 1 is the only irreducible quadratic (x² + x + 0 = x(x+1) factors). from level 2 onward, the pattern is uniform: α is always "the element 2" — the second bit set, all others zero.

this uniformity is not cosmetic. it means multiplication by α at any level is the same structural operation: shift the subfield element left by one position. the tower is self-similar.

## the tower, visually

```
level 0:  F₂           1 bit     {0, 1}
           │
           │  adjoin root of x² + x + 1
           ▼
level 1:  F₂²          2 bits    {00, 01, 10, 11}
           │
           │  adjoin root of x² + x + α₁    (α₁ = 10)
           ▼
level 2:  F₂⁴          4 bits    {0000, ..., 1111}
           │
           │  adjoin root of x² + x + α₂    (α₂ = 0010)
           ▼
level 3:  F₂⁸          8 bits    one byte — 256 elements
           │
           │  adjoin root of x² + x + α₃    (α₃ = 0000_0010)
           ▼
level 4:  F₂¹⁶         16 bits   65,536 elements
           │
           │  adjoin root of x² + x + α₄
           ▼
level 5:  F₂³²         32 bits   ~4 billion elements
           │
           │  adjoin root of x² + x + α₅
           ▼
level 6:  F₂⁶⁴         64 bits   one machine register
           │
           │  adjoin root of x² + x + α₆
           ▼
level 7:  F₂¹²⁸        128 bits  one u128 — top of the tower
```

each level contains all levels below it. an element of F₂⁸ is simultaneously a pair of F₂⁴ elements, a quad of F₂² elements, and an octet of F₂ elements. the tower is a matryoshka doll of nested fields.

## element representation

an element at level k has 2^k bits. it decomposes as a pair (lo, hi) where lo and hi are elements at level k−1:

```
element = lo + hi · x
```

the low half occupies the low bits, the high half occupies the high bits. for example, in F₂⁸ = F₂⁴[x]/(x² + x + α):

```
byte 0xAB = 0b1010_1011
  lo = 0b1011 = 0xB    (an F₂⁴ element)
  hi = 0b1010 = 0xA    (an F₂⁴ element)
  value = 0xB + 0xA·x  in F₂⁴[x]/(x² + x + α)
```

this decomposition is the key to tower arithmetic. addition at level k is XOR of the full word (because XOR distributes over the pair structure). multiplication at level k decomposes into three multiplications at level k−1 via Karatsuba. see [[karatsuba]].

## why the tower works

the tower succeeds because each level's structure is identical. the multiplication formula at every level (except the base) is:

```
(a_lo + a_hi·x)(b_lo + b_hi·x) = result_lo + result_hi·x

where:
  ll    = a_lo · b_lo                   (subfield multiplication)
  hh    = a_hi · b_hi                   (subfield multiplication)
  cross = a_lo · b_hi + a_hi · b_lo     (subfield multiplications + XOR)
  result_lo = ll + hh · α               (subfield multiplication by α + XOR)
  result_hi = cross + hh                (XOR)
```

this recursive decomposition bottoms out at F₂, where multiplication is AND. the entire tower is defined by this one pattern plus the base case.

## the reduction rule

the irreducible polynomial x² + x + α tells us: x² = x + α (in characteristic 2, subtraction equals addition). whenever a product creates an x² term, we substitute x + α. this is the reduction step that keeps results inside the field.

in code, the reduction appears as:
```rust
let c_lo = ll.add(hh.mul(alpha));   // ll + hh·α  (the x² → x + α substitution)
let c_hi = cross.add(hh);           // cross + hh  (hh contributes to both halves)
```

the hh term appears in both the low and high parts because x² = x + α means a_hi·b_hi·x² = a_hi·b_hi·x + a_hi·b_hi·α. the x coefficient goes to c_hi, the constant goes to c_lo.

## see also

- [[binary-fields]] — the algebraic foundations (GF(2), characteristic 2)
- [[karatsuba]] — the multiplication algorithm that exploits the tower
- [[inversion]] — recursive inversion through the tower
- [[packed-operations]] — at the top of the tower: 128 bits = 128 field elements
