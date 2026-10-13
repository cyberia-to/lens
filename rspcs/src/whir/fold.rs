//! Folding cosets: leaf layout and the `Fold` operator (WHIR Definition
//! 4.14, iterated).
//!
//! A codeword on the order-`n` subgroup `⟨ω⟩` with folding width
//! `W = 2^k` has `n/W` leaves; leaf `j` holds the symbols at indices
//! `j + t·(n/W)`, `t = 0..W`, i.e. the values at `x_t = ω^j·ζ^t` with
//! `ζ = ω^{n/W}` a primitive `W`-th root of unity. All `x_t` satisfy
//! `x_t^W = ω^{jW}`, the shift point of leaf `j`.

use cyber_lens_core::PcsError;
use nebu::{Fp3, Goldilocks};

use super::config::RoundSpec;
use super::proof::Opening;
use crate::field::{half, mul_base, root_of_unity};
use crate::merkle::{self, Tree};
use crate::wire::Symbols;

/// Merkle tree over the folding cosets of `cw` (`2^log_n` symbols).
pub fn coset_tree(cw: &Symbols, log_n: u32, fold: usize) -> Tree {
    let width = 1usize << fold;
    let leaves = (1usize << log_n) / width;
    Tree::build(leaves, |j| coset(cw, leaves, width, j).leaf_bytes(0, width))
}

/// The `W` symbols of leaf `j`, in coset order.
pub fn coset(cw: &Symbols, leaves: usize, width: usize, j: usize) -> Symbols {
    match cw {
        Symbols::Base(v) => Symbols::Base((0..width).map(|t| v[j + t * leaves]).collect()),
        Symbols::Ext(v) => Symbols::Ext((0..width).map(|t| v[j + t * leaves]).collect()),
    }
}

/// Open the leaves at sorted distinct `idx`.
pub fn open(cw: &Symbols, tree: &Tree, spec: &RoundSpec, idx: &[usize]) -> Opening {
    let width = 1usize << spec.fold;
    let leaves = 1usize << spec.log_leaves();
    let symbols = match cw {
        Symbols::Base(_) => Symbols::Base(
            idx.iter()
                .flat_map(|&j| match coset(cw, leaves, width, j) {
                    Symbols::Base(v) => v,
                    Symbols::Ext(_) => unreachable!(),
                })
                .collect(),
        ),
        Symbols::Ext(_) => Symbols::Ext(
            idx.iter()
                .flat_map(|&j| match coset(cw, leaves, width, j) {
                    Symbols::Ext(v) => v,
                    Symbols::Base(_) => unreachable!(),
                })
                .collect(),
        ),
    };
    Opening {
        width,
        symbols,
        siblings: tree.open(idx),
    }
}

/// `Fold(f, α_1..α_k)` at the shift point of leaf `j`, from the leaf's
/// `W = 2^k` values. Each step pairs `x` with `−x = x·ζ^{W/2}`:
/// `f'(x²) = (f(x)+f(−x))/2 + α·(f(x)−f(−x))/(2x)`.
pub fn fold_leaf(values: &[Fp3], alphas: &[Fp3], log_n: u32, j: usize) -> Fp3 {
    let width = values.len();
    debug_assert_eq!(width, 1 << alphas.len());
    let omega = root_of_unity(log_n);
    let leaves = (1usize << log_n) / width;
    let inv2 = half();
    let mut vals = values.to_vec();
    let mut x0_inv = omega.exp(j as u64).inv();
    let mut zeta_inv = omega.exp(leaves as u64).inv();
    let mut w = width;
    for &alpha in alphas {
        let h = w / 2;
        let mut x_inv = x0_inv;
        for t in 0..h {
            let a = vals[t];
            let b = vals[t + h];
            let even = mul_base(a + b, inv2);
            let odd = mul_base(a - b, inv2 * x_inv);
            vals[t] = even + alpha * odd;
            x_inv *= zeta_inv;
        }
        w = h;
        x0_inv = x0_inv * x0_inv;
        zeta_inv = zeta_inv * zeta_inv;
    }
    vals[0]
}

/// Shift point of leaf `j`: `ω^{j·W}` for the code of `spec`.
pub fn shift_point(spec: &RoundSpec, j: usize) -> Goldilocks {
    root_of_unity(spec.log_leaves()).exp(j as u64)
}

/// Authenticate an opening of `spec`'s function against `root` and return
/// the folded value of every opened leaf.
pub fn check_opening(
    root: &cyber_hemera::Hash,
    spec: &RoundSpec,
    ext: bool,
    idx: &[usize],
    opening: &Opening,
    alphas: &[Fp3],
) -> Result<Vec<Fp3>, PcsError> {
    let width = 1usize << spec.fold;
    let leaves = 1usize << spec.log_leaves();
    let is_ext = matches!(opening.symbols, Symbols::Ext(_));
    if opening.width != width || is_ext != ext || opening.symbols.len() != idx.len() * width {
        return Err(PcsError::Shape);
    }
    let bytes: Vec<Vec<u8>> = (0..idx.len())
        .map(|q| opening.symbols.leaf_bytes(q * width, width))
        .collect();
    let opened: Vec<(&[u8], usize)> = bytes
        .iter()
        .map(Vec::as_slice)
        .zip(idx.iter().copied())
        .collect();
    let digests = merkle::leaf_digests(&opened, leaves);
    merkle::verify(
        root,
        spec.log_leaves() as usize,
        idx,
        digests,
        &opening.siblings,
    )?;
    Ok(idx
        .iter()
        .enumerate()
        .map(|(q, &j)| {
            let vals: Vec<Fp3> = (0..width)
                .map(|t| opening.symbols.get(q * width + t))
                .collect();
            fold_leaf(&vals, alphas, spec.log_domain, j)
        })
        .collect())
}
