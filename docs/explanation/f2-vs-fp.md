# F₂ vs F_p: when to use which

kuro (binary fields, F₂ tower) and nebu (prime fields, Goldilocks) are complementary — not competing. each dominates a different class of computation. choosing correctly yields order-of-magnitude performance gains. choosing wrong costs 32× or more.

## two fields, two algebras

| property | F_p (Goldilocks, nebu) | F₂ tower (kuro) |
|----------|----------------------|-----------------|
| characteristic | p = 2⁶⁴ − 2³² + 1 | 2 |
| element size | 64 bits (one u64) | 1 to 128 bits (tower level) |
| addition | integer add mod p (~3 cycles) | XOR (~1 cycle) |
| multiplication | integer mul + reduction (~5 cycles) | Karatsuba tower (~20 cycles) |
| representation | one integer in [0, p) | n bits packed in a register |

nebu's strength is dense arithmetic: field multiplications, polynomial evaluations, NTT butterflies. kuro's strength is bitwise parallelism: XOR, AND, popcount, bit manipulation.

## the 32× rule for bitwise operations

in a prime field, a single field element holds a 64-bit number. to access individual bits, you must decompose: extract each bit with division and modular arithmetic. in a STARK constraint system, bit decomposition costs approximately 32 constraints (one range check per bit, amortized).

in GF(2), each bit is already a field element. XOR is addition (1 constraint). AND is multiplication (1 constraint). no decomposition needed.

```
                    F_p cost    F₂ cost    ratio
XOR of two bits     ~32          1         32×
AND of two bits     ~32          1         32×
128 parallel ANDs   ~4,096       1         4,096×
bit shift           ~32          0         ∞
```

the 32× factor is not an implementation detail — it is the algebraic distance between characteristic p and characteristic 2. no amount of engineering can close it.

## the 4× rule for field multiplication

general field multiplication goes the other direction. Goldilocks multiplication is one u64×u64 multiply plus a fast reduction — about 5 cycles on modern hardware. a multiplication in F₂¹²⁸ requires Karatsuba decomposition through 7 tower levels — about 20 cycles.

```
                    F_p cost    F₂ cost    ratio
field multiply      ~5 cycles   ~20 cycles  0.25×
field add           ~3 cycles   ~1 cycle    3×
field inversion     ~320 cycles ~1000 cycles 0.3×
```

for workloads dominated by field multiplications (polynomial evaluation, NTT, Poseidon2 rounds), Goldilocks wins by 4×. the binary tower's recursive structure is elegant but cannot match a single hardware multiply instruction.

## the cost matrix

| workload | dominant operations | winner | margin |
|----------|-------------------|--------|--------|
| hash function (Poseidon2) | field mul, S-box | F_p | 4× |
| NTT / polynomial ops | field mul, add | F_p | 3-4× |
| AES verification | XOR, AND, S-box | F₂ | 10-30× |
| SHA-256 verification | XOR, AND, rotations | F₂ | 20-30× |
| BitNet inference | AND, popcount | F₂ | 64× |
| boolean circuit | XOR, AND | F₂ | 32× |
| range check | bit decomposition | F₂ | 32× |
| lookup argument | field inversion, mul | F_p | 3× |

## the cross-algebra boundary

a proof system that uses both fields must cross between them. this crossing has a cost.

to embed an F₂ computation result into an F_p constraint system: the binary value must be range-checked (is each component actually 0 or 1?) and converted to an F_p element. estimated cost: ~766 F_p constraints per crossing.

to go the other direction (embed an F_p result into F₂): the 64-bit value must be decomposed into bits and imported as 64 GF(2) elements. this costs ~64 F₂ constraints for the decomposition.

the boundary cost is fixed per crossing, not per element. for large batches (processing thousands of binary operations before crossing back), the per-operation overhead is negligible. for frequent crossings, the overhead dominates.

**rule of thumb:** cross the boundary as rarely as possible. batch binary work together, batch arithmetic work together.

## decision guide

**use kuro (F₂) when:**
- the computation is inherently bitwise (XOR, AND, bit shifts, rotations)
- you are proving correctness of binary algorithms (AES, SHA, boolean circuits)
- the data is naturally binary (1-bit weights, boolean flags, bit vectors)
- parallelism matters: 128 operations per instruction
- the workload is BitNet inference or quantized SpMV

**use nebu (F_p) when:**
- the computation is inherently arithmetic (polynomial evaluation, matrix multiply with large entries)
- you are computing hashes (Poseidon2 runs over Goldilocks)
- the workload involves NTT or polynomial commitment (FRI/WHIR)
- field inversion is needed frequently (lookup arguments)
- the data represents large numbers (balances, counters, coordinates)

**use both when:**
- a program mixes arithmetic and bitwise operations
- the STARK has both polynomial constraints and boolean constraints
- you need to verify binary computations within an arithmetic proof
- the system naturally partitions into binary and arithmetic sub-circuits

## the architecture

the proof system deploys both fields in their natural domains:

```
                    ┌─────────────────────┐
                    │   arithmetic zone    │
                    │   nebu (Goldilocks)  │
                    │   NTT, Poseidon2,    │
                    │   polynomial ops     │
                    └────────┬────────────┘
                             │
                    ~766 constraints per crossing
                             │
                    ┌────────┴────────────┐
                    │    binary zone       │
                    │    kuro (F₂ tower)   │
                    │    XOR, AND, BitNet, │
                    │    boolean circuits  │
                    └─────────────────────┘
```

the Goldilocks accumulator (top) and binary field operations (bottom) each run in their native algebra. the cross-algebra boundary is crossed only when necessary — typically once per sub-circuit, not once per operation.

## see also

- [[binary-fields]] — the algebraic foundation of F₂
- [[applications]] — workloads that need binary fields
- [[packed-operations]] — how 128 operations fit in one instruction
- [[karatsuba]] — why F₂ multiplication costs ~20 cycles
