# audit: Tensor opening's query values are never checked against the commitment

date: 2026-09-23 · revision: 392abe752793c3e0b98f2d1d57245ba1e3173673 (origin/master)

## finding

`core/src/types.rs`'s doc comment for `Opening::Tensor` reads:

```rust
/// Brakedown, Ikat, Porphyry: recursive tensor decomposition
/// with proximity testing via codeword queries.
Tensor {
    round_commitments: Vec<Commitment>,
    final_poly: Vec<u8>,
    query_responses: Vec<(usize, Vec<u8>)>,
},
```

`brakedown/src/lib.rs`'s `verify` (Check 2) replays the transcript and, for
each `(idx, _qval)` in `query_responses`, checks that `idx` matches the
Fiat-Shamir challenge:

```rust
let (qidx, _qval) = &query_responses[query_idx];
if *qidx != expected_idx {
    return false;
}
```

the leading underscore is the tell: `_qval` — the codeword value at that
queried position — is bound into no check anywhere in `verify`. `query_idx`
is only ever compared to itself; the corresponding value is read out of the
proof, matched against nothing, and discarded. `Opening::Tensor` carries no
Merkle path or other authentication data for a queried value (contrast
`Opening::Folding`, used by Binius, which has a `merkle_paths: Vec<Vec<Hash>>`
field for exactly this purpose) — so there is no data in the type for `verify`
to check `_qval` against even if it tried. A prover can substitute any byte
string for any queried value without `verify` noticing, added test
`verify_accepts_a_proof_with_corrupted_query_values` in
`brakedown/src/lib.rs` demonstrates this on a real proof for a real
polynomial: every `query_responses` value is overwritten with `0xFF` bytes
and `verify` still returns `true`.

round commitments after the first have the same shape: `round_commitments[0]`
is checked against the public `commitment` (Check 1), but
`round_commitments[1..]` are only absorbed into the transcript — never
checked to be the actual tensor-reduction of anything. combined with the
unchecked query values, a verifier's only real guarantees are: the proof's
first commitment is the claimed one, the queried *indices* were chosen
honestly (not the values behind them), and `final_poly` decodes to the
claimed `value`. nothing ties `final_poly` to the polynomial `commitment`
actually committed to.

## scope checked

`Opening::Tensor` is shared by three of the five lenses:

- `brakedown/src/lib.rs` — this audit's regression test lives here; it is
  the backend zheng and bbg actually use (component table: "Brakedown for
  zheng, bbg openings").
- `ikat/src/lib.rs` — delegates `verify` directly to `Brakedown::verify`
  (`fn verify(...) { Brakedown::verify(...) }`), so it has the identical gap
  with no separate code path to audit.
- `porphyry/src/lib.rs` — its `open` never populates `query_responses` at
  all (always `vec![]`) and its `verify` has no query-checking loop, a
  strictly weaker version of the same gap (checked separately, not the
  subject of the added test).

`Opening::Folding` (Binius) is a different shape with its own
`merkle_paths` field and was not reviewed in this pass.

## why closing this is not a small fix

authenticating a queried codeword value against `round_commitments[i]`
needs `Opening::Tensor` to carry a Merkle authentication path per query (the
way `Opening::Folding` already does for Binius), plus a matching
`Commitment` that is actually a Merkle root over the encoded codeword rather
than the current flat hemera hash (`commit_raw` in each of the three
crates). That is a real protocol change — a new field on a shared type, a
new proof obligation on `open`, and a new check per crate's `verify` — not a
40-minute slice, and one change that needs to land identically across
brakedown, ikat and porphyry to keep them interoperable through the shared
`Opening` type.

## remains

- design the Merkle-path field on `Opening::Tensor` (or a new variant) and
  the corresponding commitment structure, shared across all three
  Tensor-based lenses
- once the type carries proof data for query values, check `round_commitments[1..]`
  are each a valid tensor-reduction step, not just absorbed into the transcript
- porphyry's `verify` needs the same query-checking loop brakedown has,
  once there is something real for it to check
- this closes before zheng or bbg's provable read path (rows 3, 5, 16, 34)
  can be trusted against an adversarial prover — today's Brakedown proofs
  bind only the first commitment and the claimed final value
