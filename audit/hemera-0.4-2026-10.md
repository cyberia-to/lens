---
tags: lens, audit, gpu, hemera, performance
crystal-type: audit
crystal-domain: comp
date: 2026-10-16
---
# lens on hemera 0.4 — receipt, 2026-10-16

lens `release/0.2` moved to `cyber-hemera` 0.4 (hemera `release/0.4`,
7237a23; the profile and the list of dependents are hemera's
`audit/profile-0.4-2026-10.md` §1, §5). Branch `feat/hemera-0.4`.

## 1. what changed

| place | 0.3 | 0.4 |
|---|---|---|
| `Cargo.toml` | `cyber-hemera = { version = "0.3", path }` | `version = "0.4"` |
| `gpu/src/consts.rs` | `[u64; 144]` | `[u64; 240]` (`NUM_CONSTANTS`), `INTERNAL = 192`; a unit test pins the round shape |
| `gpu/src/msl.rs` | `ginv` / `finv` addition chains, circ(2,3,1,1) `mat4` and wide `fexternal`, 4 + 16 + 4 rounds with `RC[128 + r]`, Montgomery batch inversion across K states, `RC[144]` | reference M4 `[[5,7,1,3],[4,6,1,1],[1,3,5,7],[1,1,4,6]]` (hemera's schedule, canonical and wide), 6 + 48 + 6 rounds with x⁷ on lane 0 and `RC[192 + r]`, `RC[240]`; the `(d₀+1)·y + rest + c` carry of the next S-box input kept; products by `ulong` `mul`/`mulhi` |
| `gpu/src/wgsl.rs`, `wgsl2.rs` | `K[160]` (constants then diagonal at 144), `ginv`, `permute2` with a shared inversion | `K[256]` (diagonal at 240), `partial_round` with x⁷, one state a thread in every permuting kernel (`permute2` removed) |
| `gpu/src/webgpu.rs`, `webgpu2.rs` | two states a thread | one state a thread (dispatch sizes) |
| `gpu/src/metal.rs`, `build.rs` | `PER_THREAD = 4` | `PER_THREAD = 1` (§3) |
| `gpu/src/mine.rs` | — | unchanged (the sponge view of the 168-byte header is the same) |
| CPU paths (`rspcs` backend: `permute_batch`, grinding, trees) | — | unchanged code; they call hemera, so they follow it |
| specs | ~736 constraints, depth 40 (x⁻¹) | ~1,440 constraints, depth 180 (`commitment`, `binary-tower`, `isogeny-curves`, `polynomial-ring`, `backend`) |

Pinned digests: lens tests pin none of hemera's outputs outside the GPU
suite (every CPU test compares with `cyber_hemera` at run time; the
`core` serde fixture is synthetic). The GPU suite now pins hemera 0.4's
four published permutation vectors (`vectors/hemera.json`).

## 2. correctness

M4 Max, macOS 26.4.1, `LENS_GPU_REQUIRE=metal,webgpu`.

| suite | result |
|---|---|
| `cargo test --release -p cyber-lens-gpu --features metal,webgpu` | 4 unit + 17 differential + 4 shader-cache: all pass (Metal and WebGPU both required) |
| `cargo test --release --workspace` | all pass (22 test binaries; rspcs 43, incl. `Cpu.permute` = `hemera::permute` and grinding vs `hemera::hash`) |

differential tests rewritten or added for 0.4:

- `edge_states_equal_hemera` — zeros, p − 1, ones, twos, every unit
  vector (1 and p − 1), alternating 0 / p − 1, iota, and hemera's four
  published vectors (checked against the pinned outputs first), alone
  and 4 099 times over, against `hemera::permutation::permute` itself.
- `a_zero_partial_sbox_input_in_every_batch_position` (was the 0⁻¹ test)
  — states crafted backwards through the reference-matrix M_E and six
  full rounds so the first partial S-box input is 0, at every position of
  a group of eight.
- `metal_states_per_thread_equal_cpu` (was the shared-inversion test) —
  k = 1, 2, 3, 4, 8: permutations, grinding, and header mining whose nonce
  equals the CPU's and passes `hemera::hash` of the header.
- unchanged and green: random permutations (4 096 / 5 000 / 70 001
  states with 0, p − 1 and small lanes), grinding (lane and hash40, past
  the first dispatch), sponges, coset leaves, Merkle levels, byte trees,
  line sessions, NTTs, async WebGPU, header mining (`mining_finds_the_cpu_nonce_and_hemera_agrees`: the CPU digest equals `hemera::hash`, every device returns the CPU's nonce).

## 3. performance

Method: `gpu/examples/hemera_bench.rs` — permutations of 2^20 random
states (with zero lanes) through `Backend::permute` (GPU rows include the
host copy in and out), and header mining (`mine`, target 0 so every
nonce of a 2^22 dispatch is tried; four permutations a nonce) — the
mining column is the clean device throughput. Every row checked against
the CPU backend. The 0.3 binary (lens `origin/release/0.2` + hemera
`origin/main` 1c385e5) and the 0.4 binary alternated, 3 rounds × 7 runs;
the table gives the median of the three per-round medians. The machine
was shared throughout: 1-minute load average 48–72 in the final rounds
(73–137 earlier); the CPU rows are dominated by that load.

| backend | 0.3 permute Mperm/s | 0.4 permute Mperm/s | 0.3 mine MH/s | 0.4 mine MH/s | 0.4 / 0.3 (mine) |
|---|---|---|---|---|---|
| Metal, default k (0.3: 4, 0.4: 1) | 50.8 | 43.4 | 14.97 | 14.07 | 0.94 |
| Metal k = 1 | 45.7 | 43.4 | 14.32 | 14.07 | 0.98 |
| Metal k = 2 | 51.3 | 42.9 | 15.64 | 13.18 | 0.84 |
| Metal k = 4 | 50.8 | 38.5 | 14.97 | 11.20 | 0.75 |
| Metal k = 8 | 51.8 | 34.0 | 15.30 | 10.04 | 0.66 |
| WebGPU (wgpu on Metal) | 10.6 | 11.4 | 5.93 | 4.41 | 0.74 |
| CPU backend, all cores | 8.3 | 6.4 | — | — | (0.77 permute) |

Metal: 0.4 mining at the new default 14.1 MH/s = 56 M permutations/s
(0.3 at its default 15.0 MH/s, at its best k = 2 15.6). The best 0.3 row
against the best 0.4 row: 0.90×.

reading:

- 0.3 put several states on a thread to share the partial rounds'
  inversion (Montgomery's trick); 0.4 has nothing to share, and each extra
  state costs 16 more 64-bit registers: k = 1 is now fastest, k = 8 loses
  a third. Hence `PER_THREAD` 4 → 1.
- 0.4 does ~1.57× the multiplications of 0.3's k = 4 GPU kernel (1 728
  against ~1 100: the inversion chain amortised over four states), yet
  Metal mining lost only 6 % at the default: the 0.3 kernel was not
  multiplication-bound (the 69-deep inversion chain serialised, and the
  K-state arrays cost occupancy).
- tried, kept: one permutation call site in the Metal mining kernel
  (k = 2: 11.0 → 12.8 MH/s, k = 4: 10.0 → 11.7, k = 1 unchanged); `ulong`
  `mul`/`mulhi` for the fast path's products (+2–3 % at k = 1, three
  alternating rounds: 13.4–13.6 → 13.7–14.0); WebGPU one state a thread
  (mining 4.53–4.65 → 4.76–5.39 MH/s, three alternating rounds against
  two states a thread).
- tried, dropped: no unrolling of the 48 partial rounds (no change);
  native `fsqr` (no gain over the 32-bit-limb square).
- WebGPU loses more (0.74×) than Metal: its Goldilocks is canonical after
  every operation over `vec2<u32>` limbs, so the 720 diagonal
  multiply-adds of the partial rounds cost in full; a lazy-representative
  WGSL kernel (the Metal fast path's structure) is the open lever.
- CPU: hemera's own receipt measures the batched path at 0.60× per core
  (§3 there); the lens CPU backend over all cores reads 0.77× here under
  load 48–72 on 16 cores — take hemera's figure.

Raw tables of the final rounds: appendix. Reproduce with

```
# 0.4 (this branch)
cargo run --release -p cyber-lens-gpu --features metal,webgpu --example hemera_bench -- 7 1,2,4,8
LENS_GPU_REQUIRE=metal,webgpu cargo test --release -p cyber-lens-gpu --features metal,webgpu
```

the 0.3 row: the same example file at lens `e11eaa8` beside hemera
`1c385e5`.

## 4. needs from hemera

Nothing blocking. `ROUND_CONSTANTS_U64`, `NUM_CONSTANTS`, `NUM_EXTERNAL`,
`MATRIX_DIAG_16`, `ROUNDS_F` / `ROUNDS_P` / `WIDTH` and
`permutation::{permute, permute_batch}` are all the GPU code reads.

## appendix — final alternating rounds (`hemera_bench 7 1,2,4,8`)

```
== before round 1 load averages: 71.56 82.56 90.26
| backend | permute 2^20: Mperm/s median | best | mine: MH/s median | best | Mperm/s (mine) | equal to cpu |
|---|---|---|---|---|---|---|
| cpu (all cores) | 4.0 | 4.4 | 0.76 | 0.99 | 3.1 | true |
| metal k=1 | 36.8 | 40.3 | 14.48 | 14.51 | 57.9 | true |
| metal k=2 | 36.0 | 51.1 | 16.48 | 16.49 | 65.9 | true |
| metal k=4 | 36.2 | 53.8 | 15.03 | 15.83 | 60.1 | true |
| metal k=8 | 28.6 | 55.2 | 16.47 | 16.56 | 65.9 | true |
| webgpu | 9.8 | 14.3 | 6.49 | 6.96 | 26.0 | true |
== after round 1 load averages: 67.02 80.49 89.25
| backend | permute 2^20: Mperm/s median | best | mine: MH/s median | best | Mperm/s (mine) | equal to cpu |
|---|---|---|---|---|---|---|
| cpu (all cores) | 6.1 | 7.0 | 0.81 | 0.85 | 3.3 | true |
| metal k=1 | 43.3 | 44.2 | 14.07 | 14.08 | 56.3 | true |
| metal k=2 | 23.6 | 38.8 | 13.50 | 13.51 | 54.0 | true |
| metal k=4 | 38.7 | 39.1 | 12.15 | 12.35 | 48.6 | true |
| metal k=8 | 33.8 | 35.4 | 10.19 | 10.83 | 40.7 | true |
| webgpu | 11.2 | 12.5 | 4.25 | 4.40 | 17.0 | true |
== before round 2 load averages: 60.31 77.87 88.05
| backend | permute 2^20: Mperm/s median | best | mine: MH/s median | best | Mperm/s (mine) | equal to cpu |
|---|---|---|---|---|---|---|
| cpu (all cores) | 11.5 | 12.9 | 1.41 | 1.46 | 5.6 | true |
| metal k=1 | 45.7 | 46.4 | 13.79 | 14.29 | 55.1 | true |
| metal k=2 | 51.3 | 51.8 | 15.02 | 15.89 | 60.1 | true |
| metal k=4 | 50.8 | 51.4 | 14.93 | 15.18 | 59.7 | true |
| metal k=8 | 51.8 | 52.9 | 15.30 | 15.72 | 61.2 | true |
| webgpu | 10.6 | 14.9 | 5.93 | 6.04 | 23.7 | true |
== after round 2 load averages: 55.41 75.66 87.02
| backend | permute 2^20: Mperm/s median | best | mine: MH/s median | best | Mperm/s (mine) | equal to cpu |
|---|---|---|---|---|---|---|
| cpu (all cores) | 6.4 | 7.4 | 0.87 | 0.90 | 3.5 | true |
| metal k=1 | 43.4 | 43.6 | 14.07 | 14.08 | 56.3 | true |
| metal k=2 | 42.9 | 43.0 | 13.11 | 13.52 | 52.5 | true |
| metal k=4 | 38.5 | 38.7 | 11.20 | 11.66 | 44.8 | true |
| metal k=8 | 34.0 | 34.5 | 9.98 | 10.36 | 39.9 | true |
| webgpu | 11.4 | 12.0 | 4.41 | 4.50 | 17.7 | true |
== before round 3 load averages: 48.73 72.46 85.54
| backend | permute 2^20: Mperm/s median | best | mine: MH/s median | best | Mperm/s (mine) | equal to cpu |
|---|---|---|---|---|---|---|
| cpu (all cores) | 8.3 | 10.9 | 1.30 | 1.39 | 5.2 | true |
| metal k=1 | 46.1 | 46.2 | 14.32 | 14.48 | 57.3 | true |
| metal k=2 | 52.6 | 52.8 | 15.64 | 16.03 | 62.6 | true |
| metal k=4 | 51.6 | 52.1 | 14.97 | 15.68 | 59.9 | true |
| metal k=8 | 53.2 | 53.9 | 15.16 | 15.56 | 60.6 | true |
| webgpu | 14.5 | 15.0 | 5.82 | 6.11 | 23.3 | true |
== after round 3 load averages: 50.02 71.31 84.83
| backend | permute 2^20: Mperm/s median | best | mine: MH/s median | best | Mperm/s (mine) | equal to cpu |
|---|---|---|---|---|---|---|
| cpu (all cores) | 6.4 | 7.1 | 0.87 | 0.89 | 3.5 | true |
| metal k=1 | 43.5 | 44.1 | 13.97 | 14.09 | 55.9 | true |
| metal k=2 | 42.9 | 43.7 | 13.18 | 13.48 | 52.7 | true |
| metal k=4 | 38.1 | 38.8 | 11.08 | 11.97 | 44.3 | true |
| metal k=8 | 34.0 | 34.4 | 10.04 | 10.64 | 40.2 | true |
| webgpu | 12.1 | 12.3 | 4.41 | 4.47 | 17.7 | true |
```
