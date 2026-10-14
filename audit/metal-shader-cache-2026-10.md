---
tags: lens, audit, gpu, metal
crystal-type: report
crystal-domain: comp
date: 2026-10-14
---
# Metal shader start-up — binary archive cache, 2026-10-14

Machine: Apple M4 Max, 48 GiB, macOS 26.4.1 (25E253), Command Line Tools
only (no `xcrun metal` / `metallib`). rustc 1.95.0, `release` profile.
Stand: lens `feat/fast-path-2-metal` (874cfec + docs), honeycrisp/aruminium
as checked out under `.stands/F2`. The machine is shared: the 1-minute load
average is recorded with every row and was 28–110 during these runs (16
cores), so absolute cold times are inflated; ratios and the archive-hit
times are the stable result.

Command: `cargo run --release -p cyber-lens-gpu --features metal --example metal_cold -- 3`.

## method

`Metal::new()` = `Gpu::open` + queue + MSL library (`newLibraryWithSource`,
the front end) + twelve compute pipelines (the back end) + small buffers.
Metal keeps two OS caches (front end: source → AIR; back end: function →
GPU binary). To measure a first run without clearing any system cache,
the example copies itself into a fresh `.app` bundle with a new
`CFBundleIdentifier` for every "cold" row: Metal's shader cache is per
bundle (`$(getconf DARWIN_USER_CACHE_DIR)/<bundle id>/com.apple.metal*`),
so that copy starts with both caches empty and compiles the shipped
source exactly as a first run does. The example deletes its bundles and
the cache directories they created afterwards.

Each row is a fresh process. Columns from `Metal::startup()`: library
(front end or metallib load), pipelines (all twelve), archive load/save,
and archive hits; the permute hash is a hemera hash of a 4096-state GPU
permutation and is identical on every row.

A nonce (`LENS_SHADER_NONCE`, diagnostic) adds an unused argument to every
kernel: that is how a kernel change looks to both caches. A comment-only
nonce is not enough — it misses the front-end cache but the back-end cache
still hits (pipelines 1.7 ms), because the function bitcode is unchanged.

## results

### sweep 1 (load 90–109)

| round | scenario | load | Metal::new ms | library ms | pipelines ms | archive ms (load / save) | hits |
|---|---|---|---|---|---|---|---|
| 0 | cold OS cache, serial, no archive (old path) | 93.5 | 4765 | 139 | 4562 | – | 0 |
| 0 | warm OS cache, serial, no archive | 90.5 | 51 | 1.3 | 1.9 | – | 0 |
| 0 | cold OS cache, concurrent, no archive | 90.5 | 4610 | 138 | 4415 | – | 0 |
| 0 | warm OS cache, concurrent, no archive | 93.2 | 46 | 1.2 | 2.4 | – | 0 |
| 0 | cold OS cache, concurrent, archive absent (writes it) | 93.2 | 3945 | 135 | 3710 | 0.0 / 40 | 0 |
| 0 | warm OS cache, archive hit | 92.4 | 53 | 1.2 | 0.9 | 1.1 / 0 | 12 |
| 0 | **cold OS cache, archive hit** | 92.4 | **254** | 186 | **0.9** | 1.1 / 0 | 12 |
| 0 | cold OS cache, archive hit, serial | 92.4 | 303 | 217 | 0.6 | 1.2 / 0 | 12 |
| 1 | cold OS cache, serial, no archive (old path) | 92.4 | 4730 | 136 | 4450 | – | 0 |
| 1 | warm OS cache, serial, no archive | 93.0 | 34 | 1.3 | 2.6 | – | 0 |
| 1 | cold OS cache, concurrent, no archive | 93.0 | 3450 | 140 | 3247 | – | 0 |
| 1 | warm OS cache, concurrent, no archive | 92.5 | 97 | 1.4 | 1.9 | – | 0 |
| 1 | cold OS cache, concurrent, archive absent (writes it) | 92.5 | 5201 | 128 | 4983 | 0.1 / 38 | 0 |
| 1 | warm OS cache, archive hit | 99.2 | 40 | 0.8 | 0.7 | 0.8 / 0 | 12 |
| 1 | **cold OS cache, archive hit** | 99.2 | **191** | 131 | **1.2** | 1.1 / 0 | 12 |
| 1 | cold OS cache, archive hit, serial | 99.2 | 183 | 131 | 0.7 | 1.1 / 0 | 12 |
| 2 | cold OS cache, serial, no archive (old path) | 99.2 | 4595 | 139 | 4404 | – | 0 |
| 2 | warm OS cache, serial, no archive | 104.9 | 48 | 3.4 | 6.5 | – | 0 |
| 2 | cold OS cache, concurrent, no archive | 104.9 | 12093 | 136 | 11897 | – | 0 |
| 2 | warm OS cache, concurrent, no archive | 108.0 | 56 | 0.8 | 2.1 | – | 0 |
| 2 | cold OS cache, concurrent, archive absent (writes it) | 108.0 | 11273 | 179 | 10967 | 0.0 / 36 | 0 |
| 2 | warm OS cache, archive hit | 109.0 | 64 | 0.7 | 1.9 | 1.1 / 0 | 12 |
| 2 | **cold OS cache, archive hit** | 109.0 | **235** | 137 | **4.2** | 1.1 / 0 | 12 |
| 2 | cold OS cache, archive hit, serial | 109.0 | 210 | 133 | 0.7 | 1.3 / 0 | 12 |

### single runs at load 27–30 (same binary, `metal_cold one …`)

| scenario | load | Metal::new ms | library ms | pipelines ms |
|---|---|---|---|---|
| cold (nonce), serial, no archive | 28.8 | 4296 | 138 | 4127 |
| cold (fresh bundle), serial, no archive | 28.6 | 4578 | 118 | 4415 |
| cold (nonce), concurrent, no archive | 28.8 | 2076 | 115 | 1930 |
| cold (nonce), concurrent, no archive | 28.8 | 2418 | 112 | 2278 |
| cold (fresh bundle), concurrent, archive absent (writes, save 35 ms) | 29.6 | 2028 | 114 | 1821 |
| cold (fresh bundle), concurrent, no archive | 29.6 | 2020 | 118 | 1860 |
| **cold (fresh bundle), archive hit** | 29.6 | **160** | 114 | **0.8** |
| cold (fresh bundle), archive hit, serial | 29.6 | 163 | 117 | 0.7 |
| warm OS cache | 28.6 | 30–60 | 0.8 | 1.6 |

## reading

- The cold cost is the **back end**: 4.1–4.6 s serial for the twelve
  pipelines at load ~28–100 (the ~0.7 s figure was an idle machine); the
  front end is 0.11–0.14 s.
- **Concurrent pipelines** halve it when the machine has idle cores
  (4.1–4.4 s → 1.8–2.3 s at load ~29); at load 90–110 the Metal compiler
  service has no spare cores and the gain disappears (3.5–12 s, noisy).
  This is the only help a *new* kernel source gets (no archive can exist
  for it yet).
- **Archive hit with a cold OS cache**: pipelines 0.6–4 ms, `Metal::new`
  0.16–0.25 s, of which the front end is 0.11–0.19 s. Independent of load.
  Writing the archive costs 35–40 ms once (1.7 MB file).
- Warm OS cache: 30–100 ms either way (mostly `Gpu::open`); the archive
  adds ~1 ms to load.
- When does the archive hit while the OS cache is cold? The OS cache is
  shared by all unbundled binaries and is size-bounded (264 MB here, five
  rotating generations), and every app bundle (cyb.app, a test harness)
  has its own: the archive is per user, shared across executables and
  bundles. A *new* lens build with changed kernels gets a new key: the
  first process compiles (concurrently) and writes; every later process,
  from any executable, hits.

## what remains

- **No offline compiler here.** `build.rs` embeds a metallib when
  `xcrun -f metal` succeeds (Xcode or the downloadable Metal toolchain);
  that path is built but untested on this machine. A metallib would also
  remove the front end (~0.12 s) — the archive cannot, since pipelines
  need the `MTLFunction`.
- The front end (0.11–0.19 s cold) remains on an archive hit. Options:
  the metallib above, or compiling per-kernel libraries in parallel
  (needs `#if` guards around each kernel in the MSL).
- WebGPU: wgpu 28 `PipelineCache` is Vulkan-only (Metal and DX12
  unimplemented); not used. On Android (Vulkan) it is the analogue of
  this archive — open.
- The archive file is trusted to Metal's own validation; a corrupt,
  truncated or bit-flipped file is rejected (tested), but a crafted file
  in a world-writable `$LENS_SHADER_CACHE` would reach the Metal loader.
  The default directory is the user's own cache directory.
