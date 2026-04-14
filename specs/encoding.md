# encoding specification

how bytes map to tower field elements and back. this encoding is shared by every system that moves data across the byte/field boundary.

## bytes to tower elements

### F₂⁸ (byte-aligned)

each byte maps directly to one F₂⁸ element. the byte's bit layout matches the tower representation:

```
byte b = [b₇ b₆ b₅ b₄ b₃ b₂ b₁ b₀]

F₂⁸ element = b₀ + b₁·x₀ + b₂·x₁ + b₃·x₁x₀ + b₄·x₂ + b₅·x₂x₀ + b₆·x₂x₁ + b₇·x₂x₁x₀

tower decomposition:
  lo = F₂⁴(b & 0x0F)     // lower nibble
  hi = F₂⁴(b >> 4)       // upper nibble
  element = lo + hi * x₂   // in F₂⁴[x₂]/(x₂² + x₂ + alpha)
```

no conversion needed. bytes are F₂⁸ elements. the identity encoding.

### F₂¹⁶ (2 bytes)

two bytes in little-endian order map to one F₂¹⁶ element:

```
bytes [b₀, b₁]:
  element = F₂¹⁶(b₀ as u16 | (b₁ as u16) << 8)
  lo = F₂⁸(b₀)
  hi = F₂⁸(b₁)
```

### F₂¹²⁸ (16 bytes)

sixteen bytes in little-endian order map to one F₂¹²⁸ element:

```
bytes [b₀, b₁, ..., b₁₅]:
  element = F₂¹²⁸(b₀ | b₁<<8 | b₂<<16 | ... | b₁₅<<120)
```

equivalent to `u128::from_le_bytes([b₀, ..., b₁₅])`.

## tower elements to bytes

### canonical representation

every tower element has exactly one canonical byte representation: the little-endian encoding of its integer representation.

```
F₂⁸(v)   -> [v]                              1 byte
F₂¹⁶(v)  -> [v & 0xFF, v >> 8]              2 bytes
F₂³²(v)  -> v.to_le_bytes()                  4 bytes
F₂⁶⁴(v)  -> v.to_le_bytes()                  8 bytes
F₂¹²⁸(v) -> v.to_le_bytes()                 16 bytes
```

### sub-byte elements

F₂, F₂², and F₂⁴ are smaller than one byte. they are encoded in the low bits of a single byte with high bits zeroed:

```
F₂(v)   -> [v & 0x01]     1 byte, bits [1:8) = 0
F₂²(v)  -> [v & 0x03]     1 byte, bits [2:8) = 0
F₂⁴(v)  -> [v & 0x0F]     1 byte, bits [4:8) = 0
```

this matches the `repr(transparent)` struct layout in Rust: the element's u8 is the canonical byte.

## encoding table

| field | element bytes | total bytes | encoding |
|-------|--------------|-------------|----------|
| F₂ | 1 (padded) | 1 | bit 0 of byte |
| F₂² | 1 (padded) | 1 | bits 0-1 of byte |
| F₂⁴ | 1 (padded) | 1 | bits 0-3 of byte |
| F₂⁸ | 1 | 1 | identity |
| F₂¹⁶ | 2 | 2 | little-endian u16 |
| F₂³² | 4 | 4 | little-endian u32 |
| F₂⁶⁴ | 8 | 8 | little-endian u64 |
| F₂¹²⁸ | 16 | 16 | little-endian u128 |

## bulk encoding

for absorbing arbitrary byte streams into tower field elements:

```
encode_stream(bytes) -> Vec<F₂¹²⁸>:
  // pad to multiple of 16 bytes with zeros
  padded = bytes ++ [0; 16 - (len % 16)]
  for each 16-byte chunk:
    elements.push(F₂¹²⁸(u128::from_le_bytes(chunk)))
```

rate: 16 bytes per F₂¹²⁸ element. no conditional reduction needed (every 128-bit value is a valid field element). this is unlike prime field encoding where overflow checking is required.

## comparison with nebu encoding

| property | nebu (Goldilocks) | kuro (F₂ tower) |
|----------|-------------------|-----------------|
| input width | 7 bytes per element | 16 bytes per element |
| overflow check | not needed (7 < 8 byte prime) | not needed (all 2^128 values valid) |
| output width | 8 bytes per element | 16 bytes per element |
| asymmetry | yes (7 in, 8 out) | no (16 in, 16 out) |
| padding | zero-pad last chunk | zero-pad last chunk |

kuro has a simpler encoding story: every bit pattern is a valid element. no reduction, no conditional branches, no constant-time concerns from encoding.

## see also

- [field](field.md) -- field element definitions
- [vectors](vectors.md) -- encoding test vectors
