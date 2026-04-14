# Packed128 operations specification

128 parallel F₂ operations in one u128 machine word. this is the computational kernel for binary workloads.

## representation

`Packed128` wraps a single `u128`. each bit position holds one F₂ element:

```
bit 127                                     bit 0
  |                                            |
  v                                            v
  [e_127, e_126, ..., e_2, e_1, e_0]

  e_i in F₂ = {0, 1}
```

`Packed128(0)` = the zero vector. `Packed128(u128::MAX)` = the all-ones vector.

## operations

### vectorized addition

```
add(a, b) = Packed128(a.0 XOR b.0)
```

128 F₂ additions in 1 instruction. addition is self-inverse: add(a, a) = ZERO.

### vectorized multiplication

```
mul(a, b) = Packed128(a.0 AND b.0)
```

128 F₂ multiplications in 1 instruction.

### vectorized complement

```
not(a) = Packed128(NOT a.0)
```

128 parallel complements. not(a) = add(a, ONES).

### popcount

```
popcount(a) = count_ones(a.0)
```

returns the number of 1-bits = the sum of all 128 F₂ elements. reduces the vector to a scalar in F₂ (mod 2) or to an integer in [0, 128].

hardware support: `popcnt` (x86), `cnt` (ARM NEON), `ctpop` (LLVM intrinsic).

### inner product

```
inner_product(a, b) = popcount(a AND b) = count_ones(a.0 & b.0)
```

the binary inner product of two 128-element vectors over F₂. 2 machine instructions (AND + popcount).

**this is the fundamental kernel.** one inner_product call computes one row of a binary matrix-vector multiplication.

## matrix-vector multiply

a binary matrix M (m rows x 128 columns) stored as m `Packed128` values. a vector v stored as one `Packed128`:

```
mat_vec_mul(M, v):
  for i in 0..m:
    result[i] = inner_product(M[i], v)    // 1 bit of output per row
```

cost: m AND instructions + m popcount instructions. for a 128x128 matrix: 128 inner_product calls = 256 instructions.

in a prime field, the same 128x128 binary matrix multiply would cost 128 * 128 = 16,384 multiplications + 128 * 127 = 16,256 additions. the packed binary approach is ~128x faster.

## applications

### BitNet inference

BitNet 1-bit quantized neural networks store weights as binary matrices. inference = matrix-vector multiply over F₂:

```
layer(input):
  // weights: M rows of Packed128
  // input:   Packed128 (128 binary activations)
  for each row:
    activation[row] = popcount(M[row] AND input)
  // threshold to produce binary output
  output[row] = (activation[row] > threshold) ? 1 : 0
```

one layer of a 128-wide BitNet model: 128 inner_product calls. the entire forward pass of a multi-layer network is a chain of mat-vec multiplies -- all using Packed128.

### SpMV (sparse matrix-vector multiply)

tri-kernel SpMV for quantized axon weights in pi iteration. sparse binary matrices stored in compressed row format where each nonzero row is a Packed128 with the nonzero pattern:

```
spmv(rows, col_packs, values, x):
  for each row i:
    acc = 0
    for each nonzero block j in row i:
      acc ^= inner_product(col_packs[j], x[block_col[j]])
    result[i] = acc & 1    // reduce mod 2
```

### relationship to tower arithmetic

`Packed128` and `F2_128` share the same underlying `u128`, but they interpret it differently:

| type | interpretation | add | mul |
|------|---------------|-----|-----|
| `Packed128` | 128 independent F₂ elements | XOR (parallel add) | AND (parallel mul) |
| `F2_128` | 1 element of GF(2^128) | XOR (field add) | tower Karatsuba (field mul) |

addition is identical (XOR). multiplication differs: Packed128 uses AND (component-wise), F2_128 uses Karatsuba (field multiplication). conversion between interpretations is free (reinterpret the bits).

## bit access

```
get_bit(a, i) = (a.0 >> i) & 1           // extract element i
set_bit(a, i) = Packed128(a.0 | (1 << i)) // set element i to 1
```

## shift operations

```
shl(a, n) = Packed128(a.0 << n)     // shift all elements left by n positions
shr(a, n) = Packed128(a.0 >> n)     // shift all elements right by n positions
```

useful for alignment and sliding-window operations.

## see also

- [field](field.md) -- tower field operations (the other interpretation of u128)
- [hardware](hardware.md) -- SIMD implementation of Packed128 operations
