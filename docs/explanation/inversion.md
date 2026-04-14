# inversion in binary fields

inversion computes a⁻¹ such that a · a⁻¹ = 1. it is the most expensive single operation in any finite field — roughly 64× the cost of multiplication. understanding how inversion works in the binary tower reveals deep structure: Fermat's theorem, the Frobenius endomorphism, and a recursive algorithm that mirrors the tower itself.

## why inversion matters

inversion enables division: a / b = a · b⁻¹. division appears in:

- **polynomial evaluation**: evaluating rational functions in the proof system
- **batch operations**: Montgomery's trick inverts n elements with 1 inversion + 3(n-1) multiplications
- **normalization**: converting projective coordinates to affine
- **lookup arguments**: the logarithmic derivative technique computes 1/(alpha - v) for each lookup

inversion is rare compared to multiplication — good algorithms minimize the number of inversions by batching. but when you need it, you need it.

## Fermat's little theorem in GF(2^n)

every finite field satisfies Fermat's little theorem. for GF(2^n) with q = 2^n elements:

```
a^(q-1) = 1    for all a ≠ 0
```

therefore:

```
a⁻¹ = a^(q-2)
```

for F₂⁸ (q = 256): a⁻¹ = a^254. for F₂¹²⁸ (q = 2¹²⁸): a⁻¹ = a^(2¹²⁸ - 2). this is a huge exponent, but the special structure of 2^n - 2 = 2(2^{n-1} - 1) makes computation tractable via repeated squaring.

the binary expansion of q - 2 consists of n - 1 ones followed by a zero: 0b111...110. this regularity enables an optimized addition chain.

## the Frobenius endomorphism: squaring is linear

in any field of characteristic 2, the map x → x² is a field homomorphism. this means:

```
(a + b)² = a² + b²
```

squaring distributes over addition. this is because 2ab = 0 in characteristic 2 — the cross term vanishes.

what does squaring look like in the tower? consider an element a = a_lo + a_hi · x in F_{2^{2k}} = F_{2^k}[x]/(x² + x + α). then:

```
a² = a_lo² + a_hi² · x²
   = a_lo² + a_hi² · (x + α)
   = (a_lo² + a_hi² · α) + a_hi² · x
```

the crucial point: a_lo² and a_hi² are squarings in the subfield. and squaring in GF(2^n) is a linear operation — it is a permutation that can be computed without any multiplications at the base level. this means squaring through the entire tower is dramatically cheaper than general multiplication.

**squaring cost vs multiplication cost:**
```
multiplication in F₂¹²⁸:  ~2,187 AND operations (Karatsuba)
squaring in F₂¹²⁸:        ~128 AND operations (linearity)
```

squaring is ~17× cheaper than multiplication. this makes Frobenius-based algorithms highly efficient.

## tower-recursive inversion

the tower structure enables a recursive inversion formula. for a = a_lo + a_hi · x in F_{2^{2k}}:

the norm of a down to the subfield is:
```
N(a) = a · a^(2^k) = a_lo² + a_lo · a_hi + a_hi² · α
```

this norm N(a) is an element of the subfield F_{2^k}. the inverse is:

```
a⁻¹ = a^(2^k) / N(a)
```

where a^(2^k) is the Frobenius conjugate (cheap to compute — just squaring k times, which is linear) and N(a)⁻¹ is an inversion in the subfield (half the size).

the recursion:
1. compute the conjugate a^(2^k) = (a_lo + a_hi · α) + a_hi · x — one subfield squaring + multiplication by α
2. compute the norm N(a) = a_lo · (a_lo + a_hi) + a_hi² · α — two subfield multiplications + one squaring
3. invert the norm: N(a)⁻¹ in the subfield (recursive call)
4. multiply: a⁻¹ = a^(2^k) · N(a)⁻¹

each level reduces inversion to: one subfield inversion + a constant number of subfield multiplications and squarings. the recursion bottoms out at F₂, where 1⁻¹ = 1.

## the base case

inversion in F₂ is trivial: the only nonzero element is 1, and 1⁻¹ = 1. inverting 0 is undefined (division by zero).

```rust
pub fn inv(self) -> Self {
    debug_assert!(self.0 == 1, "inverse of zero");
    self
}
```

## cost analysis

let I(k) be the cost of inversion in F_{2^{2^k}}, measured in subfield multiplications:

```
I(0) = 0                           (F₂: trivial)
I(k) = I(k-1) + c·M(k-1) + S(k-1)  (recursion + multiplications + squarings)
```

where M(k-1) is the subfield multiplication cost and S(k-1) is the subfield squaring cost. since squaring is cheap (linear), the dominant term is the multiplications.

a rough estimate for F₂¹²⁸:
```
I(7) ≈ 7 × (a few subfield multiplications per level)
     ≈ 40-50 tower multiplications
     ≈ 50 × 20 ≈ 1000 cycles
```

compare with Goldilocks inversion: ~64 multiplications × 5 cycles = ~320 cycles. binary field inversion is about 3× slower — the price of the recursive tower structure.

## batch inversion

Montgomery's trick works in any field, including binary towers:

```
batch_invert(a[0..n]):
  prefix[0] = a[0]
  for i in 1..n: prefix[i] = prefix[i-1] · a[i]
  inv = prefix[n-1]⁻¹              // single inversion
  for i in (n-1..0):
    result[i] = inv · prefix[i-1]
    inv = inv · a[i]
  result[0] = inv
```

amortized cost: 1 inversion + 3(n-1) multiplications. for large batches, the per-element cost approaches 3 multiplications — the inversion cost is amortized away.

## see also

- [[binary-fields]] — characteristic 2 and why squaring is linear
- [[tower-construction]] — the recursive structure that enables recursive inversion
- [[karatsuba]] — the multiplication algorithm (inversion's inner loop)
- [[f2-vs-fp]] — inversion cost comparison between binary and prime fields
