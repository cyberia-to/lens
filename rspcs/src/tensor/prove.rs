//! TensorRs prover: commit and open.

use cyber_lens_core::{Commitment, Transcript};
use nebu::{Fp3, Goldilocks};

use super::{TensorConfig, TensorRsParams, TensorRsProof, prologue};
use crate::field::{eq_table, ml_eval_base, mul_base};
use crate::merkle::Tree;
use crate::par::{par_map, par_map_min};
use crate::pow;
use crate::rs::encode_base_with;

/// Prover state: the table, its encoded rows and the column tree.
pub struct TensorRsData {
    pub(crate) config: TensorConfig,
    pub(crate) evals: Vec<Goldilocks>,
    /// `k1` encoded rows of `N` symbols.
    pub(crate) encoded: Vec<Vec<Goldilocks>>,
    pub(crate) tree: Tree,
}

impl core::fmt::Debug for TensorRsData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TensorRsData")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

pub fn commit(params: &TensorRsParams, evals: &[Goldilocks]) -> (Commitment, TensorRsData) {
    assert!(
        evals.len().is_power_of_two(),
        "evals.len() must be 2^num_vars"
    );
    let m = evals.len().trailing_zeros() as usize;
    let config = TensorConfig::derive(params, m).expect("unsupported TensorRs shape");
    let k2 = config.k2();
    let encoded: Vec<Vec<Goldilocks>> = par_map_min(config.k1(), 2, |i| {
        encode_base_with(&evals[i * k2..(i + 1) * k2], config.log_n, false)
    });
    let n = 1usize << config.log_n;
    let tree = Tree::build(n, |j| column_bytes(&encoded, j));
    let root = Commitment(tree.root());
    let data = TensorRsData {
        config,
        evals: evals.to_vec(),
        encoded,
        tree,
    };
    (root, data)
}

pub(crate) fn column_bytes(encoded: &[Vec<Goldilocks>], j: usize) -> Vec<u8> {
    let mut b = Vec::with_capacity(encoded.len() * 8);
    for row in encoded {
        b.extend_from_slice(&row[j].as_u64().to_le_bytes());
    }
    b
}

/// `Σ_i coeffs_i · U_i` over the rows of the evaluation table.
fn combine_rows(evals: &[Goldilocks], k2: usize, coeffs: &[Fp3]) -> Vec<Fp3> {
    par_map(k2, |j| {
        let mut acc = Fp3::ZERO;
        for (i, &c) in coeffs.iter().enumerate() {
            acc += mul_base(c, evals[i * k2 + j]);
        }
        acc
    })
}

pub(crate) fn powers(alpha: Fp3, n: usize) -> Vec<Fp3> {
    let mut out = Vec::with_capacity(n);
    let mut cur = Fp3::ONE;
    for _ in 0..n {
        out.push(cur);
        cur *= alpha;
    }
    out
}

pub fn open(
    params: &TensorRsParams,
    data: &TensorRsData,
    point: &[Fp3],
    transcript: &mut Transcript,
) -> (Fp3, TensorRsProof) {
    let c = data.config;
    assert_eq!(point.len(), c.num_vars, "point dimension");
    assert_eq!(
        TensorConfig::derive(params, c.num_vars).ok(),
        Some(c),
        "open params differ from commit params"
    );
    let value = ml_eval_base(&data.evals, point);
    prologue(
        transcript,
        params,
        c.num_vars,
        &Commitment(data.tree.root()),
        point,
        value,
    );

    let alpha = transcript.squeeze_fp3();
    let row_test = combine_rows(&data.evals, c.k2(), &powers(alpha, c.k1()));
    let row_eval = combine_rows(&data.evals, c.k2(), &eq_table(&point[c.col_vars..]));
    transcript.absorb_fp3_slice(&row_test);
    transcript.absorb_fp3_slice(&row_eval);

    let pow_nonce = pow::grind(transcript, c.pow_bits);
    let idx = c.sample(transcript);
    let mut columns = Vec::with_capacity(idx.len() * c.k1());
    for &j in &idx {
        columns.extend(data.encoded.iter().map(|row| row[j]));
    }
    let siblings = data.tree.open(&idx);
    let proof = TensorRsProof {
        params: *params,
        row_test,
        row_eval,
        pow_nonce,
        height: c.k1(),
        columns,
        siblings,
    };
    (value, proof)
}
