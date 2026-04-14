# agent collaboration

genies — isogeny group action arithmetic for post-quantum privacy.

## project structure

```
genies/
├── rs/              core library (genies crate, zero deps, no_std)
│   └── src/lib.rs   F_q arithmetic, curve ops, isogeny, class group action
├── wgsl/            GPU backend (genies-wgsl crate, wgpu compute shaders)
│   └── src/         batch F_q mul/add via WGSL shaders, eval_fq_op helper
├── cli/             command-line tool (genies-cli crate)
│   └── src/main.rs  calc, action, bench commands
├── specs/       canonical specifications (8 docs)
└── docs/explanation/ educational articles (6 files)
```

wgsl/ provides batch F_q GPU operations. 512-bit multi-limb arithmetic has carry dependencies that limit single-element throughput, but batch dispatch (many independent mul/add pairs) amortizes dispatch overhead and saturates GPU memory bandwidth.

## key invariants

- **foreign prime**: the CSIDH-512 prime q = 4 * 3 * 5 * 7 * ... * 587 - 1 is NOT Goldilocks. this is the one module in the cyber stack with a foreign prime. the reason is mathematical: Goldilocks p+1 has no small odd prime factors, making CSIDH impossible over F_p.
- **constant-time**: all F_q arithmetic and curve operations must be constant-time. no branches on secret data, no variable-time multiplication, no data-dependent memory access. the Montgomery ladder uses cswap, not if/else.
- **`#![no_std]`**: the core library is embeddable anywhere. no heap allocation in the hot path.
- **zero production dependencies** in the core library.
- F_q elements are 8 x 64-bit limbs, little-endian. serialized as 64 bytes LE.
- curves are Montgomery form E_A: y^2 = x^3 + Ax^2 + x, identified by the single coefficient A.
- the class group action takes an exponent vector (e_1, ..., e_74) and a curve coefficient A, returns a new A'.
- Barrett reduction for modular arithmetic (precomputed constant for the CSIDH prime).

## running

```
cargo test -p genies           # core library tests
cargo run -p genies-cli -- help # CLI tool
cargo run -p genies-cli -- calc mul <a_hex> <b_hex>
cargo run -p genies-cli -- bench
```

## relationship to other repos

| repo | role |
|------|------|
| mudra | protocols built on genies (CSIDH DH, VRF, VDF, threshold, stealth, blind) |
| nebu | Goldilocks field (the proof backbone, F_p) |
| kuro | F_2 tower arithmetic (binary regime, complementary) |
| hemera | hash function (Poseidon2, commitment anchor) |
| nox | VM (jet dispatch for accelerated isogeny ops) |
| zheng | proof system (verifies isogeny computation via F_q -> F_p folding) |

genies is the only repo that does NOT share a prime with the rest of the stack. the verification bridge (F_q element -> 8 Goldilocks limbs) connects genies to zheng.

## writing code

- reference specs in specs/ are the source of truth. code follows spec.
- `#[inline]` on fq_mul, fq_inv, point operations. `#[inline(always)]` only on fq_add/fq_sub (carry chain, small).
- all new operations need test vectors. cross-verify against SageMath CSIDH implementation.
- constant-time discipline: use conditional selection (cmov pattern), not branches. audit with tools like `dudect` or timing test harnesses.
- F_q multiplication: schoolbook 8x8 limbs -> 16 limbs, then Barrett reduction. NOT Karatsuba — the carry propagation complexity negates Karatsuba's savings at 8 limbs, and schoolbook is easier to make constant-time.
- the action algorithm iterates over 74 primes. each nonzero exponent triggers |e_i| isogenies of degree l_i. positive exponents use E_A, negative use the twist.
