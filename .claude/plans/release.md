# lens 0.1.0 release plan

> Standard structure reference: ~/cyber/hemera
> strata must publish before lens — see strata/.claude/plans/release.md

---

## gap analysis vs hemera template

hemera structure:
```
repo/
├── bench/          separate benchmark crate (publish=false)
├── cli/            binary crate
├── docs/           Diataxis: explanation/, guides/, tutorials/
├── roadmap/        future proposals
├── rs/             core implementation
├── specs/          canonical specification
├── vectors/        JSON test vector pinning
├── wgsl/           GPU backend
├── CHANGELOG.md
├── CLAUDE.md       comprehensive, project-specific
└── Cargo.toml      workspace with lints + panic=abort profiles
```

lens current structure:
```
repo/
├── core/           Lens trait + types
├── brakedown/      impl Lens<Goldilocks>
├── binius/         impl Lens<F2_128>
├── ikat/           impl Lens<Goldilocks> (NTT-aware)
├── assayer/        tropical witness wrapper
├── porphyry/       impl Lens<Fq>
├── src/            facade
└── specs/          canonical spec (8 files) ✓
```

**present:**
- `specs/` — 8 files ✓
- 5 implementations (brakedown, binius, ikat, assayer, porphyry) ✓
- CI: test, clippy, fmt ✓

**missing:**
- `cli/` — no CLI binary (hemera has full tree/prove/verify CLI)
- `bench/` — no benchmark crate (no criterion benchmarks at all)
- `docs/` — no Diataxis documentation
- `roadmap/` — no future proposals
- `vectors/` — no JSON test vector pinning
- `CHANGELOG.md`
- `CLAUDE.md` — current file is wrong (says "specified, not implemented")
- CI: missing `doc` job
- workspace `Cargo.toml` lints + `panic = "abort"` profiles

---

## Phase 1: workspace structure

### 1.1 Cargo.toml — add lints, profiles, bench member

**File:** `Cargo.toml`

```toml
[workspace]
resolver = "3"
members = [
    "core",
    "brakedown", "binius", "ikat", "assayer", "porphyry",
    "src",
    "cli",     # add
    "bench",   # add
]

[workspace.lints.rust]
missing_debug_implementations = "warn"

[workspace.lints.clippy]
unused-async = "warn"

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

Each crate's Cargo.toml must add `[lints] workspace = true`.

Also add the `zheng-compat` feature stub to `brakedown/Cargo.toml` to reserve the API
surface for pattern-0 integration (implementation is post-0.1.0):
```toml
[features]
zheng-compat = []   # reserved: verifier_ccs() for zheng pattern 0 folding
```

### 1.2 CI pipeline — add doc job

**File:** `.github/workflows/ci.yml`

Add fourth job to the existing three (test, clippy, fmt):
```yaml
  doc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo doc --workspace --no-deps
```

Also change fmt job to use nightly rustfmt (matches hemera):
```yaml
  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt
      - run: cargo +nightly fmt --check
```

### 1.3 cli/ crate

**File to create:** `cli/Cargo.toml`
```toml
[package]
name = "lens-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "lens"
path = "src/main.rs"

[dependencies]
cyber-lens = { path = "../src", version = "0.1.0" }
nebu = { workspace = true }
kuro = { workspace = true }
trop = { workspace = true }
genies = { workspace = true }

[lints]
workspace = true
```

**`cli/src/main.rs`** — command-line interface wrapping all five constructions:

```
Commands:
  commit <file> [--algo brakedown|binius|ikat|assayer|porphyry]
      Commit a file as a multilinear polynomial. Prints 32-byte hex commitment.

  open <file> <point-hex> [--algo <name>]
      Generate opening proof that committed poly evaluates to value at point.
      point-hex: space-separated field elements in hex.

  verify <commitment-hex> <file> <point-hex> <value-hex> [--algo <name>]
      Verify a committed polynomial evaluates to value at point.
      Exits 0 on success, 1 on failure.

  bench [--algo <name>] [--vars N]
      Run a quick commit/open/verify cycle and print timings.

  vectors
      Print test vectors for all five constructions (for cross-verification).

  --help
      Print usage with parameter descriptions.
```

Architecture follows hemera's cli/src/main.rs pattern:
- Backend selection via `--algo` flag
- Hex parsing helpers
- Error handling: exit codes, no panics
- Progress line to stderr for large inputs

### 1.4 bench/ crate

**File to create:** `bench/Cargo.toml`
```toml
[package]
name = "lens-bench"
version.workspace = true
edition.workspace = true
publish = false

[dev-dependencies]
criterion = "0.5"
cyber-lens = { path = "../src" }
nebu = { workspace = true }

[[bench]]
name = "commit"
harness = false

[[bench]]
name = "open"
harness = false

[[bench]]
name = "verify"
harness = false

[lints]
workspace = true
```

**Benchmarks to write:**

`bench/benches/commit.rs` — `Brakedown::commit` at sizes 2^10, 2^16, 2^20
`bench/benches/open.rs` — `Brakedown::open` at sizes 2^10, 2^16
`bench/benches/verify.rs` — `Brakedown::verify`; comparison across constructions

Targets:
- commit(2^16): < 10ms
- open(2^16): < 50ms
- verify(2^16): < 5ms

### 1.5 vectors/ — JSON test vector pinning

**File to create:** `vectors/lens.json`

Cross-implementation verification anchors. The CLI `lens vectors` command
prints these; any implementation must match them exactly.

```json
{
  "brakedown": {
    "commit_4": "<32-byte-hex>",
    "commit_16": "<32-byte-hex>",
    "open_roundtrip_4": true,
    "open_roundtrip_16": true
  },
  "binius": {
    "commit_4": "<32-byte-hex>",
    "commit_16": "<32-byte-hex>"
  },
  "ikat": {
    "commit_ring_1024": "<32-byte-hex>"
  },
  "assayer": {
    "dijkstra_3_node": "<32-byte-hex>"
  },
  "porphyry": {
    "commit_4": "<32-byte-hex>"
  }
}
```

**Test file to create:** `src/tests/vectors.rs` — loads JSON, runs all five
constructions, asserts outputs match pinned values.

Generation: run `lens vectors` once the implementation is complete, pin the
output. Any future change that breaks a vector is a breaking change requiring
a version bump.

### 1.6 docs/ — Diataxis structure

**Files to create:**
```
docs/
├── README.md                             index
└── explanation/
    ├── README.md
    ├── why-polynomial-commitment.md      what a PCS is and why lens needs one
    ├── five-constructions.md             when to use each construction
    ├── brakedown.md                      expander graph + tensor decomposition
    ├── binius.md                         binary-native, AND/XOR at 1 constraint
    ├── assayer.md                        LP duality, tropical witness-verify
    ├── fiat-shamir.md                    transcript security model
    └── recursion-cost.md                 in-circuit verification cost per algebra
```

guides/ and tutorials/ as stubs — deferred post-release.

### 1.7 roadmap/

**Files to create:**
```
roadmap/
├── README.md
├── binius-merkle-auth.md       fix Merkle path auth in Binius verifier
├── porphyry-margulis.md        replace Fibonacci hash with Margulis expander
├── domain-tags.md              canonical production Fiat-Shamir domain tags
├── action-ct.md                constant-time Porphyry after genies action_ct
└── pattern0-axis.md            verifier_ccs() for zheng pattern 0 folded opening
```

**`roadmap/pattern0-axis.md`** content — merge from `.claude/plans/pattern0-axis-folded-opening.md`:

Pattern 0 (axis) in nox evaluates a noun polynomial committed in `r4` at position `r6`
and asserts the result equals `r7`. `zheng/src/ccs/patterns.rs` returns `trivial_ccs()`
for `pattern_axis()` because lens does not yet expose a CCS encoding of its verifier.

What lens must provide:

1. **`verifier_ccs(params: &BrakedownParams) -> CCSInstance`** in `brakedown/src/verifier_ccs.rs`
   — returns the Brakedown opening verifier as a CCS instance (~825 constraints across
   `log(n)` levels). Pure function, no I/O, no randomness.

2. **`BrakedownParams`** — stable, serializable struct (`serde` derives, `version: u8`).
   Stored inside zheng `Statement`/`Proof` types — must be stable across lens versions.

3. **`zheng-compat` feature flag** — gates the zheng-types dependency so lens can be
   built without zheng in contexts that don't need it.

Interface contract:
- lens returns a `CCSInstance` using zheng's own type (via `zheng-types` crate or direct dep)
- zheng detects pattern-0 rows in `commit()`, calls `verifier_ccs()`, folds via same
  mechanism as pattern 17 (look)
- Inline wiring constraints (C_0a, C_0b) in `patterns.rs` bind trace registers to
  the opening-proof inputs — zheng can implement these immediately without waiting for lens

Files to create/modify when implementing:

| file | action |
|------|--------|
| `brakedown/src/verifier_ccs.rs` | new — `verifier_ccs()` |
| `brakedown/src/lib.rs` | re-export under `zheng-compat` feature |
| `Cargo.toml` | `zheng-compat` feature, optional `zheng-types` dep |
| `zheng/src/ccs/patterns.rs` | `pattern_axis()` with C_0a / C_0b |
| `zheng/src/lib.rs` | fold `verifier_ccs` sub-instance in `commit()` |

Prerequisite: zheng must publish `zheng-types` before this can land in lens.
Round-trip test: `verifier_ccs()` instantiated with a known opening must satisfy the CCS.

**Scope for 0.1.0:** add `zheng-compat` feature stub to `Cargo.toml` (empty for now)
so the API surface is reserved. Full implementation is post-0.1.0.

### 1.8 CHANGELOG.md

**File to create:** `CHANGELOG.md`
```markdown
# Changelog

## [0.1.0] - 2026-05-14

### Added
- `cyb-lens-core`: Lens trait, MultilinearPoly, Commitment, Opening, Transcript
- `cyb-lens-brakedown`: Margulis expander + tensor decomposition over Goldilocks (18 tests)
- `cyb-lens-binius`: binary folding + hemera Merkle tree over F₂¹²⁸ (16 tests)
- `cyb-lens-ikat`: NTT-batched Brakedown over R_q NTT slots (11 tests)
- `cyb-lens-assayer`: tropical witness + LP dual certificate → Brakedown (15 tests)
- `cyb-lens-porphyry`: expander codes over F_q 512-bit (9 tests)
- `cyber-lens`: facade (28 integration tests)
- `lens-cli`: command-line interface for all five constructions
- `vectors/lens.json`: pinned test vectors for cross-implementation verification

### Fixed
- `specs/commitment.md` §10: corrected status from "specified, not implemented"
  to "implemented" for all five constructions

### Known gaps (tracked in roadmap/)
- Brakedown verifier: EXPANSION_M_PLACEHOLDER must use dynamic codeword size
- Binius verifier: Merkle path authentication not yet enforced
- Porphyry encoding: Fibonacci hash used instead of Margulis expander
- Production Fiat-Shamir domain tags not yet defined as constants
```

### 1.9 CLAUDE.md — replace

**File:** `CLAUDE.md`

Current CLAUDE.md is wrong (says "specified, not implemented") and incomplete.
Rewrite to match hemera's CLAUDE.md structure:

1. agent collaboration (from cyber/midao/dev.md)
2. engineering patterns (from cyber/midao/engineering.md)
3. quality control: 12 passes, severity tiers (from cyber/midao/quality.md)
4. project structure conventions
5. documentation methodology (Diataxis)
6. lens-specific:
   - architecture: `hemera → lens → nox/zheng/bbg`
   - five constructions with their fields and crates
   - do-not-touch zones: Cargo.toml versions, specs/ canonical, Lens trait interface
   - companion repos: strata (path deps), hemera (hemera sponge), nox/zheng/bbg (consumers)
7. remove all "specified, not implemented" language

---

## Phase 2: fix status discrepancy (first commit)

**Files:** `specs/commitment.md` §10, `CLAUDE.md`

Update implementation status table to show all constructions as "implemented"
with test counts. This is the first commit of the release branch:

`docs: correct implementation status — all five constructions implemented`

---

## Phase 3: critical bugs (release-blocking)

### EXPANSION_M_PLACEHOLDER verifier bug
**File:** `brakedown/src/lib.rs` ~line 209

The verifier uses `EXPANSION_M_PLACEHOLDER = 1024` (wrong codeword size for
anything other than a 512-evaluation polynomial). Fix: track `current_size`
through the verification loop, compute `m = EXPANSION * current_size` per round.

This is the highest-priority code fix — the verifier is functionally broken
for general polynomials.

### Binius Merkle path authentication
**File:** `binius/src/lib.rs`

`verify()` checks `round_commitments[0] != *commitment` but does not use
`merkle_paths` to authenticate individual leaves. Soundness gap.

Fix: use the paths in `verify()` to verify each queried leaf against the
round commitment via the Merkle path.

### Porphyry Fiat-Shamir not query-bound
**File:** `porphyry/src/lib.rs`

`verify()` absorbs into transcript but never squeezes challenges — proximity
query indices are not Fiat-Shamir bound. Fix: squeeze challenges after each
round commitment, verify `query_responses` indices match.

---

## Phase 4: twelve quality passes

All twelve passes at release tier. Key items not covered in Phase 1/3:

**Pass 1 — determinism:**
- [ ] Two transcripts with identical absorption produce identical squeeze() values
- [ ] `Transcript::squeeze()` re-seeding is deterministic

**Pass 3 — arithmetic correctness:**
- [ ] `tensor_reduce()` add guards: `evals.len() >= 2 && evals.len() % 2 == 0`
- [ ] Tensor reduction at `r=0` returns even elements, at `r=1` returns odd elements
- [ ] Expander linearity test: if `a + b = c`, then `encode(a) + encode(b) = encode(c)`
- [ ] Verify `cyber_hemera::tree::tree_hash()` is a proper Merkle tree (path-authenticatable)
  not a flat hash — if flat hash, Binius commit is semantically wrong

**Pass 4 — crypto hygiene:**
- [ ] Production domain tags defined as constants in each construction crate:
  `pub const DOMAIN: &[u8] = b"lens/brakedown/v0.1.0";`
- [ ] `grep -r "sha2\|sha3\|blake\|md5" .` returns nothing
- [ ] `TropicalWitness` documented as non-ZK (revealed to verifier)

**Pass 12 — testability:**
- [ ] `vectors/lens.json` loaded and asserted in `src/tests/vectors.rs`
- [ ] Roundtrip proptest: random polynomial, random point, all five constructions
- [ ] Adversarial: tampered `round_commitments[0]` must fail verify
- [ ] Domain separation: proof with domain A fails under domain B transcript
- [ ] Assayer: `source == target`, `assignment = []` edge case

---

## Phase 5: publishing

### Prerequisite: strata 0.1.0 on crates.io

After strata publishes, update `lens/Cargo.toml` workspace deps:

```toml
# Remove path = ... from workspace.dependencies entries.
# Add [patch.crates-io] for local development:
[patch.crates-io]
strata-core = { path = "../strata/core" }
strata-proof = { path = "../strata/proof" }
strata-nebu = { path = "../strata/nebu/rs" }
strata-kuro = { path = "../strata/kuro/rs" }
strata-jali = { path = "../strata/jali/rs" }
strata-trop = { path = "../strata/trop/rs" }
strata-genies = { path = "../strata/genies/rs" }
```

`cargo publish` strips `[patch.crates-io]` — published crates reference registry versions.

### Publishing order

```bash
# Layer 1
cargo publish -p cyb-lens-core

# Layer 2 (any order)
cargo publish -p cyb-lens-brakedown
cargo publish -p cyb-lens-binius
cargo publish -p cyb-lens-porphyry

# Layer 3
cargo publish -p cyb-lens-ikat
cargo publish -p cyb-lens-assayer

# Layer 4
cargo publish -p cyber-lens

# Binary (not published to crates.io — it's a local tool)
# lens-cli: install via cargo install --path cli
```

### Pre-publish check per crate
```bash
cargo publish --dry-run -p <crate>
cargo package --list -p <crate>
cargo doc -p <crate> --no-deps
```

---

## Phase 6: post-release

- [ ] Verify crates.io pages and docs.rs builds
- [ ] `git tag -a v0.1.0 -m "lens 0.1.0" && git push origin v0.1.0`
- [ ] `cargo install --path cli` — verify `lens --help` works
- [ ] Open GitHub issues for roadmap/ items (Binius auth, Porphyry expander, domain tags)

---

## critical file table

| file | issue | priority |
|------|-------|----------|
| `brakedown/src/lib.rs` ~209 | EXPANSION_M_PLACEHOLDER verifier bug | release-blocking |
| `binius/src/lib.rs` | Merkle path auth missing in verify | release-blocking |
| `porphyry/src/lib.rs` | Fiat-Shamir not query-bound | release-blocking |
| `specs/commitment.md` §10 | "specified, not implemented" wrong | first commit |
| `CLAUDE.md` | wrong + incomplete | Phase 1 |
| `cli/` | missing entirely | Phase 1 |
| `bench/` | missing entirely | Phase 1 |
| `vectors/lens.json` | missing cross-impl anchors | Phase 1 |
| `docs/` | missing Diataxis tree | Phase 1 |
| `roadmap/` | missing proposals | Phase 1 |
| `Cargo.toml` | missing lints + profiles | Phase 1 |
| CI `doc` job | missing fourth CI job | Phase 1 |
