# hardware specification

**status: proposal** -- design considerations for a binary field processor. no silicon exists yet. this page defines target hardware primitives and SIMD utilization strategies for F₂ tower arithmetic.

## binary field processor primitives

the BFP (Binary Field Processor) -- four hardware primitives optimized for F₂ tower operations:

| primitive | operation | signature | purpose |
|-----------|-----------|-----------|---------|
| `bma` | binary multiply-accumulate | (a, b, acc) -> acc XOR (a AND b) | packed inner product |
| `tmul` | tower multiply | (a, b, level) -> a * b | tower Karatsuba at specified level |
| `popc` | popcount | (a) -> count_ones(a) | vector reduction |
| `clmul` | carry-less multiply | (a, b) -> a CLMUL b | polynomial multiplication in GF(2)[x] |

### bma -- binary multiply-accumulate

the fundamental kernel for BitNet inference and SpMV:

```
bma(a, b, acc):
  return acc XOR (a AND b)
```

fuses AND + XOR + accumulate into one operation. one `bma` per matrix row computes the partial inner product. chaining `bma` calls across 128-bit blocks accumulates a full row.

### tmul -- tower multiply

executes Karatsuba multiplication at a specified tower level. a single instruction replaces the recursive decomposition:

```
tmul(a, b, 7):    // F₂¹²⁸ multiplication
  // hardware recursively decomposes through all 7 levels
  // 3^7 = 2187 AND gates, fully pipelined
```

### popc -- popcount

hardware popcount with single-cycle throughput. required for the inner_product reduction step.

### clmul -- carry-less multiply

polynomial multiplication in GF(2)[x] without reduction. used as a building block for tower multiplication and for GCM/GHASH compatibility:

```
clmul(a, b):   // a, b are 64-bit polynomials
  return 128-bit product in GF(2)[x]
```

already exists in hardware: `PCLMULQDQ` (x86), `PMULL` (ARM).

## SIMD utilization

### ARM NEON (128-bit)

NEON provides native 128-bit registers (`uint8x16_t`, `uint64x2_t`):

| operation | NEON instruction | throughput |
|-----------|-----------------|------------|
| XOR (add) | `veorq_u8` | 1 cycle |
| AND (packed mul) | `vandq_u8` | 1 cycle |
| NOT | `vmvnq_u8` | 1 cycle |
| popcount | `vcntq_u8` + horizontal sum | ~4 cycles |
| carry-less mul | `vmull_p64` (PMULL) | 1 cycle |

one NEON register = one `Packed128` = one `F2_128`. native fit.

### x86 SSE/SSE2 (128-bit)

| operation | SSE instruction | throughput |
|-----------|----------------|------------|
| XOR (add) | `_mm_xor_si128` | 1 cycle |
| AND (packed mul) | `_mm_and_si128` | 1 cycle |
| NOT | `_mm_andnot_si128` with all-ones | 1 cycle |
| popcount | `_mm_popcnt_u64` x2 | 2 cycles |
| carry-less mul | `_mm_clmulepi64_si128` (PCLMULQDQ) | ~3 cycles |

### x86 AVX2 (256-bit)

process 2 `Packed128` values per instruction:

| operation | AVX2 instruction | Packed128s per op |
|-----------|-----------------|-------------------|
| XOR | `_mm256_xor_si256` | 2 |
| AND | `_mm256_and_si256` | 2 |
| popcount | no native; emulate via lookup | 2 (amortized) |

### x86 AVX-512 (512-bit)

process 4 `Packed128` values per instruction:

| operation | AVX-512 instruction | Packed128s per op |
|-----------|---------------------|-------------------|
| XOR | `_mm512_xor_si512` | 4 |
| AND | `_mm512_and_si512` | 4 |
| popcount | `_mm512_popcnt_epi64` (VPOPCNT) | 4 |
| ternary logic | `_mm512_ternarylogic_epi64` | 4 (arbitrary 3-input boolean) |

AVX-512 VPOPCNT (Ice Lake+) provides native 512-bit popcount. combined with ternary logic, it can compute `(a AND b) XOR c` in one instruction -- fusing the entire `bma` primitive.

## GPU compute shader design

the `kuro-wgsl` crate provides GPU-side tower arithmetic via wgpu compute shaders.

### workgroup layout

```
@workgroup_size(64, 1, 1)
fn tower_mul(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    let a = load_element(idx, input_a);
    let b = load_element(idx, input_b);
    output[idx] = tmul_128(a, b);
}
```

each thread computes one F₂¹²⁸ multiplication. 64 threads per workgroup. for N elements: dispatch ceil(N/64) workgroups.

### memory layout

tower elements stored as `vec4<u32>` (128 bits = 4 x 32-bit words) in storage buffers:

```
struct F2_128 {
    w: vec4<u32>,    // w.x = bits [0:32), w.y = [32:64), w.z = [64:96), w.w = [96:128)
}
```

addition in WGSL: component-wise XOR on `vec4<u32>`. WGSL lacks native u128 and u64, so all wide operations decompose into u32 arithmetic.

### packed operations on GPU

packed inner product for matrix-vector multiply:

```
fn packed_inner_product(a: vec4<u32>, b: vec4<u32>) -> u32 {
    let c = vec4<u32>(a.x & b.x, a.y & b.y, a.z & b.z, a.w & b.w);
    return countOneBits(c.x) + countOneBits(c.y) + countOneBits(c.z) + countOneBits(c.w);
}
```

`countOneBits` is a WGSL built-in. each call produces one row of the binary matrix-vector product.

### throughput model

| target | packed inner_products/sec (estimate) | notes |
|--------|-------------------------------------|-------|
| Apple M2 GPU (10 cores) | ~10 billion | metal backend via wgpu |
| NVIDIA RTX 4090 | ~50 billion | vulkan backend |
| AMD RX 7900 | ~30 billion | vulkan backend |
| CPU AVX-512 (single core) | ~2 billion | comparison baseline |

GPU throughput dominates for batch workloads (BitNet inference, SpMV). CPU wins for latency-sensitive single-element operations.

## see also

- [packed](packed.md) -- Packed128 operations specification
- [field](field.md) -- tower arithmetic that hardware accelerates
