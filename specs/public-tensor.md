# Public tensor commitment

`PublicTensor` authenticates complete public multilinear evaluation tables.
It reveals every witness element, uses linear verification and makes no
zero-knowledge, sublinear-verification or expander-distance claim.

For `N=2^v` elements, split variables into `r=ceil(v/2)` row bits and `c=floor(v/2)`
column bits. Column `j` contains table positions `j*2^r .. (j+1)*2^r` as canonical
little-endian Goldilocks elements. Commit to all `2^c` raw columns with the
existing indexed, domain-separated column Merkle tree. Wrap that root in a
hash of `lens-public-tensor-v1`, `v` encoded as u64, and the root.

An opening contains every column exactly once in increasing index order, its
Merkle path, and the row-weighted column values for the requested point. The
verifier requires exact byte lengths, canonical fields, exact path lengths,
path directions matching each index bit, and authentication to the same root.
It checks every row-weighted column value and its multilinear evaluation at the
remaining point coordinates. The opening can use the existing `TensorMerkle`
container; the chosen backend and commitment domain determine its semantics.

Every point and value and the commitment is transcript-bound under this
versioned domain. Batch verification authenticates the table once and checks
each provided point/value separately against that same table. Empty batches
are rejected. Dimensions are limited to 20 variables before allocating.

Binding follows from the Merkle hash assumption and exact evaluation; no
sampling, code distance or proximity-testing assumption is needed. This does
not alter the Goldilocks algebraic soundness bound of an enclosing sumcheck.
