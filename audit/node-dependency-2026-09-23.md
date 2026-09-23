---
tags: lens, audit, node
crystal-type: audit
crystal-domain: computer science
---

# Node dependency review — 2026-09-23

The node's local Nox, Zheng and BBG manifests already require Lens 0.2.0.
The corresponding implementation was still uncommitted. This change preserves
that implementation together with its specifications and previously untracked
tests, so a source revision can reproduce the node's current dependency behavior.

The reviewed base is `9b134c4cc90c051bc3bafb330e1f7a7db9be9a98`, including
the tensor-Merkle and exact public-table backends. Capturing only the package
version changes would omit the commitment format consumed by BBG certificates.

## Included behavior

- Brakedown commitment version 2: systematic raw-symbol prefix, integer encoding
  dimensions, separate raw/tensor domains, and dimension-bound tensor roots.
- Opening version 3: the complete transcript sampling schedule, one canonical
  payload per distinct sampled column, authenticated paths, bounded dimensions,
  and exact canonical field encodings.
- Unsupported empty and multi-point Brakedown/Porphyry batches fail closed.
- Matching 0.2.0 backend, facade and CLI manifests; scalar/isogeny contracts;
  mutation, format-version, systematic-encoding and batch regressions.

Brakedown version-1 commitments and old dense openings require regeneration.
PublicTensor retains its separate version-1 domain and complete-table behavior.
No dependency was silently downgraded to make the build resolve.

## Verification

The review uses an isolated worktree; the original working tree is preserved.
Library `Cargo.lock` is intentionally ignored by this repository. A local lock
is generated offline for these checks; the node release must retain its own
tracked lock and the exact source revision for every path dependency.

On macOS arm64 with Rust 1.95.0:

- `cargo generate-lockfile --offline`: passed.
- `cargo check --workspace --all-features --all-targets --locked`: passed without
  warnings.
- `cargo test --workspace --all-features --locked`: 128 tests and one doctest
  passed, including the exhaustive serialized-opening bit-flip scan.
- `cargo fmt -p cyber-lens-brakedown -p cyber-lens-porphyry -- --check`: passed.
- `git diff --check`: passed.

The consumer's own lockfile and integration tests remain the authority for the
assembled node; these checks validate the Lens workspace independently.

## Limits

This review establishes a reproducible implementation dependency, not a new
cryptographic security result. The sampled Brakedown protocol still has the
conditional security boundary described in [the scalar contract](../specs/scalar-field.md).
Exact public-table consumers must use PublicTensor or authenticate the complete
raw table. These backends disclose data and do not establish zero knowledge.

Porphyry's legacy single-point verifier still does not authenticate intermediate
polynomial reductions. Rejecting unsupported batches does not repair that
underlying construction. It is outside the node's Goldilocks execution path;
production Fq proof support needs a separate implementation and review.

Full-workspace formatting also has inherited differences in unchanged CLI/core
files. The modified Brakedown and Porphyry packages are checked separately; this
dependency change does not mix in unrelated formatting edits.
