# squeeze_field panics for wide fields

date: 2026-09-24 · revision: 392abe752793c3e0b98f2d1d57245ba1e3173673 (origin/master)

## finding

`Transcript::squeeze_field<F: Reduce>` (`core/src/transcript.rs`) always
squeezed exactly one 32-byte hemera hash and passed it straight to
`F::reduce`. `strata_proof::Reduce`'s contract documents a per-field byte
requirement — Goldilocks needs 8, F₂¹²⁸ needs 16, F_q needs 64 — and every
`Reduce` implementation asserts it (`strata/genies/rs/src/algebra_impl.rs:82`
panics with `need at least 64 bytes for Fq` below the threshold). A single
32-byte squeeze silently satisfied the narrow fields by accident and always
failed for F_q.

command:

```
$ cargo test -p cyber-lens-porphyry batch_roundtrip
thread 'tests::batch_roundtrip' panicked at strata/genies/rs/src/algebra_impl.rs:82:9:
need at least 64 bytes for Fq
```

reachable from `Transcript::squeeze_field::<Fq>`, called by
`Porphyry::batch_open` (`cyber_lens_core::Lens<Fq>` impl) for every Fiat-Shamir
challenge over F_q — a panic on the verifier side of this path is a Fiat-Shamir
DoS, not just a test failure.

## fix

`squeeze_field<F: Reduce + Codec>` now squeezes repeatedly, chaining the
sponge state each call (`Transcript::squeeze` already re-seeds), until
`F::byte_len()` bytes are collected, then reduces exactly that many. For
Goldilocks and F₂¹²⁸, one squeeze already covers `byte_len()`, so the loop
runs once and the challenge is bit-identical to before
(`squeeze_field_matches_manual_single_squeeze_for_a_narrow_field`). For F_q it
now squeezes twice and no longer panics
(`squeeze_field_does_not_panic_for_a_wide_field`,
`squeeze_field_is_deterministic_for_a_wide_field`).

## verified

```
$ cargo test -p cyber-lens-core
test result: ok. 13 passed; 0 failed

$ cargo test -p cyber-lens-porphyry batch_roundtrip
test tests::batch_roundtrip ... ok

$ cargo check --tests --workspace
Finished, no errors

$ cargo test --workspace
every crate: test result: ok, 0 failed
```

## remains

`Reduce::reduce` still panics rather than returning `Option<Self>` on a
short buffer (defense in depth for any other caller that under-supplies
bytes); out of scope here since `squeeze_field` is the only caller found by
a repo-wide grep and it now always supplies the right length.
