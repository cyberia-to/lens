# agent collaboration

jali (जाली) — polynomial ring arithmetic R_q = F_p[x]/(x^n+1) over Goldilocks.

## project structure

```
jali/
├── rs/              core library (jali crate, depends on nebu, no_std)
│   └── src/lib.rs   RingElement, NTT, noise tracking, sampling, automorphisms
├── cli/             command-line tool (jali-cli crate)
│   └── src/main.rs  ring ops, NTT, sample, bench commands
├── specs/       canonical specifications (7 docs)
└── docs/explanation/ educational articles (5 files)
```

no wgsl/ directory. ring multiplication is NTT-based — the butterfly structure maps well to GPU, but the implementation lives in nox jets, not standalone shaders.

## key invariants

- **depends on nebu**: jali uses nebu for Goldilocks F_p scalar arithmetic and NTT roots of unity. nebu is the only dependency.
- **`#![no_std]`**: the core library is embeddable anywhere. no heap allocation in the hot path (ring degree is compile-time const generic or fixed parameter set).
- **negacyclic NTT**: multiplication in R_q = F_p[x]/(x^n+1) uses the negacyclic NTT — standard NTT with a twisting pre-/post-multiply by powers of ψ (a primitive 2n-th root of unity). Goldilocks has 2^32-th roots of unity, so n up to 2^31 is supported.
- **noise tracking**: every FHE-relevant operation tracks noise growth. NoiseBudget is a log₂ bound. addition grows noise by +1 bit, multiplication by log₂(n) + sum of input bounds. bootstrap resets noise to a fixed level.
- **dual representation**: RingElement can be in coefficient form or NTT form. lazy conversion — stay in whichever domain the current operation needs. multiply-heavy workloads stay NTT; add-heavy stay coefficient.
- **automorphisms**: σ_k: x → x^{5^k mod 2n}. in NTT domain this is a permutation (zero arithmetic). in coefficient domain it is index remap + conditional negation.
- **deterministic sampling**: all sampling (uniform, ternary, CBD, Gaussian) is seeded for reproducibility. same seed → same ring element.

## running

```
cargo test -p jali            # core library tests
cargo run -p jali-cli -- help # CLI tool
```

## relationship to other repos

| repo | role |
|------|------|
| nebu | Goldilocks field arithmetic (the scalar field, F_p) — jali's only dependency |
| kuro | F₂ tower arithmetic (binary regime, complementary) |
| mudra | protocols built on jali (seal = ML-KEM, veil = TFHE) |
| zheng | proof system (ring-aware PCS uses jali for NTT batching) |
| nox | VM (jets for gadget_decompose, ntt_batch, key_switch) |

jali is the ring layer. nebu provides scalars. mudra and zheng consume ring elements.

## writing code

- reference specs in specs/ are the source of truth. code follows spec.
- `#[inline]` on ntt_butterfly, pointwise_mul, ring_add. `#[inline(always)]` only on coefficient-wise add/sub (trivially parallel, no branching).
- avoid `#[inline(always)]` on NTT routines — they have loops and the compiler should decide unrolling.
- all new operations need test vectors. cross-verify NTT against naive polynomial multiplication.
- ring degree n is a parameter, not hardcoded. support 1024, 2048, 4096.
- NTT roots come from nebu: ψ = g^{(p-1)/2n} where g is the Goldilocks generator. precompute twiddle tables at initialization.
- noise bounds are conservative (worst-case). never underestimate noise — correctness depends on it.
