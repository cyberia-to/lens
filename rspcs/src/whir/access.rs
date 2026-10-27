//! Read access to a committed WHIR function `f_0`, for protocols that
//! query the codeword directly (accumulation spot checks, out-of-domain
//! answers) before or instead of an opening.
//!
//! Layout (`specs/whir.md` § commit): the codeword of `f̂_0` lives on the
//! order-`n_0` subgroup `⟨ω⟩`, symbol `s` is `f̂_0(ω^s)`; leaf `j` of the
//! Merkle tree holds the folding coset `{s = j + t·(n_0/W) : t < W}`,
//! `W = 2^{k_0}`. A query of symbol `s` opens leaf `s mod (n_0/W)`.

use cyber_hemera::Hash;
use cyber_lens_core::PcsError;
use nebu::{Fp3, Goldilocks};

use super::fold::{coset, open as open_cosets};
use super::proof::Opening;
use super::prover::{Message, WhirData};
use super::{WhirConfig, WhirParams};
use crate::field::{root_of_unity, univariate_base, univariate_ext};
use crate::merkle;
use crate::wire::Symbols;

/// Domain and leaf geometry of `f_0` for given parameters and size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafLayout {
    /// `log2 n_0`: the codeword length.
    pub log_domain: u32,
    /// `log2 W`: symbols per leaf.
    pub log_width: u32,
}

impl LeafLayout {
    pub fn derive(params: &WhirParams, num_vars: usize) -> Result<Self, PcsError> {
        let spec = WhirConfig::derive(params, num_vars)?.rounds[0];
        Ok(Self {
            log_domain: spec.log_domain,
            log_width: spec.fold as u32,
        })
    }
    /// `log2` of the number of leaves.
    pub fn log_leaves(&self) -> u32 {
        self.log_domain - self.log_width
    }
    /// Leaf holding symbol `s`, and the symbol's position in that leaf.
    pub fn locate(&self, s: usize) -> (usize, usize) {
        let leaves = 1usize << self.log_leaves();
        (s % leaves, s / leaves)
    }
    /// The evaluation point `ω^s` of symbol `s`.
    pub fn point(&self, s: usize) -> Goldilocks {
        root_of_unity(self.log_domain).exp(s as u64)
    }
}

impl WhirData {
    /// The Merkle root of `f_0`.
    pub fn root(&self) -> Hash {
        self.tree.root()
    }
    /// Variables of the committed multilinear.
    pub fn num_vars(&self) -> usize {
        self.config.num_vars
    }
    /// Whether the committed message is over Fp3.
    pub fn is_ext(&self) -> bool {
        matches!(self.message, Message::Ext { .. })
    }
    /// Codeword symbol `s`, lifted to Fp3.
    pub fn symbol(&self, s: usize) -> Fp3 {
        self.codeword.get(s)
    }
    /// `f̂_0(z)` for any `z ∈ K` (out-of-domain answers).
    pub fn univariate(&self, z: Fp3) -> Fp3 {
        match &self.message {
            Message::Base { coeffs, .. } => univariate_base(coeffs, z),
            Message::Ext { coeffs, .. } => univariate_ext(coeffs, z),
        }
    }
    /// The evaluation table, lifted to Fp3.
    pub fn table(&self) -> Vec<Fp3> {
        match &self.message {
            Message::Base { evals, .. } => evals.iter().map(|&x| Fp3::from_base(x)).collect(),
            Message::Ext { evals, .. } => evals.clone(),
        }
    }
    /// Open the sorted, distinct leaves `leaves` (cosets of `f_0`).
    pub fn open_leaves(&self, leaves: &[usize]) -> Opening {
        open_cosets(&self.codeword, &self.tree, &self.config.rounds[0], leaves)
    }
    /// The symbols of leaf `j`, lifted to Fp3 (prover side).
    pub fn leaf(&self, j: usize) -> Vec<Fp3> {
        let spec = self.config.rounds[0];
        let width = 1usize << spec.fold;
        let leaves = 1usize << spec.log_leaves();
        let c = coset(&self.codeword, leaves, width, j);
        (0..width).map(|t| c.get(t)).collect()
    }
}

/// Authenticate an opening of the sorted, distinct `leaves` of a function
/// committed under `root` with `layout`; `ext` says whether its symbols are
/// Fp3. Returns the symbols of every opened leaf (lifted to Fp3), leaf by
/// leaf in coset order.
pub fn verify_leaves(
    root: &Hash,
    layout: &LeafLayout,
    ext: bool,
    leaves: &[usize],
    opening: &Opening,
) -> Result<Vec<Vec<Fp3>>, PcsError> {
    let width = 1usize << layout.log_width;
    let count = 1usize << layout.log_leaves();
    let is_ext = matches!(opening.symbols, Symbols::Ext(_));
    if opening.width != width
        || is_ext != ext
        || opening.symbols.len() != leaves.len() * width
        || leaves.windows(2).any(|w| w[0] >= w[1])
        || leaves.last().is_some_and(|&j| j >= count)
    {
        return Err(PcsError::Shape);
    }
    let bytes: Vec<Vec<u8>> = (0..leaves.len())
        .map(|q| opening.symbols.leaf_bytes(q * width, width))
        .collect();
    let opened: Vec<(&[u8], usize)> = bytes
        .iter()
        .map(Vec::as_slice)
        .zip(leaves.iter().copied())
        .collect();
    let digests = merkle::leaf_digests(&opened, count);
    merkle::verify(
        root,
        layout.log_leaves() as usize,
        leaves,
        digests,
        &opening.siblings,
    )?;
    Ok((0..leaves.len())
        .map(|q| {
            (0..width)
                .map(|t| opening.symbols.get(q * width + t))
                .collect()
        })
        .collect())
}
