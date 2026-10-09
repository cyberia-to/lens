---
tags: lens, audit, pcs, whir
crystal-type: audit
crystal-domain: crypto
---
# RS / WHIR PCS measurements — 2026-10-09

Machine: Apple M4 Max, 16 cores, 48 GiB, macOS 26.4.1. rustc 1.95.0
(Homebrew), default `release` profile (no LTO). Stand: lens `feat/rs-whir-pcs`
on f214ae0, strata 56aedb2, hemera 23f3bbc.

Command: `cargo run --release -p cyber-lens-rspcs --example measure -- 5`.
Median of 5 runs (fixed seeds) for every time and for proof bytes. Prove =
commit + open. Default params otherwise: TensorRs `pow_bits 16`, shape
automatic; WHIR `k = 4`, `pow_bits 16`, Johnson, `max_final_vars 8`; target
128. "security bits" = `MultilinearPcs::security_bits` (proven
round-by-round soundness, grinding included). "(all)": every
column/leaf opened.

hemera hash_node (one permutation): 4.433 us (median of 5 × 100k)

| scheme | num_vars | rate | shape / schedule | proof bytes | prove ms (commit + open) | verify ms | security bits | 128-bit proven |
|---|---|---|---|---|---|---|---|---|
| TensorRs | 10 | 1/2 | k1=2^10 k2=2^0 N=2^1 t=2 (all) | 16462 | 2.07 (1.97 + 0.10) | 1.415 | 181.00 | yes |
| WHIR | 10 | 1/2 | k=4, 1 rounds [t=128(all)], final 2^6 | 18180 | 3.30 (2.92 + 0.38) | 3.391 | 131.34 | yes |
| TensorRs | 10 | 1/4 | k1=2^3 k2=2^7 N=2^9 t=166 | 21510 | 37.35 (9.36 + 27.99) | 4.590 | 128.56 | yes |
| WHIR | 10 | 1/4 | k=4, 1 rounds [t=114], final 2^6 | 16812 | 58.12 (5.95 + 52.18) | 3.013 | 128.01 | yes |
| TensorRs | 10 | 1/16 | k1=2^3 k2=2^7 N=2^11 t=123 | 27078 | 36.27 (8.45 + 27.83) | 4.987 | 128.24 | yes |
| WHIR | 10 | 1/16 | k=4, 1 rounds [t=57], final 2^6 | 15116 | 55.81 (6.93 + 48.88) | 2.509 | 128.07 | yes |
| TensorRs | 12 | 1/2 | k1=2^4 k2=2^8 N=2^9 t=270 | 45958 | 40.56 (11.86 + 28.70) | 8.332 | 128.06 | yes |
| WHIR | 12 | 1/2 | k=4, 1 rounds [t=230], final 2^8 | 36972 | 38.55 (11.53 + 27.02) | 6.858 | 128.05 | yes |
| TensorRs | 12 | 1/4 | k1=2^4 k2=2^8 N=2^10 t=166 | 41958 | 37.33 (6.95 + 30.38) | 6.891 | 128.56 | yes |
| WHIR | 12 | 1/4 | k=4, 1 rounds [t=114], final 2^8 | 29132 | 35.81 (6.93 + 28.88) | 4.893 | 128.01 | yes |
| TensorRs | 12 | 1/16 | k1=2^4 k2=2^8 N=2^12 t=123 | 44902 | 66.33 (14.90 + 51.44) | 6.666 | 128.24 | yes |
| WHIR | 12 | 1/16 | k=4, 1 rounds [t=57], final 2^8 | 23212 | 44.37 (15.55 + 28.81) | 3.393 | 128.07 | yes |
| TensorRs | 16 | 1/2 | k1=2^5 k2=2^11 N=2^12 t=270 | 192230 | 76.22 (19.25 + 56.97) | 33.367 | 128.06 | yes |
| WHIR | 16 | 1/2 | k=4, 2 rounds [t=230 t=57], final 2^8 | 98920 | 188.10 (27.16 + 160.94) | 15.541 | 128.05 | yes |
| TensorRs | 16 | 1/4 | k1=2^6 k2=2^10 N=2^12 t=166 | 153574 | 62.84 (29.25 + 33.59) | 17.741 | 128.56 | yes |
| WHIR | 16 | 1/4 | k=4, 2 rounds [t=114 t=46], final 2^8 | 71496 | 234.15 (42.37 + 191.79) | 10.581 | 128.01 | yes |
| TensorRs | 16 | 1/16 | k1=2^6 k2=2^10 N=2^14 t=123 | 136742 | 114.37 (80.92 + 33.46) | 17.414 | 128.24 | yes |
| WHIR | 16 | 1/16 | k=4, 2 rounds [t=57 t=33], final 2^8 | 53288 | 455.02 (135.86 + 319.16) | 7.548 | 128.07 | yes |
| TensorRs | 20 | 1/2 | k1=2^7 k2=2^13 N=2^14 t=270 | 711718 | 213.98 (128.13 + 85.85) | 117.130 | 128.06 | yes |
| WHIR | 20 | 1/2 | k=4, 3 rounds [t=231 t=57 t=33], final 2^8 | 159012 | 1529.16 (291.98 + 1237.18) | 23.711 | 128.05 | yes |
| TensorRs | 20 | 1/4 | k1=2^8 k2=2^12 N=2^14 t=166 | 565350 | 273.77 (223.03 + 50.74) | 66.131 | 128.56 | yes |
| WHIR | 20 | 1/4 | k=4, 3 rounds [t=115 t=46 t=29], final 2^8 | 113988 | 1696.15 (488.91 + 1207.24) | 16.232 | 128.01 | yes |
| TensorRs | 20 | 1/16 | k1=2^8 k2=2^12 N=2^16 t=123 | 480838 | 908.88 (859.01 + 49.87) | 53.480 | 128.24 | yes |
| WHIR | 20 | 1/16 | k=4, 3 rounds [t=58 t=33 t=23], final 2^8 | 83716 | 4899.25 (1828.20 + 3071.05) | 11.625 | 128.22 | yes |

Configurations with security bits ≥ 128: all 24 rows.

## tests

`cargo test --release` in the lens workspace: all green; `cyber-lens-rspcs`
28 tests (+2 serde round-trips with `--features serde`), `cyber-lens-core` 10.

Bit-flip scans (every bit of the serialized proof flipped; each must fail to
parse or be rejected), all clean:

| scheme | num_vars | params | proof bytes | flips rejected |
|---|---|---|---|---|
| TensorRs | 6 | r=2, pow 6 (all columns) | 2126 | 17008 |
| TensorRs | 8 | r=2, pow 4, row_vars 1 (sampled) | 15174 | 121392 |
| WHIR | 5 | defaults | 4404 | 35232 |
| WHIR | 6 | r=2, k=2, pow 4, final ≤1 (3 commitments, saturated queries) | 7204 | 57632 |
| WHIR | 8 | r=3, k=2, pow 4, final ≤1 (4 commitments, sampled, nonces) | 25720 | 205760 |

The first run of the scan found accepted flips in grinding nonces when the
query count exceeded the number of positions (a different valid nonce
re-sampled the same full index set). Fixed by opening every position without
sampling or grinding in that case.
