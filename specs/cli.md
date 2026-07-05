---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: lens cli, lens command-line, lens tooling
---
# lens cli

command-line tool for the commitment layer. commit to a file as a polynomial,
open evaluations, verify openings without the file. wraps the five constructions
of [[commitment]] behind one binary.

the tool lives in the `cli/` crate (package `lens-cli`, binary `lens`), a
workspace member alongside the construction crates. it follows the house
convention shared by [[hemera]] and the [[strata|algebra]] CLIs: one binary,
`lens <command> <args> [flags]`, hand-rolled argument parsing, data to stdout,
timings to stderr in gray, non-zero exit on failure.

## the spine

verify never touches the original file. a polynomial commitment proves an
evaluation without the polynomial, so the command asymmetry is the design:

- `commit` reads the file — you hold the data
- `open` reads the file — the prover holds the polynomial
- `verify` reads only the commitment and the opening — the verifier does not
  hold the polynomial

every command below follows from keeping that boundary clean.

## invocation

```
lens <command> [--algo A] <args...> [flags]
```

`--algo` selects the construction and, with it, the field (see §5). default is
`brakedown`. a file is read as a sequence of field elements via the algebra's
encoding, then padded to the next power of two — that length is `2^ν`, giving
`ν` variables. a point is `ν` field elements.

## 1. commit

```
lens commit <file> [--algo A]
```

read `file`, encode as a [[MultilinearPoly]], print its 32-byte commitment as
lowercase hex to stdout. the commitment is a [[hemera]] hash — identical format
across all constructions.

```
$ lens commit data.bin
7a3f...c21b
```

`commit` reads real data only. synthetic polynomials for measurement live in
`bench` and `check`, never here — a commitment is to something.

## 2. open

```
lens open <file> <point...> [--algo A] [-o opening.lens]
```

produce a proof that `file`'s polynomial evaluates to a value at `point`. write
the opening artifact (§6) to `-o` path, or stdout hex if omitted. print the
claimed value to stdout.

```
$ lens open data.bin 0 1 0 1 -o entry.lens
value: 0000000000000009
```

## 3. verify

```
lens verify <commitment> <opening.lens>
```

check that the opening binds its claimed value at its point to `commitment`.
print `valid` or `invalid` to stdout; exit 0 on valid, 1 on invalid. the
original file is absent by design — the opening carries point and value, the
commitment is supplied independently (from a chain, a peer, a prior `commit`).

```
$ lens verify 7a3f...c21b entry.lens
valid
$ echo $?
0
```

`verify` reads `algo` from the opening artifact; no `--algo` flag is needed.

## 4. eval

```
lens eval <file> <point...> [--algo A]
```

evaluate the polynomial at `point` and print the value — the multilinear
extension, no proof. this is the ground-truth path: the value `open` claims and
`verify` accepts must equal what `eval` prints.

```
$ lens eval data.bin 0 1 0 1
0000000000000009
```

## 5. check

```
lens check <file> [--algo A] [--vars N]
```

run the full cycle on `file` — commit, open at a transcript-derived point,
verify — and print `PASS` or `FAIL` with per-stage timings. exit 0 on pass, 1
on fail. the fastest way to see a construction work end to end, and the source
of the pinned vectors (§9).

```
$ lens check data.bin
commit  7a3f...c21b   8.2ms
open    value 0000000000000009   31ms
verify  valid   2ms
PASS
```

`--vars N` commits a deterministic synthetic polynomial of `2^N` evaluations
instead of a file, for sizing without input data.

## 6. the opening artifact

the file written by `open` is self-describing, so `verify` stays minimal — it
carries everything except the commitment:

```
algo · num_vars · point[ν] · claimed_value · proof_bytes
```

the commitment is passed to `verify` separately because it is the object the
verifier independently knows and trusts. the artifact never contains the
polynomial or the file. its on-disk form is the [[Opening]] enum plus a small
header (`algo`, `num_vars`, `point`, `value`); the proof body is the
construction's `Opening` variant — `Tensor` for Brakedown / Ikat / Porphyry,
`Folding` for Binius.

## 7. construction and field

`--algo` fixes both the construction and the field, so point and value parsing
is algebra-aware:

| algo | construction | field | element hex | decimal |
|------|-------------|-------|-------------|---------|
| `brakedown` | Brakedown | Goldilocks (nebu) | 8 bytes | yes |
| `binius` | Binius | F₂¹²⁸ (kuro) | 16 bytes | no |
| `ikat` | Ikat | Goldilocks slots (jali) | 8 bytes | yes |
| `porphyry` | Porphyry | F_q (genies) | 64 bytes | no |

Goldilocks elements accept decimal or hex; wide-field elements are hex only.
non-canonical elements (≥ modulus) are rejected before any work, per
[[commitment]] §1.

## 8. assayer

Assayer does not implement the [[trait|Lens]] trait — the tropical semiring has
no commit/open/verify. it is witness-verify over an optimization problem, so it
takes its own two commands rather than pretending to fit the core shape:

```
lens assayer <problem.json> [-o witness.lens]
lens assayer-verify <commitment> <witness.lens>
```

`assayer` solves the tropical problem (shortest path, assignment, transport),
packs the witness and LP dual certificate as a Goldilocks polynomial, and
commits it via Brakedown (see [[tropical-semiring]]). `assayer-verify` checks
structural validity, cost correctness, and dual feasibility. the `problem.json`
schema is the tropical workload description — nodes, edges, weights, source,
target.

## 9. meta commands

```
lens bench   [--algo A] [--vars N]   commit / open / verify timings; compare constructions
lens vectors                          print pinned test vectors as JSON → vectors/lens.json
lens params  [--algo A]               field, expansion factor, security λ, proof size
lens help  ·  -h  ·  --help           usage
```

`bench` generates a deterministic synthetic polynomial (no input file) and
reports timings at the given size, defaulting to a sweep across `2^10`, `2^16`,
`2^20`. `vectors` emits the canonical anchors that
`cli/tests/vectors.rs` and every future implementation assert against.
`params` prints the construction's public parameters from [[commitment]] §9.

## 10. batch

```
lens open-batch   <file> <points-file> [--algo A] [-o opening.lens]
lens verify-batch <commitment> <opening.lens>
```

amortize `m` openings into one proof via the multilinear equality polynomial
([[commitment]] §3.5). `points-file` holds one point per line. the artifact and
verify semantics match §3 and §6 — one commitment, one opening, `m` claimed
values.

## 11. output conventions

- data to stdout: commitments and values as lowercase hex, one per line;
  `verify` prints `valid` / `invalid`, `check` prints per-stage lines and a
  final `PASS` / `FAIL`.
- timings to stderr in gray (`\x1b[90m…\x1b[0m`), formatted `[algo stage 2^ν  Nms]`,
  microseconds under a millisecond.
- errors to stderr, exit 1. verification failure is exit 1 with `invalid` on
  stdout, so both a human and a script read it.
- no GPU path in this release — CPU only. `--gpu` / `--cpu` are reserved to
  match the [[strata|algebra]] CLIs when the construction GPU backends land.

## 12. usage text

`lens help` prints the colored banner in the [[hemera]] style, then:

```
  lens commit <file>                    commit a file, print 32-byte root
  lens open   <file> <point...> [-o f]  open at a point, write proof
  lens verify <commitment> <proof>      verify an opening (no file)
  lens eval   <file> <point...>         evaluate, no proof (ground truth)
  lens check  <file>                    commit + open + verify, print PASS/FAIL
  lens assayer <problem.json> [-o f]    solve tropical problem, emit witness
  lens assayer-verify <commit> <proof>  verify tropical witness
  lens bench   [--vars N]               timings across constructions
  lens vectors                          print test vectors (JSON)
  lens params                           construction parameters

  flags:  --algo brakedown|binius|ikat|porphyry   (default: brakedown)
          --vars N                                 synthetic 2^N polynomial
          -o <file>                                write proof to file
```

## 13. crate structure

```
cli/
├── Cargo.toml     package lens-cli, [[bin]] name lens
└── src/
    └── main.rs    argument parsing, command dispatch, field-aware hex parsing
```

depends on `cyber-lens` (the facade) for all constructions, and on the algebra
crates (`nebu`, `kuro`, `trop`, `genies`) for field-element parsing. published
with the workspace but installed as a tool via `cargo install --path cli`, not
consumed as a library.

## status

implemented. all commands are wired in the `cli/` crate: `commit`, `open`,
`verify`, `eval`, `check` (§1–§5) over every `Lens` construction — Brakedown
(default) and Ikat over Goldilocks, Binius over F₂¹²⁸, Porphyry over F_q —
through a field-generic command path; `assayer`/`assayer-verify` (§8);
`open-batch` and batch verification (§10); `params` and `vectors` (§9). no GPU
backend yet — CPU only.
