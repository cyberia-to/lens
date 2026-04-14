# F_p encoding

how tropical elements and matrices are represented over the Goldilocks field F_p (p = 2⁶⁴ - 2³² + 1).

## element encoding

a tropical element is a single F_p value with the following interpretation:

| F_p value | tropical meaning |
|-----------|-----------------|
| 0 | tropical multiplicative identity (zero cost) |
| 1, 2, ..., p-2 | finite tropical elements (costs/weights) |
| p-1 | +∞ (tropical additive identity, unreachable) |

the usable range is [0, p-2], giving 2⁶⁴ - 2³² distinct finite values — sufficient for any practical optimization problem.

## +∞ sentinel

p-1 is the sentinel for tropical +∞. the arithmetic must respect:

- min(a, p-1) = a for all a (p-1 is the identity for tropical add)
- a + (p-1) = p-1 for all a (p-1 is absorbing for tropical mul)

the second rule overrides standard field addition: when either operand is the sentinel, the result is the sentinel. implementation checks for p-1 before performing field addition.

## overflow protection

tropical multiplication is F_p addition. for finite elements a, b in [0, p-2]:

```
trop_mul(a, b) = if a + b < p-1 then a + b else p-1
```

if the sum reaches or exceeds p-1, it saturates to +∞. this prevents wrap-around that would produce incorrect tropical results. note: standard F_p addition wraps mod p, but tropical addition must saturate.

## matrix serialization

tropical matrices are serialized in row-major order:

```
matrix A ∈ T^(m × n) → [A_{11}, A_{12}, ..., A_{1n}, A_{21}, ..., A_{mn}]
```

total length: m × n F_p elements. each element is a 64-bit little-endian unsigned integer (Goldilocks canonical representation).

padding: for matrices smaller than the allocated size, unused entries are filled with p-1 (+∞). this ensures unused rows/columns are "disconnected" and do not affect tropical operations.

## (max, +) encoding

for the dual semiring, values are negated: neg(a) = p - 1 - a (for finite elements). the sentinel p-1 maps to 0, which serves as -∞ in the (max, +) encoding. see [dual.md](dual.md).

## identity matrix encoding

the tropical identity matrix I ∈ T^(n × n):
- I_{ii} = 0 (diagonal: zero cost self-loop)
- I_{ij} = p-1 for i ≠ j (off-diagonal: unreachable)

## zero matrix encoding

the tropical zero matrix 0 ∈ T^(n × n):
- all entries = p-1 (+∞)
