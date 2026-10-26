//! Soundness of a batched opening: `m` words of one code (WHIR round 0's
//! at `(params, ℓ)`), carrying `J` evaluation claims in total, opened at
//! once (`specs/whir.md` § batched openings).
//!
//! ```text
//! γ        Σ_i Σ_e γ^e·w_e(X)·f_i(X): the claims batched
//! sumcheck ℓ rounds to one point ρ (m > 1; one word takes its claims
//!          as WHIR's initial weights directly)
//! r        f_0 = Σ_i r^i·f_i, WHIR on f_0
//! ```
//!
//! | round | error | source |
//! |---|---|---|
//! | `γ` | `|Λ|·(J − 1)/|K|` | Schwartz–Zippel in `γ`, union over the list `Λ` of `C^m` at δ (WHIR Lemma 4.4) |
//! | sumcheck round | `2|Λ|/|K|` | sumcheck, degree 2 |
//! | `r` | `ε_mca(m − 1) + |Λ|(m − 1)/|K|`, minus grinding | BCGM eprint 2025/2051 Thm 9.2 / Lemma 9.3 (powers generator, degree `m − 1`) with the argument of WHIR Lemma 4.13 |
//!
//! `ε_mca(d) = d·` the affine-line term ([`crate::soundness::mca_log2`]).
//! The weights may be any multilinear polynomials whose value the
//! verifier computes at the folding point (eq and pow points, products
//! of a row and a column polynomial, sparse linear forms): WHIR's fold
//! term `3ℓ/|K|` holds for weights of degree ≤ 1 in every variable
//! (Theorem 5.2, `d* = 3`).

use super::WhirConfig;
use crate::soundness::{ext_field_bits, log2_add, mca_log2, proximity};

fn log_list0(c: &WhirConfig) -> f64 {
    let s = c.rounds[0];
    proximity(s.regime, s.log_inv_rate, s.log_domain).log_list
}

/// `log2` of the combination round's error before grinding (−∞ for one
/// word).
pub fn comb_log_err(c: &WhirConfig, inputs: usize) -> f64 {
    if inputs <= 1 {
        return f64::NEG_INFINITY;
    }
    let s = c.rounds[0];
    let d = ((inputs - 1) as f64).log2();
    let mca = mca_log2(s.regime, s.log_inv_rate, s.log_domain) + d;
    log2_add(mca, log_list0(c) + d - ext_field_bits())
}

/// Grinding before `r` that brings the combination to `target` bits.
pub fn comb_pow(c: &WhirConfig, inputs: usize, target: f64) -> u32 {
    (target + comb_log_err(c, inputs)).ceil().max(0.0) as u32
}

/// The batch's round-by-round terms `(name, bits)` for `inputs` words
/// and `claims` claims, `comb_pow` grinding bits before `r`.
pub fn terms(c: &WhirConfig, inputs: usize, claims: usize, comb_pow: u32) -> Vec<(String, f64)> {
    let k = ext_field_bits();
    let list = log_list0(c);
    let mut out = Vec::new();
    if claims > 1 {
        out.push(("batch γ".into(), k - list - ((claims - 1) as f64).log2()));
    }
    if inputs > 1 {
        out.push(("batch sumcheck".into(), k - list - 1.0));
        out.push((
            "batch combine".into(),
            -comb_log_err(c, inputs) + f64::from(comb_pow),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WhirParams;

    #[test]
    fn a_batch_reaches_the_target_with_its_grinding() {
        let p = WhirParams {
            log_inv_rate: 4,
            pow_bits: 24,
            ..WhirParams::default()
        };
        let c = WhirConfig::derive(&p, 20).unwrap();
        assert!(terms(&c, 1, 1, 0).is_empty());
        assert_eq!(terms(&c, 1, 5, 0).len(), 1, "one word: the claims' γ only");
        for m in [2, 3, 4] {
            let pow = comb_pow(&c, m, 128.0);
            let t = terms(&c, m, 60, pow);
            assert!(t.iter().all(|(_, b)| *b >= 128.0), "{m}: {t:?}");
            assert!(pow <= 32, "grinding {pow}");
        }
        assert_eq!(comb_pow(&c, 1, 128.0), 0);
    }
}
