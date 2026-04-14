---
tags: jali, crypto
crystal-type: entity
crystal-domain: crypto
---
# encoding — RingElement serialization

how ring elements are serialized to bytes and deserialized back. two forms exist: coefficient encoding and NTT encoding.

## coefficient form encoding

a RingElement in coefficient form is n Goldilocks field elements. each coefficient is 8 bytes little-endian:

```
encode_coeff(a: RingElement) → [u8; 8n]:
  for i in 0..n:
    bytes[8*i .. 8*(i+1)] = a.coeffs[i].to_le_bytes()

decode_coeff(bytes: [u8; 8n]) → RingElement:
  for i in 0..n:
    coeffs[i] = F_p::from_le_bytes(bytes[8*i .. 8*(i+1)])
```

total size: 8n bytes. at n = 1024: 8 KiB. at n = 4096: 32 KiB.

## NTT form encoding

a RingElement in NTT form is also n Goldilocks field elements — the evaluations at the 2n-th roots of unity. the encoding is identical in structure:

```
encode_ntt(a: RingElement) → [u8; 8n]:
  for i in 0..n:
    bytes[8*i .. 8*(i+1)] = a.ntt_coeffs[i].to_le_bytes()
```

the distinction is in the header/tag: coefficient form and NTT form produce the same byte layout but represent different mathematical objects. the serialization format must include a tag or the context must be unambiguous.

## format tag

```
byte 0:  0x00 = coefficient form
         0x01 = NTT form
bytes 1-2: n as u16 little-endian (ring degree)
bytes 3-4: reserved (zero)
bytes 5..: 8n coefficient bytes
```

total wire size: 5 + 8n bytes.

## compressed encoding

for ring elements with small coefficients (ternary secrets, CBD noise), a compressed encoding reduces size:

```
ternary: 2 bits per coefficient → 2n bits = n/4 bytes
  encoding: {-1 → 0b10, 0 → 0b00, 1 → 0b01}

CBD(η): ceil(log₂(2η+1)) bits per coefficient
  η = 2: 3 bits per coefficient → 3n/8 bytes
```

compressed encoding is used for key transmission. the full 8-byte encoding is used for computation and proofs.

## coefficient ordering

coefficients are stored in natural order: a[0], a[1], ..., a[n-1] where the polynomial is a[0] + a[1]*x + a[2]*x² + ... + a[n-1]*x^{n-1}.

NTT coefficients are stored in bit-reversed order (matching the NTT output). no additional permutation is needed for the in-place NTT algorithm.

## comparison with nebu encoding

| property | nebu (single F_p) | jali (R_q) |
|----------|-------------------|------------|
| element size | 8 bytes | 8n bytes (8-32 KiB) |
| overflow check | needed (value < p) | needed per coefficient |
| endianness | little-endian | little-endian |
| compression | none | possible for small coefficients |

## see also

- [ring](ring.md) — the RingElement type
- [ntt](ntt.md) — NTT form definition
- [sample](sample.md) — compressed encoding of sampled elements
