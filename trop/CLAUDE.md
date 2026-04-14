# agent collaboration

trop — tropical semiring arithmetic for provable optimization.

## project structure

```
trop/
├── rs/              core library (trop crate, zero deps, no_std)
│   ├── element.rs   Tropical: u64 wrapper, add = min, mul = saturating +
│   ├── matrix.rs    TropMatrix: n x n with tropical matmul, power
│   ├── kleene.rs    Kleene star via Floyd-Warshall (all-pairs shortest paths)
│   ├── determinant.rs  tropical det via Heap's permutation search
│   └── eigenvalue.rs   minimum mean cycle weight via Karp's algorithm
├── wgsl/            GPU compute shaders (trop-wgsl crate)
│   └── shaders/tropical.wgsl  tiled 16x16 tropical matmul
├── cli/             command-line tool (trop-cli crate)
├── specs/       canonical specifications (9 files)
└── docs/explanation/ educational articles (6 files)
```

## key invariants

- zero production dependencies in the core library
- `#![no_std]` — embeddable anywhere
- the tropical semiring (min, +) is NOT a field: no additive inverse
  - tropical addition: a + b = min(a, b) — idempotent
  - tropical multiplication: a * b = a + b (ordinary, saturating)
  - additive identity (tropical zero): +inf (u64::MAX)
  - multiplicative identity (tropical one): 0
- u64::MAX is the sentinel for +infinity throughout
- Kleene star = Floyd-Warshall = all-pairs shortest paths in O(n^3)
- matrix dimension capped at MAX_DIM = 64 (stack-allocated flat array)
- determinant uses naive O(n!) enumeration, capped at n <= 10

## running

```
cargo test -p trop            # core library tests
cargo run -p trop-cli -- help # CLI tool
cargo run -p trop-cli -- calc add 3 7
cargo run -p trop-cli -- calc mul 3 7
cargo run -p trop-cli -- kleene 3 0 1 INF 5 0 2 INF 3 0
cargo run -p trop-cli -- bench
```

## relationship to other repos

| repo | role |
|------|------|
| nebu | Goldilocks field (verification backbone, F_p) |
| kuro | F₂ tower field (binary arithmetic) |
| nox | VM (tropical algorithms run as nox programs) |
| zheng | proof system (verifies optimality via dual certificates) |

## writing code

- reference specs in specs/ are the source of truth. code follows spec.
- `#[inline]` on element add/mul (single comparison or add). `#[inline(always)]` not needed — these are trivial.
- all matrix operations use the flat `[Tropical; MAX_DIM * MAX_DIM]` layout with `i * MAX_DIM + j` indexing.
- the Kleene star initializes with I (min) A, then runs Floyd-Warshall relaxation.
- tropical add is idempotent: a + a = a. this changes everything compared to field arithmetic.
- no negative weights in the u64 encoding — Kleene star always converges.
- verification of optimality uses LP duality certificates, checked over F_p (in zheng), not in trop.
