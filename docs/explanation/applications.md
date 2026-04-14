# applications: why binary fields matter

binary fields are not an academic curiosity. they sit at the intersection of three accelerating trends: quantized AI inference, efficient proof systems, and the bitwise operations that dominate real-world computation. kuro exists because these workloads are 32-4096× cheaper in GF(2) than in a prime field.

## BitNet and 1-bit quantized AI

BitNet (Microsoft Research, 2023) demonstrated that large language models can be quantized to 1-bit weights ({-1, +1}, encoded as {0, 1}) with minimal accuracy loss. the core operation in transformer inference is matrix-vector multiplication. with 1-bit weights:

```
output[i] = Σⱼ weight[i][j] · activation[j]
```

when weights are bits and activations are bits, this is a binary inner product: AND the row with the activation vector, then popcount.

```
output[i] = popcount(row[i] AND activation)
```

one row of a 128-wide matrix takes two instructions. a full 4096×4096 matrix, processed in 128-bit chunks, takes 4096 × 32 = 131,072 AND+popcount pairs. on a 3 GHz processor executing one pair per cycle, this completes in ~44 microseconds. the same matrix in float32 takes ~67 million multiply-adds.

the field-theoretic perspective: BitNet inference is linear algebra over GF(2). proving correctness of BitNet inference (inside a STARK) requires field arithmetic over GF(2). kuro provides that arithmetic.

## binary proving (STARK over F₂)

traditional STARKs operate over a prime field. every constraint is a polynomial equation over F_p. bitwise operations (XOR, AND, bit shifts) are expensive in F_p because they require bit decomposition: extracting 64 individual bits from a field element costs ~32 constraints.

a binary STARK operates over F₂ and its extensions. in this setting:

```
XOR constraint:  c = a + b           1 constraint (XOR is addition)
AND constraint:  c = a · b           1 constraint (AND is multiplication)
bit shift:       native              0 constraints (already bits)
```

compare with the prime field cost:
```
XOR constraint:  ~32 constraints     (bit decompose, XOR, recompose)
AND constraint:  ~32 constraints     (bit decompose, AND, recompose)
```

the 32× advantage is structural — it comes from the algebra, not the implementation. for programs dominated by bitwise operations (hash functions, bitwise permutations, boolean circuits), binary proving is categorically faster.

the Binius proof system (Ulvetanna, 2024) implements this approach using the same tower field construction that kuro provides. kuro's F₂ through F₂¹²⁸ tower is the arithmetic substrate for binary STARKs.

## coding theory and error correction

binary fields are the original home of error-correcting codes:

**Hamming codes.** the first error-correcting code (1950). parity check matrices operate over GF(2). syndrome computation is matrix-vector multiplication over GF(2) — exactly the inner product kernel kuro implements.

**BCH and Reed-Solomon over GF(2^m).** BCH codes are cyclic codes defined by roots in GF(2^m). Reed-Solomon codes (used in QR codes, CDs, deep-space communication) are evaluation codes over extension fields. the error correction algorithm requires arithmetic in GF(2⁸) or larger binary extensions.

**LDPC codes.** the codes used in 5G, WiFi 6, and satellite communication. their decoding algorithms perform iterative message-passing over GF(2) — billions of XOR operations per decoded block.

the connection to kuro: proving correctness of error correction (for verifiable communication protocols) requires binary field arithmetic inside the proof system.

## AES and symmetric cryptography

the Advanced Encryption Standard operates in GF(2⁸) = GF(256), defined by the irreducible polynomial x⁸ + x⁴ + x³ + x + 1.

the AES round function:
1. **SubBytes**: S-box lookup, equivalent to inversion in GF(2⁸) followed by an affine map
2. **ShiftRows**: byte permutation (free in binary representation)
3. **MixColumns**: matrix multiplication over GF(2⁸)
4. **AddRoundKey**: XOR with round key (addition in GF(2⁸))

every AES operation is binary field arithmetic. proving AES execution inside a binary STARK is natural — the constraints match the algebra directly. an AES round in a binary STARK costs roughly 200 constraints. the same round in a prime-field STARK costs roughly 6,000 constraints due to the bit-decomposition overhead.

## the tri-kernel SpMV

the cyber network's consensus mechanism uses sparse matrix-vector multiplication (SpMV) with quantized weights. in the tri-kernel variant, axon weights are ternary ({-1, 0, +1}) or binary ({0, 1}).

the SpMV kernel:
```
for each row i:
  result[i] = Σⱼ weight[i][j] · input[j]
```

with binary weights, this reduces to:
```
for each row i:
  result[i] = popcount(weight_row[i] AND input)
```

the same inner product kernel as BitNet. the same packed operations. the same binary field arithmetic.

proving the SpMV computation (that the consensus iteration was executed correctly) requires binary STARK constraints. the binary field makes both the computation and its proof efficient.

## the quantization convergence

three independent trends point to binary fields:

| trend | driver | binary field role |
|-------|--------|------------------|
| AI quantization | energy efficiency, inference speed | 1-bit weights = GF(2) elements |
| proof compression | verifiable computation at scale | bitwise operations at native cost |
| network consensus | decentralized verification | quantized SpMV provably correct |

kuro sits at the convergence point. it provides the field arithmetic that all three workloads need — and all three workloads are growing.

## see also

- [[packed-operations]] — the inner product kernel that drives BitNet and SpMV
- [[f2-vs-fp]] — when binary fields win vs when prime fields win
- [[binary-fields]] — the algebraic foundation
- [[tower-construction]] — the tower that enables binary STARK arithmetic
