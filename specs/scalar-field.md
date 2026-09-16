---
tags: cyber, computer science, cryptography
crystal-type: entity
crystal-domain: computer science
alias: expander lens, Brakedown, tensor-Merkle Brakedown, linear-code lens, expander-pcs
---
# scalar field lens (Brakedown)

## commitment version 2

`COMMITMENT_VERSION = 2`. This version changes every Brakedown raw and tensor
commitment. Version 1 roots and openings are not accepted as version 2. Consumers
(nox, Zheng, BBG and stored CLI artifacts) must recompute commitments and proofs;
there is no fallback to the old interpretation. `PublicTensor` remains a separate
backend with its existing `lens-public-tensor-v1` domain and is unaffected.

For n canonical Goldilocks elements x, define the systematic linear encoding

```
E(x) = x || A_n(x)                         length 3n
```

A_n is the existing sparse adjacency sum into 2n parity positions: choose the
smallest prime p with p² >= 2n; map input i to (floor(i/p), i mod p); compose the
six existing affine transformations twice; reduce each resulting coordinate
index modulo 2n; add x[i] for every edge, including repeated edges. Parity is
placed after the n raw positions. For n=0, the encoding is empty.

The raw prefix proves injectivity: E(x)=E(y) implies x=y. Every nonzero encoded
vector has at least one nonzero symbol. No stronger distance bound is assumed.
Spectral statements about the unmodified Margulis graph do not by themselves
prove distance for this finite, reduced sparse map.

`commit_raw(x)` hashes, with Hemera, the concatenation of ASCII
`lens-brakedown-raw-v2`, n as u64 little endian, and canonical little-endian u64
symbols of E(x). It is a complete-table binding commitment under the hash
assumption. It offers no partial opening and is distinct from the tensor root.

## tensor layout

For N=2^v, let row_vars=ceil(v/2), col_vars=floor(v/2), k1=2^row_vars,
k2=2^col_vars. Set U[row][col]=evals[col*k1+row]. Encode each row with E.
There are m=3*k2 encoded columns, each containing k1 canonical field elements.
The first k2 columns are exactly the original evaluation table columns.

Build an indexed column Merkle tree. Pad its leaves to the next power of two
with empty byte strings, hashed at their own indices; padding leaves are never
queried as data. Path depth is log2(next_power_of_two(m)); path sides must match
index bits. The tensor commitment hashes ASCII `lens-brakedown-tensor-v2`, v as
u64 little endian, and the tree root. Dimensions are checked before allocation
and limited to 20 variables. This also separates tensor roots from raw roots.

## single-point opening

Opening protocol version3 uses domain `lens-brakedown-opening-v3`. It preserves
the version2 commitment roots and sampling count but encodes each challenged
column once, in strictly increasing index order. Prover and verifier draw every
challenge, including repeats; the wire contains exactly the resulting index set.
Missing, extra, duplicate or reordered columns fail. Each unique column's full
bytes/path and weighted equation are authenticated once. Existing dense openings
must be regenerated. This removes repeated wire payloads and hash work without
dropping any sampled constraint or changing transcript advancement.

For point r=(r_row,r_col), reveal v=Σ_row eq(r_row,row)*U[row,:]. Bind the version
domain, commitment, point dimension, canonical point, claimed value, and v into
the transcript. Draw independently t column indices modulo m and authenticate
each column. For every index j require E(v)[j] to equal the weighted sum of that
column. Finally require the multilinear evaluation of v at r_col to equal the
claimed value. Field encodings, sizes, path depths and index-side relations must
be canonical; malformed input is rejected.

The sampling count is 100*m, computed with integers: (1-1/m)^m <= e^-1 < 1/2.
It uses only the proven minimum absolute weight of one for a nonzero valid
codeword. This is a conditional
sampling budget, not a claim of 100-bit security for the complete PCS: validity
of a maliciously committed encoded matrix is not independently proved, and
Goldilocks algebraic/transcript bounds remain separate. This implementation is
not the full recursive Brakedown construction and is not zero knowledge.

Consumers needing exact public-table authentication should use `PublicTensor`,
or reveal the complete raw table and recompute the versioned commitment. BBG's
full-table certificate uses this latter route; it must not treat sampled
acceptance as exact table authentication.

## batches

One-point batch operations use the single-point protocol. Empty or multi-point
batches are unsupported: verification returns false, and proving rejects the
request. The former random-point implementation failed to authenticate the
caller's multiple claimed values and is removed. `PublicTensor` supports exact
public batches when full table disclosure is acceptable.

## synchronized components

Changes to this contract propagate together through `expander.rs`, Brakedown
raw/tensor commitment code, the column tree, verifier dimensions/transcript and
consumer roots/proofs. The public `Lens` trait and `TensorMerkle` container stay
unchanged; the selected backend and versioned root determine their semantics.
