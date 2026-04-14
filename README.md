---
tags: cyber, cip
crystal-type: entity
crystal-domain: cyber
alias: kuro, F2 tower, binary field arithmetic
---
# kuro (黒)

F₂ tower field arithmetic for binary proving. kuro is to F₂ what [[nebu]] is to [[Goldilocks field|Goldilocks]].

## the tower

```
F₂ → F₂² → F₂⁴ → F₂⁸ → F₂¹⁶ → F₂³² → F₂⁶⁴ → F₂¹²⁸
 1b    2b    4b    8b    16b     32b     64b     128b
```

each extension: $F_{2^{2k}} = F_{2^k}[x] / (x^2 + x + \alpha_k)$

Wiedemann tower construction: $\alpha_k$ = product of all previous generators. at the top: F₂¹²⁸ = 128 F₂ elements packed in one u128 machine word.

## operations

at every tower level:

| operation | description | complexity |
|-----------|-------------|------------|
| add | XOR | 1 instruction |
| mul | tower Karatsuba | 3 recursive muls at half-width |
| inv | tower-recursive norm-based | 1 sub-field inv + few muls |
| square | Frobenius endomorphism | linear in char 2 |
| sqrt | inverse Frobenius | n-1 squarings |
| frobenius | a^(2^k) | k squarings |
| trace | Tr(a) ∈ F₂ | tower-recursive |
| norm | N(a) to sub-field | tower-recursive |
| exp | square-and-multiply | O(log e) |

## packed operations

128 parallel F₂ operations in one u128:

```
packed_add(a, b) = XOR      128 additions, 1 instruction
packed_mul(a, b) = AND      128 multiplications, 1 instruction
inner_product(a, b)          popcount(AND), 2 instructions — the BitNet kernel
```

## why kuro exists

bitwise operations in [[Goldilocks field|Goldilocks]] (F_p) cost ~32 constraints each — bit decomposition is expensive in a prime field. in F₂, they cost 1 constraint. the 32× gap is the algebraic distance between F_p and F₂.

two workloads dominate the binary regime:
- quantized AI inference: BitNet 1-bit models. matrix-vector multiply = XOR + popcount
- tri-kernel SpMV: quantized axon weights for π iteration

## structure

```
kuro/
├── rs/              core library (77 tests, zero deps, no_std)
│   ├── tower.rs     8 tower levels with full arithmetic
│   ├── packed.rs    Packed128 SIMD operations
│   ├── inv.rs       checked inversion
│   ├── batch.rs     Montgomery batch inversion
│   └── encoding.rs  bytes ↔ tower elements
├── wgsl/            GPU compute shaders
├── cli/             command-line tool
├── reference/       canonical specifications (9 docs)
└── docs/explanation/ educational articles (8 docs)
```

## usage

```
cargo test -p kuro
cargo bench -p kuro
cargo run -p kuro-cli -- help
cargo run -p kuro-cli -- calc mul 0xDEADBEEF 0xCAFEBABE --level 32
```

## companion repos

| repo | role |
|------|------|
| [[nebu]] | Goldilocks field arithmetic (the prime field) |
| [[hemera]] | hash function (trust anchor) |
| [[nox]] | VM (Bt = nox<F₂, Z/2, external>) |
| [[zheng]] | proof system (binary PCS uses kuro for F₂ ops) |

## license

cyber license: don't trust. don't fear. don't beg.
