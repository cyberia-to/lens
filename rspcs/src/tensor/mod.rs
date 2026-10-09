//! TensorRs — Ligero-geometry multilinear PCS with Reed–Solomon rows.
//!
//! The evaluation table is a `k1 × k2` matrix `U` (row `i` = entries
//! `i·k2 .. (i+1)·k2`, so rows are indexed by the high variables and
//! columns by the low ones). Each row is RS-encoded to `N = k2·2^r`
//! symbols; a hemera Merkle tree is built over the `N` columns of the
//! encoded matrix `Û`. Opening at `z = (z_lo, z_hi)` sends
//! `u = Σ α^i U_i` (proximity test, Ligero testing phase) and
//! `y = Σ eq(z_hi, i) U_i` (evaluation phase), then opens `t` random
//! columns shared by both checks. See `specs/tensor-rs.md`.
//!
//! Soundness, unique-decoding regime (`e = ⌊(N − k2)/2⌋`):
//! - α round: `(k1 − 1)·N/|K|` — correlated agreement of RS codes for the
//!   powers generator (BCIKS20 as restated in WHIR eprint 2024/1586,
//!   Theorem 4.8, first case).
//! - column round: `((1 + ρ)/2)^t` — per opened column, a δ-far
//!   combination (`δ = e/N`) passes with probability `≤ 1 − (e+1)/N`, and
//!   for a jointly δ-close matrix a wrong `y` passes only on the
//!   `≤ (k2 − 1) + e` columns where the codewords agree or the matrix is
//!   corrupted (the binding argument of Brakedown, eprint 2021/1043, §4,
//!   with `γ/3` replaced by the unique-decoding radius). Both are
//!   `≤ (N + k2)/(2N) = (1 + ρ)/2`.

mod proof;
mod prove;
mod verify;

pub use proof::TensorRsProof;
pub use prove::TensorRsData;

use cyber_lens_core::{Commitment, MultilinearPcs, PcsError, Transcript};
use nebu::{Fp3, Goldilocks};

use crate::soundness::ext_field_bits;

/// Security parameters for TensorRs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TensorRsParams {
    /// `ρ = 2^-log_inv_rate`, at least 1.
    pub log_inv_rate: u8,
    /// Target soundness in bits; sets the column count.
    pub security_target: u16,
    /// Grinding bits before column sampling.
    pub pow_bits: u8,
    /// Row variables (`k1 = 2^row_vars`); `None` picks the size-minimal
    /// shape deterministically.
    pub row_vars: Option<u8>,
}

impl Default for TensorRsParams {
    fn default() -> Self {
        Self {
            log_inv_rate: 2,
            security_target: 128,
            pow_bits: 16,
            row_vars: None,
        }
    }
}

const VERSION: u8 = 1;
const AUTO: u8 = 0xFF;
/// Largest supported number of variables.
pub const MAX_VARS: usize = 30;

impl TensorRsParams {
    /// 6-byte header: version, log_inv_rate, pow_bits, target (u16 LE),
    /// row_vars (0xFF = automatic).
    pub fn header(&self) -> [u8; 6] {
        let t = self.security_target.to_le_bytes();
        [
            VERSION,
            self.log_inv_rate,
            self.pow_bits,
            t[0],
            t[1],
            self.row_vars.unwrap_or(AUTO),
        ]
    }

    pub fn from_header(h: [u8; 6]) -> Result<Self, PcsError> {
        if h[0] != VERSION {
            return Err(PcsError::Malformed);
        }
        Ok(Self {
            log_inv_rate: h[1],
            pow_bits: h[2],
            security_target: u16::from_le_bytes([h[3], h[4]]),
            row_vars: (h[5] != AUTO).then_some(h[5]),
        })
    }
}

/// Shape and query count derived from params at one size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TensorConfig {
    pub num_vars: usize,
    pub row_vars: usize,
    pub col_vars: usize,
    /// `N = 2^log_n` encoded columns.
    pub log_n: u32,
    pub queries: usize,
    pub pow_bits: u32,
}

impl TensorConfig {
    pub fn k1(&self) -> usize {
        1 << self.row_vars
    }
    pub fn k2(&self) -> usize {
        1 << self.col_vars
    }

    /// `−log2` of the per-column acceptance bound `(1 + ρ)/2`.
    fn query_bits(r: u32) -> f64 {
        -((1.0 + (-(r as f64)).exp2()) / 2.0).log2()
    }

    /// Bits of the α (correlated agreement) round.
    pub fn alpha_bits(&self) -> f64 {
        if self.row_vars == 0 {
            return f64::INFINITY;
        }
        let k1m1 = (self.k1() - 1) as f64;
        ext_field_bits() - k1m1.log2() - self.log_n as f64
    }

    /// Every column is opened (no sampling).
    pub fn opens_all(&self) -> bool {
        self.queries == 1 << self.log_n
    }

    /// Bits of the column round, grinding included. Opening every column
    /// checks both linear relations everywhere: error 0.
    pub fn column_bits(&self, r: u32) -> f64 {
        if self.opens_all() {
            return f64::INFINITY;
        }
        self.queries as f64 * Self::query_bits(r) + self.pow_bits as f64
    }

    /// Opened column indices: all of them, or `queries` transcript samples.
    pub(crate) fn sample(&self, t: &mut Transcript) -> Vec<usize> {
        if self.opens_all() {
            (0..1 << self.log_n).collect()
        } else {
            crate::merkle::dedup_sorted(t.squeeze_indices(self.queries, self.log_n))
        }
    }

    fn with_rows(params: &TensorRsParams, m: usize, row_vars: usize) -> Option<Self> {
        let r = u32::from(params.log_inv_rate);
        let col_vars = m - row_vars;
        let log_n = col_vars as u32 + r;
        if log_n > 32 {
            return None;
        }
        let need = (f64::from(params.security_target) - f64::from(params.pow_bits)).max(0.0);
        let queries = ((need / Self::query_bits(r)).ceil() as usize).max(1);
        let n = 1usize << log_n;
        // saturated sampling: open every column, no grinding, no error
        let (queries, pow_bits) = if queries >= n {
            (n, 0)
        } else {
            (queries, u32::from(params.pow_bits))
        };
        Some(Self {
            num_vars: m,
            row_vars,
            col_vars,
            log_n,
            queries,
            pow_bits,
        })
    }

    /// Estimated proof bytes (selection heuristic only).
    fn estimate(&self) -> f64 {
        let n = (1u64 << self.log_n) as f64;
        let t = (self.queries as f64).min(n);
        let path = (self.log_n as f64 - t.log2()).max(0.0);
        48.0 * self.k2() as f64 + t * 8.0 * self.k1() as f64 + t * path * 32.0
    }

    pub fn derive(params: &TensorRsParams, m: usize) -> Result<Self, PcsError> {
        if params.log_inv_rate == 0 || m > MAX_VARS {
            return Err(PcsError::Shape);
        }
        match params.row_vars {
            Some(rv) => {
                let rv = usize::from(rv);
                if rv > m {
                    return Err(PcsError::Shape);
                }
                Self::with_rows(params, m, rv).ok_or(PcsError::Shape)
            }
            None => (0..=m)
                .filter_map(|rv| Self::with_rows(params, m, rv))
                .min_by(|a, b| a.estimate().total_cmp(&b.estimate()))
                .ok_or(PcsError::Shape),
        }
    }
}

/// Ligero-geometry PCS over Goldilocks with RS rows.
#[derive(Clone, Copy, Debug)]
pub struct TensorRs;

impl MultilinearPcs for TensorRs {
    const NAME: &'static str = "lens-tensor-rs-v1";
    type Params = TensorRsParams;
    type ProverData = TensorRsData;
    type Proof = TensorRsProof;

    fn commit(params: &Self::Params, evals: &[Goldilocks]) -> (Commitment, Self::ProverData) {
        prove::commit(params, evals)
    }

    fn open(
        params: &Self::Params,
        data: &Self::ProverData,
        point: &[Fp3],
        transcript: &mut Transcript,
    ) -> (Fp3, Self::Proof) {
        prove::open(params, data, point, transcript)
    }

    fn verify(
        params: &Self::Params,
        commitment: &Commitment,
        num_vars: usize,
        point: &[Fp3],
        value: Fp3,
        proof: &Self::Proof,
        transcript: &mut Transcript,
    ) -> Result<(), PcsError> {
        verify::verify(
            params, commitment, num_vars, point, value, proof, transcript,
        )
    }

    fn security_bits(params: &Self::Params, num_vars: usize) -> f64 {
        match TensorConfig::derive(params, num_vars) {
            Ok(c) => c
                .alpha_bits()
                .min(c.column_bits(u32::from(params.log_inv_rate))),
            Err(_) => 0.0,
        }
    }
}

/// Transcript prologue shared by prover and verifier.
pub(crate) fn prologue(
    t: &mut Transcript,
    params: &TensorRsParams,
    m: usize,
    root: &Commitment,
    point: &[Fp3],
    value: Fp3,
) {
    t.absorb(TensorRs::NAME.as_bytes());
    t.absorb(&params.header());
    t.absorb_u64(m as u64);
    t.absorb(root.as_bytes());
    t.absorb_fp3_slice(point);
    t.absorb_fp3(value);
}
