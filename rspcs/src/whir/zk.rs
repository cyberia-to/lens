//! Zero-knowledge opening (`specs/whir.md` § zero-knowledge opening).
//!
//! The caller supplies the masks: a table `G` on the high `m − k₀`
//! variables (the virtual word `g(x) = G(x_{k₀+1..m})`, constant on every
//! folding coset, so a leaf carries one Fp3 symbol `Ĝ(y_j)` for it) and one
//! four-limb salt per round-0 leaf. WHIR runs on `h = c + β·g` with `β`
//! drawn after the claim's two values `v = Λ(c)`, `u = Λ(g)`. lens fixes
//! the commitment, transcript and checks; the zero-knowledge argument
//! (which masks the caller must place in `c`, and why every message is
//! then simulatable) belongs to the caller (zheng `specs/execution.md`
//! § veil-W).

use cyber_lens_core::{Commitment, MultilinearPcs, PcsError, Transcript};
use nebu::{Fp3, Goldilocks};

use super::config::RoundSpec;
use super::fold::fold_leaf;
use super::proof::{Opening, WhirProof};
use super::prover::{Coeffs, Message, WhirData, prove_rounds};
use super::verifier::{Constraint, shape, verify_rounds};
use super::{WhirConfig, WhirParams, batch};
use crate::field::{
    add_scaled_eq, mobius_base, mobius_ext, ml_eval_base, ml_eval_ext, univariate_base,
    univariate_ext,
};
use crate::merkle::{self, Tree};
use crate::rs::{encode_base, encode_ext};
use crate::soundness::{ext_field_bits, proximity};
use crate::wire::{Symbols, base_bytes};

/// Salt limbs per round-0 leaf.
pub const SALT_LIMBS: usize = 4;
/// Extra base symbols per round-0 leaf: `Ĝ(y_j)` (3) and the salt (4).
pub const EXTRA: usize = 3 + SALT_LIMBS;

/// A linear functional on tables over `{0,1}^m`:
/// `Σ_e coef_e·eq(q_e, ·) + Σ_i v_i·[· = b_i]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Weight {
    pub eq: Vec<(Vec<Fp3>, Fp3)>,
    pub explicit: Vec<(usize, Fp3)>,
}

impl Weight {
    fn check(&self, m: usize) -> Result<(), PcsError> {
        if self.eq.iter().any(|(q, _)| q.len() != m) || self.explicit.iter().any(|&(b, _)| b >> m != 0) {
            return Err(PcsError::Shape);
        }
        Ok(())
    }

    /// The weight's table on `{0,1}^m`.
    pub fn table(&self, m: usize) -> Vec<Fp3> {
        let mut w = vec![Fp3::ZERO; 1 << m];
        for (q, c) in &self.eq {
            add_scaled_eq(&mut w, q, *c);
        }
        for &(b, v) in &self.explicit {
            w[b] += v;
        }
        w
    }

    /// `Σ_b Λ(b)·t(b)` for a base table.
    pub fn apply_base(&self, t: &[Goldilocks]) -> Fp3 {
        let e: Fp3 = self.eq.iter().map(|(q, c)| *c * ml_eval_base(t, q)).fold(Fp3::ZERO, |a, x| a + x);
        self.explicit.iter().fold(e, |a, &(b, v)| a + v * Fp3::from_base(t[b]))
    }

    /// `Λ(g)` for `g(x) = G(x_{k₀+1..m})` given `G`'s table.
    pub fn apply_lifted(&self, g: &[Fp3], k0: usize) -> Fp3 {
        let e: Fp3 = self.eq.iter().map(|(q, c)| *c * ml_eval_ext(g, &q[k0..])).fold(Fp3::ZERO, |a, x| a + x);
        self.explicit.iter().fold(e, |a, &(b, v)| a + v * g[b >> k0])
    }

    fn absorb(&self, t: &mut Transcript) {
        t.absorb_u64(self.eq.len() as u64);
        for (q, c) in &self.eq {
            t.absorb_fp3_slice(q);
            t.absorb_fp3(*c);
        }
        t.absorb_u64(self.explicit.len() as u64);
        for &(b, v) in &self.explicit {
            t.absorb_u64(b as u64);
            t.absorb_fp3(v);
        }
    }
}

/// The masks of a zero-knowledge commitment (prover side).
pub struct ZkRound0 {
    g_evals: Vec<Fp3>,
    g_coeffs: Vec<Fp3>,
    g_codeword: Vec<Fp3>,
    salts: Vec<Goldilocks>,
    block: Option<usize>,
}

/// Masks the caller draws: `g` over `m − k₀` variables, `SALT_LIMBS` salt
/// limbs per round-0 leaf (leaf-major), and optionally `log2` of the
/// caller's masking block at the top of the table: then [`open_zk`] runs
/// the round-0 rank check (`check`) and answers [`ZkOutcome::Resample`]
/// when the coins fail it.
pub struct ZkMask {
    pub g: Vec<Fp3>,
    pub salts: Vec<Goldilocks>,
    pub block: Option<usize>,
}

/// What [`open_zk`] produced.
#[derive(Debug)]
pub enum ZkOutcome {
    Opened(ZkValues, WhirProof),
    /// The round-0 coins fail the masking lemma's rank condition: commit
    /// again with fresh masks and salts (never output this proof).
    Resample,
}

/// `(k₀, log2 leaves)` of round 0 for `(params, m)`.
pub fn round0_shape(params: &WhirParams, m: usize) -> Result<(usize, u32), PcsError> {
    let spec = WhirConfig::derive(params, m)?.rounds[0];
    Ok((spec.fold, spec.log_leaves()))
}

fn leaf_symbols(cw: &[Goldilocks], spec: &RoundSpec, zk: &ZkRound0, j: usize) -> Vec<Goldilocks> {
    let width = 1usize << spec.fold;
    let leaves = 1usize << spec.log_leaves();
    let mut v: Vec<Goldilocks> = Vec::with_capacity(width + EXTRA);
    v.extend((0..width).map(|t| cw[j + t * leaves]));
    let g = zk.g_codeword[j];
    v.extend([g.c0, g.c1, g.c2]);
    v.extend_from_slice(&zk.salts[SALT_LIMBS * j..SALT_LIMBS * (j + 1)]);
    v
}

/// Commit `c` with the masks of a zero-knowledge opening.
pub fn commit_zk(params: &WhirParams, evals: &[Goldilocks], mask: ZkMask) -> Result<(Commitment, WhirData), PcsError> {
    if !evals.len().is_power_of_two() {
        return Err(PcsError::Shape);
    }
    let m = evals.len().trailing_zeros() as usize;
    let config = WhirConfig::derive(params, m)?;
    let spec = config.rounds[0];
    let leaves = 1usize << spec.log_leaves();
    if mask.g.len() != 1 << (m - spec.fold) || mask.salts.len() != SALT_LIMBS * leaves {
        return Err(PcsError::Shape);
    }
    let mut coeffs = evals.to_vec();
    mobius_base(&mut coeffs);
    let cw = encode_base(&coeffs, spec.log_domain);
    let mut g_coeffs = mask.g.clone();
    mobius_ext(&mut g_coeffs);
    // Ĝ on the shift points y_j = ω^{j·2^{k₀}}: the order-(n₀/2^{k₀}) subgroup
    let g_codeword = encode_ext(&g_coeffs, spec.log_leaves());
    if mask.block.is_some_and(|rho| rho < spec.fold || rho >= m) {
        return Err(PcsError::Shape);
    }
    let zk = ZkRound0 {
        g_evals: mask.g,
        g_coeffs,
        g_codeword,
        salts: mask.salts,
        block: mask.block,
    };
    let tree = Tree::build(leaves, |j| base_bytes(&leaf_symbols(&cw, &spec, &zk, j)));
    let root = Commitment(tree.root());
    let message = Message::Base {
        evals: evals.to_vec(),
        coeffs,
    };
    Ok((
        root,
        WhirData {
            config,
            message,
            codeword: Symbols::Base(cw),
            tree,
            zk: Some(zk),
        },
    ))
}

fn prologue_zk(t: &mut Transcript, params: &WhirParams, m: usize, root: &Commitment, w: &Weight, v: Fp3, u: Fp3) {
    t.absorb(super::Whir::NAME.as_bytes());
    t.absorb(b"zk");
    t.absorb(&params.header());
    t.absorb_u64(m as u64);
    t.absorb(root.as_bytes());
    w.absorb(t);
    t.absorb_fp3(v);
    t.absorb_fp3(u);
}

/// The values a zero-knowledge opening carries beside its `WhirProof`:
/// `v = Λ(c)`, `u = Λ(g)` and the grinding nonce before `β` (0 when the
/// schedule grinds none there).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZkValues {
    pub v: Fp3,
    pub u: Fp3,
    pub nonce: u64,
}

/// Grinding bits before `β`: what brings the two-word combination to the
/// target (`batch::comb_pow` with two words).
pub fn comb_pow(params: &WhirParams, m: usize) -> Result<u32, PcsError> {
    let cfg = WhirConfig::derive(params, m)?;
    Ok(batch::comb_pow(&cfg, 2, f64::from(params.security_target)))
}

/// Open the claim `Λ`.
pub fn open_zk(params: &WhirParams, data: &WhirData, weight: &Weight, t: &mut Transcript) -> Result<ZkOutcome, PcsError> {
    let cfg = &data.config;
    let m = cfg.num_vars;
    if WhirConfig::derive(params, m).as_ref() != Ok(cfg) {
        return Err(PcsError::ParamsMismatch);
    }
    weight.check(m)?;
    let (Some(zk), Message::Base { evals, coeffs }) = (&data.zk, &data.message) else {
        return Err(PcsError::Shape);
    };
    let spec0 = cfg.rounds[0];
    let k0 = spec0.fold;
    let v = weight.apply_base(evals);
    let u = weight.apply_lifted(&zk.g_evals, k0);
    prologue_zk(t, params, m, &Commitment(data.tree.root()), weight, v, u);
    let nonce = crate::pow::grind(t, comb_pow(params, m)?).unwrap_or(0);
    let beta = t.squeeze_fp3();
    // h = c + β·g: table, monomial coefficients
    let f: Vec<Fp3> = crate::par::par_map(evals.len(), |b| Fp3::from_base(evals[b]) + beta * zk.g_evals[b >> k0]);
    let mut hc: Vec<Fp3> = coeffs.iter().map(|&x| Fp3::from_base(x)).collect();
    for (i, &gc) in zk.g_coeffs.iter().enumerate() {
        hc[i << k0] += beta * gc;
    }
    let cw = match &data.codeword {
        Symbols::Base(v) => v,
        Symbols::Ext(_) => return Err(PcsError::Shape),
    };
    let univariate0 = |z: Fp3| {
        let zk0 = (0..k0).fold(z, |a, _| a * a);
        univariate_base(coeffs, z) + beta * univariate_ext(&zk.g_coeffs, zk0)
    };
    let open0 = |idx: &[usize]| Opening {
        width: (1 << k0) + EXTRA,
        symbols: Symbols::Base(idx.iter().flat_map(|&j| leaf_symbols(cw, &spec0, zk, j)).collect()),
        siblings: data.tree.open(idx),
    };
    let table = weight.table(m);
    let lambda_block = zk.block.map(|rho| table[(1 << m) - (1 << rho)..].to_vec());
    let (proof, coins0) = prove_rounds(params, cfg, t, f, Coeffs::Ext(hc), table, &univariate0, &open0);
    if let (Some(rho), Some(lambda)) = (zk.block, lambda_block) {
        if !check::rank_ok(&spec0, m, rho, &lambda, beta, &coins0) {
            return Ok(ZkOutcome::Resample);
        }
    }
    Ok(ZkOutcome::Opened(ZkValues { v, u, nonce }, proof))
}

/// Verify a zero-knowledge opening of `Λ` with values `vals`.
pub fn verify_zk(
    params: &WhirParams,
    root: &Commitment,
    m: usize,
    weight: &Weight,
    vals: &ZkValues,
    proof: &WhirProof,
    t: &mut Transcript,
) -> Result<(), PcsError> {
    if proof.params != *params {
        return Err(PcsError::ParamsMismatch);
    }
    let cfg = WhirConfig::derive(params, m)?;
    weight.check(m)?;
    shape(&cfg, proof)?;
    let ZkValues { v, u, nonce } = *vals;
    prologue_zk(t, params, m, root, weight, v, u);
    let pow_bits = comb_pow(params, m)?;
    if pow_bits == 0 && nonce != 0 {
        return Err(PcsError::Shape);
    }
    if !crate::pow::check(t, pow_bits, nonce) {
        return Err(PcsError::ProofOfWork);
    }
    let beta = t.squeeze_fp3();
    let spec0 = cfg.rounds[0];
    let cons = weight
        .eq
        .iter()
        .map(|(q, c)| Constraint { round: 0, point: q.clone(), coef: *c })
        .collect();
    let check0 = |idx: &[usize], o: &Opening, alphas: &[Fp3]| check_zk_opening(&root.0, &spec0, idx, o, alphas, beta);
    verify_rounds(&cfg, t, cons, &weight.explicit, v + beta * u, proof, &check0)
}

/// Authenticate opened round-0 leaves and fold `h = c + β·Ĝ(y_j)`.
fn check_zk_opening(
    root: &cyber_hemera::Hash,
    spec: &RoundSpec,
    idx: &[usize],
    o: &Opening,
    alphas: &[Fp3],
    beta: Fp3,
) -> Result<Vec<Fp3>, PcsError> {
    let width = 1usize << spec.fold;
    let w = width + EXTRA;
    let leaves = 1usize << spec.log_leaves();
    let Symbols::Base(sym) = &o.symbols else {
        return Err(PcsError::Shape);
    };
    if o.width != w || sym.len() != idx.len() * w {
        return Err(PcsError::Shape);
    }
    let bytes: Vec<Vec<u8>> = sym.chunks_exact(w).map(base_bytes).collect();
    let opened: Vec<(&[u8], usize)> = bytes.iter().map(Vec::as_slice).zip(idx.iter().copied()).collect();
    let digests = merkle::leaf_digests(&opened, leaves);
    merkle::verify(root, spec.log_leaves() as usize, idx, digests, &o.siblings)?;
    Ok(idx
        .iter()
        .zip(sym.chunks_exact(w))
        .map(|(&j, leaf)| {
            let g = beta * Fp3::new(leaf[width], leaf[width + 1], leaf[width + 2]);
            let vals: Vec<Fp3> = leaf[..width].iter().map(|&x| Fp3::from_base(x) + g).collect();
            fold_leaf(&vals, alphas, spec.log_domain, j)
        })
        .collect())
}

/// Round-by-round terms of a zero-knowledge opening at `(params, m)`:
/// WHIR's on `h`, the two-word combination by `β` (`ε_mca(1) + |Λ|/|K|`,
/// `batch::comb_log_err`, with its grinding), and the binding of `u`
/// before `β` (`|Λ|²/|K|`).
pub fn zk_terms(params: &WhirParams, m: usize) -> Result<Vec<(String, f64)>, PcsError> {
    let cfg = WhirConfig::derive(params, m)?;
    let mut out = cfg.terms();
    let pow_bits = comb_pow(params, m)?;
    out.push(("zk combine β".into(), -batch::comb_log_err(&cfg, 2) + f64::from(pow_bits)));
    let s = cfg.rounds[0];
    let list = proximity(s.regime, s.log_inv_rate, s.log_domain).log_list;
    out.push(("zk u before β".into(), ext_field_bits() - 2.0 * list));
    Ok(out)
}

/// Proven bits of a zero-knowledge opening (minimum over [`zk_terms`]).
pub fn zk_security_bits(params: &WhirParams, m: usize) -> f64 {
    zk_terms(params, m).map_or(0.0, |t| t.into_iter().map(|(_, b)| b).fold(f64::INFINITY, f64::min))
}

mod check;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod lemma;
#[cfg(test)]
mod lemma_prod;
