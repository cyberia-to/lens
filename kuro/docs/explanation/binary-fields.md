# binary fields: the algebra of bits

a binary field is a finite field built from the simplest possible foundation: a set with two elements. every digital computer already speaks this language — bits are zeros and ones. binary fields give bits algebraic structure: addition, multiplication, and division that stay within the set.

## GF(2): one bit, two elements

the smallest finite field is GF(2) = {0, 1}. its arithmetic is:

| + | 0 | 1 |     | × | 0 | 1 |
|---|---|---|     |---|---|---|
| 0 | 0 | 1 |     | 0 | 0 | 0 |
| 1 | 1 | 0 |     | 1 | 0 | 1 |

addition is XOR. multiplication is AND. these are not analogies — they are exact identities. the field axioms (closure, associativity, commutativity, distributivity, inverses) are satisfied by the truth tables of XOR and AND.

the additive identity is 0. the multiplicative identity is 1. every nonzero element (just 1) has a multiplicative inverse (1⁻¹ = 1). and crucially: 1 + 1 = 0. every element is its own additive inverse.

in kuro:
```rust
F2(1).add(F2(1)) == F2(0)   // 1 XOR 1 = 0
F2(1).mul(F2(1)) == F2(1)   // 1 AND 1 = 1
```

## characteristic 2: why it changes everything

the characteristic of a field is the number of times you must add 1 to itself to get 0. in GF(2): 1 + 1 = 0, so the characteristic is 2. compare with the Goldilocks field (characteristic p ≈ 2⁶⁴): you must add 1 to itself 2⁶⁴ − 2³² + 1 times before reaching 0.

characteristic 2 has profound consequences:

**addition equals subtraction.** since a + a = 0 for every element, we have a = −a. there is no distinction between adding and subtracting. this eliminates an entire class of edge cases in algorithms.

**squaring is linear.** in any field of characteristic 2, (a + b)² = a² + 2ab + b² = a² + b² because 2ab = 0. the Frobenius map x → x² is a field homomorphism — it distributes over addition. this makes squaring dramatically cheaper than general multiplication.

**no carries.** addition in GF(2) never carries — XOR is a bitwise operation with no interaction between positions. this is why 128 GF(2) additions can happen in a single XOR instruction.

## GF(2^n): vector spaces of bits

GF(2) is one bit. to build larger fields, we extend: GF(2²) has 4 elements, GF(2⁴) has 16, GF(2⁸) has 256, and so on up to GF(2¹²⁸) with 2¹²⁸ elements.

each GF(2^n) is an n-dimensional vector space over GF(2). its elements are n-bit vectors. addition is bitwise XOR (vector addition over GF(2)). multiplication is more complex — it depends on the irreducible polynomial used to define the extension.

think of it this way: an element of GF(2⁸) is a byte. there are 256 possible bytes. addition of two bytes is XOR. multiplication of two bytes follows polynomial arithmetic modulo an irreducible polynomial of degree 8. the result is always another byte — closure is guaranteed by the reduction.

this is not abstract: AES encryption multiplies bytes in GF(2⁸) defined by the polynomial x⁸ + x⁴ + x³ + x + 1. every AES encryption you have ever used performs binary field arithmetic.

## the tower approach

kuro does not use a single irreducible polynomial of high degree. instead, it builds GF(2¹²⁸) as a tower of quadratic extensions:

```
GF(2)   →  GF(2²)  →  GF(2⁴)  →  GF(2⁸)  →  GF(2¹⁶) → GF(2³²) → GF(2⁶⁴) → GF(2¹²⁸)
 1 bit      2 bits     4 bits     8 bits     16 bits    32 bits    64 bits    128 bits
```

each step doubles the field by adjoining a root of x² + x + α, where α is a specific element of the current field. this recursive doubling enables Karatsuba multiplication: at each level, one multiplication decomposes into three multiplications at the level below. see [[tower-construction]] and [[karatsuba]].

## historical context

binary fields emerge independently in multiple branches of mathematics and engineering.

**Galois (1830s).** Evariste Galois proved that finite fields exist only for prime powers. GF(2^n) is the case p = 2 — the smallest prime. Galois died at 20, leaving behind the theory that underlies all of modern algebra.

**Shannon and coding theory (1948).** Claude Shannon's information theory led to error-correcting codes over GF(2). Hamming codes, Reed-Muller codes, and BCH codes all perform arithmetic in binary fields. the parity check — XOR of all bits — is the simplest GF(2) computation.

**Rijndael / AES (1998).** the Advanced Encryption Standard performs its core operations in GF(2⁸). the S-box, MixColumns, and key schedule all use binary field arithmetic. every TLS connection, every encrypted disk, every secure message uses GF(2⁸).

**Binius and binary STARKs (2023-present).** binary fields are entering the proof system world. Binius uses tower fields (the same structure kuro implements) to build STARKs where bitwise operations cost one constraint instead of thirty-two. this is the application that motivates kuro.

## see also

- [[tower-construction]] — how the tower is built, step by step
- [[packed-operations]] — 128 GF(2) elements in one machine word
- [[f2-vs-fp]] — binary fields vs prime fields, when to use which
