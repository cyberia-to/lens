# test vectors

known-answer tests for F₂ tower field arithmetic. any conforming implementation must produce identical results.

## F₂ (level 0)

| a | b | add | mul |
|---|---|-----|-----|
| 0 | 0 | 0 | 0 |
| 0 | 1 | 1 | 0 |
| 1 | 0 | 1 | 0 |
| 1 | 1 | 0 | 1 |

complete truth table. add = XOR, mul = AND.

## F₂² (level 1)

elements: {0, 1, x, x+1} = {0b00, 0b01, 0b10, 0b11}. irreducible: x² + x + 1.

### multiplication table

| * | 0 | 1 | x | x+1 |
|---|---|---|---|-----|
| **0** | 0 | 0 | 0 | 0 |
| **1** | 0 | 1 | x | x+1 |
| **x** | 0 | x | x+1 | 1 |
| **x+1** | 0 | x+1 | 1 | x |

verification: x * (x+1) = x² + x = 1 (from x² + x + 1 = 0, so x² + x = 1).

### inversion

| a (bin) | a^{-1} (bin) |
|---------|-------------|
| 0b01 (= 1) | 0b01 (= 1) |
| 0b10 (= x) | 0b11 (= x+1) |
| 0b11 (= x+1) | 0b10 (= x) |

verification: x * (x+1) = 1.

## F₂⁴ (level 2)

irreducible: x² + x + alpha, where alpha = 0b10 (= x in F₂²).

### selected products

| a (hex) | b (hex) | a * b (hex) |
|---------|---------|-------------|
| 0x1 | 0x1 | 0x1 |
| 0x1 | 0xF | 0xF |
| 0x2 | 0x2 | 0x7 |
| 0x3 | 0x5 | 0xF |

row 3: alpha² = alpha + alpha_sub (tower Karatsuba result). row 4: (x+1)(x²+1) in the tower.

### identity

for all a in F₂⁴: `a * 1 = a` and `a + a = 0`.

## F₂⁸ (level 3)

### selected products

| a (hex) | b (hex) | a * b (hex) |
|---------|---------|-------------|
| 0x01 | 0x01 | 0x01 |
| 0x01 | 0xAB | 0xAB |
| 0xAB | 0x01 | 0xAB |
| 0xFF | 0xFF | (verify via implementation) |

### self-inverse under addition

| a (hex) | a + a |
|---------|-------|
| 0x00 | 0x00 |
| 0xAB | 0x00 |
| 0xFF | 0x00 |

## F₂¹⁶ (level 4)

### identity preservation

| a (hex) | a * 1 |
|---------|-------|
| 0x0000 | 0x0000 |
| 0xABCD | 0xABCD |
| 0xFFFF | 0xFFFF |

### addition (= XOR)

| a (hex) | b (hex) | a + b (hex) |
|---------|---------|-------------|
| 0xFF00 | 0x00FF | 0xFFFF |
| 0xAAAA | 0x5555 | 0xFFFF |
| 0x1234 | 0x1234 | 0x0000 |

## F₂³² (level 5)

### identity preservation

| a (hex) | a * 1 |
|---------|-------|
| 0xDEADBEEF | 0xDEADBEEF |

### addition

| a (hex) | b (hex) | a + b (hex) |
|---------|---------|-------------|
| 0xFFFFFFFF | 0x00000001 | 0xFFFFFFFE |
| 0xDEADBEEF | 0xDEADBEEF | 0x00000000 |

## F₂¹²⁸ (level 7)

### addition

| a (hex) | b (hex) | a + b (hex) |
|---------|---------|-------------|
| 0xFF00FF00...FF00FF00 | 0x00FF00FF...00FF00FF | 0xFFFFFFFF...FFFFFFFF |
| any a | a | 0x0000...0000 |

### identity

| a | a * 1 |
|---|-------|
| 0xFF00FF00FF00FF00FF00FF00FF00FF00 | 0xFF00FF00FF00FF00FF00FF00FF00FF00 |

## edge cases

### zero behavior

at every level:

| operation | result |
|-----------|--------|
| 0 + 0 | 0 |
| 0 + a | a |
| 0 * a | 0 |
| a + a | 0 |
| a * 1 | a |
| 1 * 1 | 1 |

### multiplicative identity

at every level, the element with representation 0x01 is the multiplicative identity: a * ONE = a for all a.

### additive self-inverse

at every level, every element is its own additive inverse: a + a = 0. this is characteristic 2.

## Packed128

### inner product

| a (hex) | b (hex) | inner_product |
|---------|---------|--------------|
| 0xFFFF...FFFF (128 ones) | 0xFFFF...FFFF | 128 |
| 0x0000...0000 | 0xFFFF...FFFF | 0 |
| 0xAAAA...AAAA (64 ones) | 0xFFFF...FFFF | 64 |
| 0b11110000 | 0b10101010 | 2 |

### popcount

| a | popcount |
|---|----------|
| 0 | 0 |
| 1 | 1 |
| u128::MAX | 128 |
| 0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA | 64 |

## see also

- [field](field.md) -- operation definitions
- [encoding](encoding.md) -- byte encoding of test values
