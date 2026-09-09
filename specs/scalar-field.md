---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: expander lens, Brakedown, tensor-Merkle Brakedown, linear-code lens, expander-pcs
---
# scalar field lens (Brakedown)

the [[Goldilocks field|Goldilocks]] polynomial commitment. commits via a
tensor-matrix Merkle tree over expander-graph linear codes
(Ligero/Brakedown-style — Golovnev, Lee, Setty, Thaler, Wahby, "Brakedown:
Linear-time and post-quantum SNARKs for R1CS", §5). one Merkle root binds
the whole matrix; opening reveals a small row-combination plus a handful
of authenticated columns.

implements the [[trait|Lens]] trait for $\mathbb{F}_p$. part of the
five-lens architecture — see [[commitment]] for the full picture.

replaces an earlier per-round "commit fresh codeword, then reduce" scheme
that had no soundness argument at all: it never checked the queried
codeword values against the commitment (a flat hash of a whole codeword
cannot authenticate one symbol), and consecutive rounds used unrelated
expander graphs (`Expander::new(n)` builds a fresh graph per length), so
there was no linear relationship between rounds for a spot-check to
verify even if the values were checked. see
github.com/cyberia-to/lens/issues/6.

## the matrix

reshape the evaluation table (length $N = 2^\nu$) into a $k_1 \times k_2$
matrix $U$, $k_1 = 2^{\lceil \nu/2 \rceil} \geq k_2 = 2^{\lfloor \nu/2
\rfloor}$, powers of two, $k_1 k_2 = N$ — as close to $\sqrt N \times
\sqrt N$ as a power-of-two split allows. index convention (bit $j$ of an
index is variable $j$, matching `MultilinearPoly::evaluate` and
`tensor::evaluate_small`):

```
row(i) = i & (k1 - 1)     — low log2(k1) bits of i
col(i) = i >> log2(k1)    — remaining bits of i
U[row][col] = evals[col*k1 + row]
```

so a point $r$ splits as $r_\text{row} = r[..\log_2 k_1]$, $r_\text{col} =
r[\log_2 k_1..]$.

## commit

every row of $U$ is encoded by the SAME linear code (the Margulis
expander over $\mathbb{Z}_p \times \mathbb{Z}_p$, `expander.rs`) into an
$m$-column matrix $\hat U$, $m = \text{EXPANSION} \cdot k_2$
(EXPANSION = 2). because every row has the same length $k_2$, one
`Expander` instance is built and reused for all $k_1$ rows — the fix for
the old "fresh expander per round" problem, which meant no two committed
codewords were ever linearly related.

$\hat U$ is committed as a Merkle tree over its **columns**: each column
($k_1$ field elements) is one leaf, hashed with hemera's `hash_leaf`
convention (`brakedown::merkle`, a purpose-built tree — hemera's own
`tree.rs` chunks raw bytes into fixed 4096-byte windows, which does not
line up with "one leaf per column" for arbitrary $k_1$). the tree root
**is** the `Commitment` — there is exactly one hash the prover cannot
equivocate on, unlike the old per-round flat hashes.

## open

to prove $f(r) = y$, split $r = (r_\text{row}, r_\text{col})$:

```
OPEN(U, r):
  weights = eq_table(r_row)                          k1 elements
  v = Σ_row weights[row] · U[row, :]                  k2 elements — sent in the clear
  transcript.absorb(commitment, v)
  for t in 1..NUM_QUERIES:
    j = transcript.squeeze() mod m
    reveal column j of Û (k1 elements) + its Merkle path

  proof = (v, [(j, Û[:,j], path_j) for t queries])
```

## verify

```
VERIFY(root, r, y, proof):
  transcript.absorb(root, v)
  encoded_v = encode(v)                               same expander as the rows
  weights = eq_table(r_row)
  for each queried (j, column, path):
    j == transcript.squeeze() mod m                   ELSE reject
    merkle_verify_path(root, hash_leaf(column, j), path)   ELSE reject
    encoded_v[j] == Σ_row weights[row] · column[row]  ELSE reject   ← the check the old scheme never made
  evaluate_small(v, r_col) == y                        ELSE reject
```

the consistency check ties the revealed columns to $v$: because `encode`
is linear and every row uses the identical map, an honest matrix
satisfies `encode(v)[j] == Σ_row weights[row]·Û[row,j]` for every column
$j$ (linearity: $encode(\sum w_i U_i) = \sum w_i\, encode(U_i)$). a
dishonest `v` — or a `v` that does not actually match the committed
rows — produces a nonzero "error" codeword; because $r$ (hence
$r_\text{row}$, hence $v$'s definition) is only known to the prover
*after* the matrix is committed via Fiat-Shamir, the prover cannot
tailor $\hat U$ to survive a not-yet-known challenge. each of the
`NUM_QUERIES` uniformly random columns then independently has a chance
of exposing the mismatch bounded by the code's distance at that column
count — see "query count" below.

## query count

`NUM_QUERIES` is **not a fixed constant** — it is a function of $k_2$
(`brakedown::num_queries`), because the expander code's distance is not
$k_2$-independent for this implementation. `expander::tests::
empirical_distance_probe` measures it directly: a minimal two-input
attack (pick two rows, one free scaling ratio, cancel one output
position they share) drives the achievable output weight down to
roughly 9–10 nonzero symbols out of $m = 2 k_2$, for $k_2$ ranging over
64..1024 — i.e. the **absolute** weight an adversary can force stays
roughly constant while $m$ grows, so relative distance (and per-query
soundness) shrinks as $1/k_2$. this is a real limitation of a
single-layer Margulis expander code (degree 36, two composition steps):
it is not the recursive, distance-amplifying code the Brakedown paper
constructs specifically to get a $k_2$-independent relative distance.

`num_queries(k2)` derives a query count from a target soundness (100
bits) and an assumed worst-case absolute weight of 4 (below the ~9–10
measured floor, since the probe only searched 1- and 2-input attacks):

```
m = EXPANSION * k2
relative_distance = min(4, m-1) / m
queries = ceil(100 / -log2(1 - relative_distance))
```

for $k_2 = 8$ (zheng's decider commits a witness of length 128, giving
$k_1=16, k_2=8$) this is a modest number of queries. for large future
circuits ($k_2$ in the thousands) the formula honestly reports that
query count — and proof size — grows with the circuit; a fixed small
`NUM_QUERIES` would silently under-deliver on soundness at that scale.
hardening `expander.rs` (a longer composition walk, or the paper's full
recursive code) is tracked as follow-up work, not done here.

## batch opening

unchanged in shape from before: `batch_open` squeezes a single random
point $r^*$ over all $\nu$ variables and opens the polynomial there;
`batch_verify` extracts the prover's own claimed evaluation from the
opening and checks it via `verify`. this does not independently recombine
the caller's `(r_i, y_i)` claims against $\alpha$-weighted sums — a
pre-existing gap, not introduced by the tensor-Merkle rewrite, and out of
scope for github.com/cyberia-to/lens/issues/6.

## Goldilocks compatibility

requires:
1. fast field arithmetic — Goldilocks: 4-5 cycle multiply ([[nebu]])
2. an expander graph family for the per-row code — Margulis here
3. linear-time encoding — sparse matrix multiply per row

see [[commitment]] for the shared interface, [[binary-tower]] for Binius,
[[isogeny-curves]] for Porphyry (Porphyry has not migrated to this
construction yet — it still uses the old, unsound per-round scheme; see
`Opening::Tensor`'s doc comment in cyber-lens-core).
