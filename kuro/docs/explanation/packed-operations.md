# Packed128: 128 operations in one instruction

a u128 register holds 128 bits. each bit is an element of GF(2). a single XOR instruction adds 128 field elements in parallel. a single AND instruction multiplies 128 field elements in parallel. this is the computational payoff of binary fields: the hardware already implements vectorized field arithmetic.

## the SIMD idea

SIMD (Single Instruction, Multiple Data) processors execute one operation across multiple data lanes. AVX-512 adds 16 floats in one instruction. binary fields take this further: the "SIMD width" equals the register width in bits. a 128-bit register is a 128-lane binary SIMD unit by construction.

no special hardware. no intrinsics. no alignment requirements. every CPU with XOR and AND instructions is a binary field SIMD machine.

## Packed128 as a vector

kuro's `Packed128` type wraps a u128. it represents a vector of 128 GF(2) elements:

```
Packed128(0b1010...0110)
         ────────────────
         128 field elements, indexed by bit position
```

bit 0 is element 0, bit 1 is element 1, and so on. the vector has length 128.

## vectorized arithmetic

**addition (XOR).** add two vectors element-wise:
```rust
Packed128(a).add(Packed128(b)) = Packed128(a ^ b)
```
one instruction. 128 additions. each bit position is independent.

**multiplication (AND).** multiply two vectors element-wise:
```rust
Packed128(a).mul(Packed128(b)) = Packed128(a & b)
```
one instruction. 128 multiplications. each bit position is independent.

**complement (NOT).** negate every element (in GF(2), negation = identity + 1):
```rust
Packed128(a).not() = Packed128(!a)
```
one instruction. 128 negations.

## the inner product kernel

the inner product of two binary vectors is the sum of their element-wise products:

```
<a, b> = a₀·b₀ + a₁·b₁ + ... + a₁₂₇·b₁₂₇
```

in GF(2): multiplication is AND, addition is XOR. but the final sum (over all 128 terms) is not XOR — it is a count. how many of the products are 1? if the count is odd, the inner product is 1; if even, it is 0.

this is popcount:

```rust
Packed128(a).inner_product(Packed128(b)) = (a & b).count_ones()
```

two instructions: AND then popcount. this computes the binary inner product of a 128-element vector.

## why inner product matters for AI

a matrix-vector multiply is a collection of inner products — one per row of the matrix. in a binary neural network (BitNet), both the weights and activations are single bits. one matrix row is a 128-bit vector. one activation vector is a 128-bit vector. one row's output is `popcount(row AND activation)`.

for a matrix with m rows and n = 128 columns:
```
for each row i:
  output[i] = popcount(row[i] AND activation)
```

m iterations, each costing two instructions. a 1024×128 matrix-vector multiply takes 1024 AND + 1024 popcount = 2048 instructions. compare with a conventional float32 matrix-vector multiply: 1024 × 128 = 131,072 multiply-adds. the binary version is ~64× fewer instructions, operating on ~32× less data.

this is why quantized AI models care about binary fields.

## popcount and hardware support

popcount (population count, Hamming weight) counts the number of set bits in a word. it is the GF(2) analogue of summation.

modern CPUs provide hardware popcount:
- x86-64: `popcnt` instruction (SSE4.2 and later)
- AArch64: `cnt` instruction (operating on NEON vectors)
- RISC-V: `cpop` instruction (Zbb extension)
- WebGPU: `countOneBits()` built-in

without hardware support, popcount can be computed in O(log n) operations using bit tricks. with hardware support, it is a single-cycle instruction.

kuro's `Packed128::popcount` compiles to the hardware instruction on supporting platforms:
```rust
pub fn popcount(self) -> u32 {
    self.0.count_ones()
}
```

## the dual interpretation

the same 128-bit value can be interpreted two ways:

1. **packed vector**: 128 independent GF(2) elements (the `Packed128` view)
2. **tower element**: one element of GF(2¹²⁸) (the `F2_128` view)

the `as_tower` method reinterprets:
```rust
let packed = Packed128(bits);
let tower = packed.as_tower();  // same bits, different algebraic structure
```

as a packed vector, XOR is 128 parallel additions and AND is 128 parallel multiplications. as a tower element, XOR is one addition in GF(2¹²⁸) — but multiplication requires the full Karatsuba tower decomposition.

this duality is central to kuro's design. the proof system can choose the interpretation that matches the workload: packed for bitwise parallelism, tower for field-level algebraic operations.

## see also

- [[binary-fields]] — why XOR is addition and AND is multiplication in GF(2)
- [[karatsuba]] — tower multiplication when bits are interpreted as one field element
- [[applications]] — BitNet inference, the tri-kernel SpMV, and other uses of packed operations
