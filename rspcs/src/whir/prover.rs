//! WHIR prover.

use cyber_lens_core::{Commitment, Transcript};
use nebu::{Fp3, Goldilocks};

use super::config::RoundSpec;
use super::fold::{coset_tree, open as open_leaves, shift_point};
use super::proof::{Opening, WhirProof, WhirRoundProof};
use super::{WhirConfig, WhirParams, prologue, sumcheck};
use crate::field::{
    add_scaled_eq, eq_table, fold_coeffs_base, fold_coeffs_ext, ml_eval_base, ml_eval_ext,
    mobius_base, mobius_ext, pow_point, univariate_ext,
};
use crate::merkle::Tree;
use crate::pow;
use crate::rs::{encode_base, encode_ext};
use crate::wire::Symbols;

/// The committed message `f_0`: its table and monomial coefficients, over
/// Goldilocks or over Fp3.
#[derive(Clone)]
pub(crate) enum Message {
    Base {
        evals: Vec<Goldilocks>,
        coeffs: Vec<Goldilocks>,
    },
    Ext {
        evals: Vec<Fp3>,
        coeffs: Vec<Fp3>,
    },
}

/// Prover state: table, monomial coefficients, codeword and tree of `f_0`.
pub struct WhirData {
    pub(crate) config: WhirConfig,
    pub(crate) message: Message,
    pub(crate) codeword: Symbols,
    pub(crate) tree: Tree,
    /// The masks of a zero-knowledge commitment (`whir::zk`).
    pub(crate) zk: Option<super::zk::ZkRound0>,
}

impl core::fmt::Debug for WhirData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WhirData")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

pub fn commit(params: &WhirParams, evals: &[Goldilocks]) -> (Commitment, WhirData) {
    assert!(
        evals.len().is_power_of_two(),
        "evals.len() must be 2^num_vars"
    );
    let m = evals.len().trailing_zeros() as usize;
    let config = WhirConfig::derive(params, m).expect("unsupported WHIR shape");
    let spec = config.rounds[0];
    let mut coeffs = evals.to_vec();
    mobius_base(&mut coeffs);
    let codeword = Symbols::Base(encode_base(&coeffs, spec.log_domain));
    let tree = coset_tree(&codeword, spec.log_domain, spec.fold);
    let root = Commitment(tree.root());
    let message = Message::Base {
        evals: evals.to_vec(),
        coeffs,
    };
    (root, WhirData { config, message, codeword, tree, zk: None })
}

/// Commit an Fp3-valued table: the code is `RS[K, L, 2^m]` with the same
/// domain and leaf layout as a base commitment; leaves carry `3×u64`
/// symbols.
pub fn commit_ext(params: &WhirParams, evals: &[Fp3]) -> (Commitment, WhirData) {
    assert!(
        evals.len().is_power_of_two(),
        "evals.len() must be 2^num_vars"
    );
    let m = evals.len().trailing_zeros() as usize;
    let config = WhirConfig::derive(params, m).expect("unsupported WHIR shape");
    let spec = config.rounds[0];
    let mut coeffs = evals.to_vec();
    mobius_ext(&mut coeffs);
    let codeword = Symbols::Ext(encode_ext(&coeffs, spec.log_domain));
    let tree = coset_tree(&codeword, spec.log_domain, spec.fold);
    let root = Commitment(tree.root());
    let message = Message::Ext {
        evals: evals.to_vec(),
        coeffs,
    };
    (root, WhirData { config, message, codeword, tree, zk: None })
}

/// Monomial coefficients of the current folded polynomial.
pub(crate) enum Coeffs {
    Base(Vec<Goldilocks>),
    Ext(Vec<Fp3>),
}

impl Coeffs {
    pub(crate) fn fold(&mut self, alpha: Fp3) {
        match self {
            Coeffs::Base(c) => *self = Coeffs::Ext(fold_coeffs_base(c, alpha)),
            Coeffs::Ext(c) => fold_coeffs_ext(c, alpha),
        }
    }
    pub(crate) fn ext(&self) -> &[Fp3] {
        match self {
            Coeffs::Ext(c) => c,
            Coeffs::Base(_) => unreachable!("at least one fold precedes"),
        }
    }
}

pub fn open(
    params: &WhirParams,
    data: &WhirData,
    point: &[Fp3],
    t: &mut Transcript,
) -> (Fp3, WhirProof) {
    let cfg = &data.config;
    let m = cfg.num_vars;
    assert_eq!(point.len(), m, "point dimension");
    assert_eq!(
        WhirConfig::derive(params, m).as_ref(),
        Ok(cfg),
        "open params differ from commit params"
    );
    assert!(data.zk.is_none(), "a zero-knowledge commitment opens with open_zk");
    let ext0 = matches!(data.message, Message::Ext { .. });
    let value = match &data.message {
        Message::Base { evals, .. } => ml_eval_base(evals, point),
        Message::Ext { evals, .. } => ml_eval_ext(evals, point),
    };
    prologue(t, params, m, &Commitment(data.tree.root()), point, value, ext0);
    let (f, coeffs): (Vec<Fp3>, Coeffs) = match &data.message {
        Message::Base { evals, coeffs } => (
            evals.iter().map(|&x| Fp3::from_base(x)).collect(),
            Coeffs::Base(coeffs.clone()),
        ),
        Message::Ext { evals, coeffs } => (evals.clone(), Coeffs::Ext(coeffs.clone())),
    };
    let spec0 = cfg.rounds[0];
    let proof = prove_rounds(
        params,
        cfg,
        t,
        f,
        coeffs,
        eq_table(point),
        &|z| data.univariate(z),
        &|idx| open_leaves(&data.codeword, &data.tree, &spec0, idx),
    );
    (value, proof)
}

/// WHIR from step 2 of the transcript on: the round-0 word `f_0` given
/// as its table `f`, its monomial coefficients and its initial weight
/// table `w`; `univariate0` answers `f̂_0(ζ)`, `open0` opens leaves of
/// `f_0`'s tree. The caller has absorbed the prologue and the claim.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prove_rounds(
    params: &WhirParams,
    cfg: &WhirConfig,
    t: &mut Transcript,
    mut f: Vec<Fp3>,
    mut coeffs: Coeffs,
    mut w: Vec<Fp3>,
    univariate0: &(dyn Fn(Fp3) -> Fp3 + Sync),
    open0: &(dyn Fn(&[usize]) -> Opening + Sync),
) -> WhirProof {
    // commitment-phase OOD on f_0, then combine the weights (Construction 5.5)
    let m = cfg.num_vars;
    let spec0 = cfg.rounds[0];
    let mut ood0 = Vec::with_capacity(spec0.ood);
    let mut ood_points = Vec::with_capacity(spec0.ood);
    for _ in 0..spec0.ood {
        let z = t.squeeze_fp3();
        let y = univariate0(z);
        t.absorb_fp3(y);
        ood0.push(y);
        ood_points.push(z);
    }
    let gamma = t.squeeze_fp3();
    let mut g = gamma;
    for &z in &ood_points {
        add_scaled_eq(&mut w, &pow_point(z, m), g);
        g *= gamma;
    }

    let mut sumcheck0 = Vec::new();
    let mut fold_nonces0 = Vec::new();
    let mut alphas = sumcheck::prove(
        t,
        &mut f,
        &mut w,
        spec0.fold,
        spec0.fold_pow,
        &mut sumcheck0,
        &mut fold_nonces0,
        |a| coeffs.fold(a),
    );

    // the previously committed function after f_0: (codeword, tree)
    let mut prev_owned: Option<(Symbols, Tree)> = None;
    let mut prev_spec: RoundSpec = spec0;
    let mut rounds = Vec::with_capacity(cfg.rounds.len() - 1);
    let open_prev = |prev: &Option<(Symbols, Tree)>, spec: &RoundSpec, idx: &[usize]| match prev {
        Some((cw, tree)) => open_leaves(cw, tree, spec, idx),
        None => open0(idx),
    };

    for spec in cfg.rounds.iter().skip(1).copied() {
        let c = coeffs.ext().to_vec();
        let cw = Symbols::Ext(encode_ext(&c, spec.log_domain));
        let tree = coset_tree(&cw, spec.log_domain, spec.fold);
        let root = tree.root();
        t.absorb(root.as_bytes());

        let mut ood = Vec::with_capacity(spec.ood);
        let mut zs = Vec::with_capacity(spec.ood);
        for _ in 0..spec.ood {
            let z = t.squeeze_fp3();
            let y = univariate_ext(&c, z);
            t.absorb_fp3(y);
            ood.push(y);
            zs.push(z);
        }
        let query_nonce: Vec<u64> = pow::grind(t, prev_spec.query_pow).into_iter().collect();
        let idx = prev_spec.sample(t);
        let gamma = t.squeeze_fp3();
        let opening = open_prev(&prev_owned, &prev_spec, &idx);

        let mut g = gamma;
        for &z in &zs {
            add_scaled_eq(&mut w, &pow_point(z, spec.num_vars), g);
            g *= gamma;
        }
        for &j in &idx {
            let r = Fp3::from_base(shift_point(&prev_spec, j));
            add_scaled_eq(&mut w, &pow_point(r, spec.num_vars), g);
            g *= gamma;
        }

        let mut msgs = Vec::new();
        let mut fold_nonces = Vec::new();
        let new_alphas = sumcheck::prove(
            t,
            &mut f,
            &mut w,
            spec.fold,
            spec.fold_pow,
            &mut msgs,
            &mut fold_nonces,
            |a| coeffs.fold(a),
        );
        alphas.extend(new_alphas);
        rounds.push(WhirRoundProof {
            root,
            ood,
            query_nonce,
            opening,
            sumcheck: msgs,
            fold_nonces,
        });
        prev_owned = Some((cw, tree));
        prev_spec = spec;
    }

    let final_poly = coeffs.ext().to_vec();
    t.absorb_fp3_slice(&final_poly);
    let final_nonce: Vec<u64> = pow::grind(t, prev_spec.query_pow).into_iter().collect();
    let idx = prev_spec.sample(t);
    let final_opening = open_prev(&prev_owned, &prev_spec, &idx);

    WhirProof {
        params: *params,
        ood0,
        sumcheck0,
        fold_nonces0,
        rounds,
        final_poly,
        final_nonce,
        final_opening,
    }
}
