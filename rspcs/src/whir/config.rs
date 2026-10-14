//! WHIR round schedule and its proven soundness, as code.
//!
//! Function `f_i` (`i = 0..M−1`) has `m_i` variables and lives on the
//! order-`n_i` subgroup, `n_i = 2^{m_i + r_i}`, rate `ρ_i = 2^{−r_i}`,
//! `r_{i+1} = r_i + k_i − 1` (so `n_{i+1} = n_i / 2`). Round `i` folds
//! `k_i` variables by sumcheck; `f_i` is queried `t_i` times on the folded
//! domain of order `n_i / 2^{k_i}`.
//!
//! Round-by-round errors (WHIR = Arnon–Chiesa–Fenzi–Yogev, eprint
//! 2024/1586; `ℓ_i` = list bound, `δ_i` = proximity of `f_i`'s code):
//! - `ood_0`   `(ℓ_0²/2)·(2^{m_0}/|K|)^{s_0}` — WHIR Theorem 7.5 (`ε^out`)
//!   with Lemma 4.25 for `s_0` samples.
//! - `comb_0`  `s_0·ℓ_0/|K|` — WHIR Theorem 5.6 (`(t−1)ℓ/|F|`, `t = 1+s_0`
//!   weights).
//! - `fold_{i,s}` `3ℓ_i/|K| + ε_mca(RS[n_i/2^s, 2^{m_i−s}])`, `s = 1..k_i`
//!   — WHIR Theorem 5.2 (`ε^fold`, `d* = d = 3` for `Z·eq(z, X)`
//!   weights); `ε_mca` from `soundness::mca_log2`.
//! - `ood_i`   `(ℓ_i²/2)·(2^{m_i}/|K|)^{s_i}` — Theorem 5.2 (`ε^out`) with
//!   Lemma 4.25.
//! - `shift_i` `(1−δ_{i−1})^{t_{i−1}} + ℓ_i·(t_{i−1}+s_i)/|K|` — Theorem 5.2
//!   (`ε^shift`; the γ-combination has degree `t+s` with `s` OOD samples).
//! - `fin`     `(1−δ_{M−1})^{t_{M−1}}` — Theorem 5.2 (`ε^fin`).
//!
//! Grinding before a challenge adds its bits to that round's term. The
//! reported security is the minimum over all terms.

use cyber_lens_core::PcsError;

use super::{Decoding, WhirParams};
use crate::soundness::{Regime, ext_field_bits, log2_add, mca_log2, proximity};

/// One committed function and its folding round.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundSpec {
    pub num_vars: usize,
    pub fold: usize,
    pub log_inv_rate: u32,
    pub log_domain: u32,
    pub regime: Regime,
    /// OOD samples on this function after its commitment.
    pub ood: usize,
    /// Queries on this function (next round's shift, or the final check).
    pub queries: usize,
    pub query_pow: u32,
    /// Grinding before each folding challenge of this round.
    pub fold_pow: u32,
}

impl RoundSpec {
    /// log2 of the number of leaves (folding cosets).
    pub fn log_leaves(&self) -> u32 {
        self.log_domain - self.fold as u32
    }

    /// Every leaf is opened (no sampling, no grinding).
    pub fn opens_all(&self) -> bool {
        self.queries == 1 << self.log_leaves()
    }

    /// Opened leaf indices: all of them, or `queries` transcript samples.
    pub(crate) fn sample(&self, t: &mut cyber_lens_core::Transcript) -> Vec<usize> {
        if self.opens_all() {
            (0..1 << self.log_leaves()).collect()
        } else {
            crate::merkle::dedup_sorted(t.squeeze_indices(self.queries, self.log_leaves()))
        }
    }

    /// `log2` of the query error `(1−δ)^t` (−∞ when every leaf is opened).
    pub fn query_log_err(&self) -> f64 {
        if self.opens_all() {
            return f64::NEG_INFINITY;
        }
        self.queries as f64
            * proximity(self.regime, self.log_inv_rate, self.log_domain).log_one_minus_delta
    }
}

/// Schedule derived from params at one size.
#[derive(Clone, Debug, PartialEq)]
pub struct WhirConfig {
    pub num_vars: usize,
    pub rounds: Vec<RoundSpec>,
    pub final_vars: usize,
}

const MAX_JOHNSON_M: u32 = 64;

fn fold_log_err(regime: Regime, r: u32, log_n: u32, fold: usize) -> f64 {
    let k = ext_field_bits();
    let list = 3f64.log2() + proximity(regime, r, log_n).log_list - k;
    (1..=fold)
        .map(|s| log2_add(list, mca_log2(regime, r, log_n - s as u32)))
        .fold(f64::NEG_INFINITY, f64::max)
}

fn ood_log_err(log_list: f64, m: usize, s: usize) -> f64 {
    if s == 0 {
        return if log_list == 0.0 {
            f64::NEG_INFINITY
        } else {
            0.0
        };
    }
    2.0 * log_list - 1.0 + s as f64 * (m as f64 - ext_field_bits())
}

fn query_log_err(log_one_minus_delta: f64, t: usize, comb: f64) -> f64 {
    log2_add(t as f64 * log_one_minus_delta, comb)
}

impl WhirConfig {
    pub fn derive(params: &WhirParams, m: usize) -> Result<Self, PcsError> {
        let k = usize::from(params.folding_factor);
        let r0 = u32::from(params.log_inv_rate);
        if m == 0 || k == 0 || r0 == 0 || m as u32 + r0 > 32 {
            return Err(PcsError::Shape);
        }
        let target = f64::from(params.security_target);
        let pow_max = u32::from(params.pow_bits);

        // schedule: fold min(k, m) first, keep committing while > max_final_vars remain
        let mut folds = vec![k.min(m)];
        let mut rem = m - folds[0];
        while rem > usize::from(params.max_final_vars) {
            let f = k.min(rem);
            folds.push(f);
            rem -= f;
        }

        // pass 1: per-code regime, OOD count, fold grinding
        let mut rounds = Vec::with_capacity(folds.len());
        let (mut mi, mut ri) = (m, r0);
        for &f in &folds {
            let log_n = mi as u32 + ri;
            let candidates: Vec<Regime> = match params.decoding {
                Decoding::Unique => vec![Regime::Unique],
                Decoding::Johnson => (3..=MAX_JOHNSON_M).map(Regime::Johnson).collect(),
            };
            let mut best: Option<(usize, u32, Regime)> = None;
            for reg in candidates {
                let fold_bits = -fold_log_err(reg, ri, log_n, f);
                let fold_pow = (target - fold_bits).ceil().max(0.0) as u32;
                if fold_pow > pow_max {
                    continue;
                }
                let prox = proximity(reg, ri, log_n);
                let need =
                    (target - f64::from(pow_max.min(params.security_target.into()))).max(0.0);
                let t = ((need / -prox.log_one_minus_delta).ceil() as usize).max(1);
                if best.is_none_or(|(bt, bp, _)| (t, fold_pow) < (bt, bp)) {
                    best = Some((t, fold_pow, reg));
                }
            }
            // nothing within the grinding budget: take the smallest MCA error
            let (_, fold_pow, regime) = best.unwrap_or_else(|| {
                let reg = match params.decoding {
                    Decoding::Unique => Regime::Unique,
                    Decoding::Johnson => Regime::Johnson(3),
                };
                (0, pow_max, reg)
            });
            let log_list = proximity(regime, ri, log_n).log_list;
            let ood = match regime {
                Regime::Unique => 0,
                Regime::Johnson(_) => (1..64)
                    .find(|&s| -ood_log_err(log_list, mi, s) >= target)
                    .unwrap_or(64),
            };
            rounds.push(RoundSpec {
                num_vars: mi,
                fold: f,
                log_inv_rate: ri,
                log_domain: log_n,
                regime,
                ood,
                queries: 0,
                query_pow: pow_max.min(params.security_target.into()),
                fold_pow,
            });
            mi -= f;
            ri += f as u32 - 1;
        }

        // pass 2: queries, including the next round's γ-combination term
        for i in 0..rounds.len() {
            let cur = rounds[i];
            let prox = proximity(cur.regime, cur.log_inv_rate, cur.log_domain);
            let comb = match rounds.get(i + 1) {
                Some(next) => {
                    let l = proximity(next.regime, next.log_inv_rate, next.log_domain).log_list;
                    Some((l, next.ood))
                }
                None => None,
            };
            let mut t = 1;
            loop {
                let c = comb.map_or(f64::NEG_INFINITY, |(l, s)| {
                    l + ((t + s) as f64).log2() - ext_field_bits()
                });
                let bits =
                    -query_log_err(prox.log_one_minus_delta, t, c) + f64::from(cur.query_pow);
                if bits >= target || t >= 1 << 16 {
                    break;
                }
                t += 1;
            }
            let leaves = 1usize << cur.log_leaves();
            if t >= leaves {
                // saturated sampling: open every leaf, no grinding
                rounds[i].queries = leaves;
                rounds[i].query_pow = 0;
            } else {
                rounds[i].queries = t;
            }
        }

        Ok(Self {
            num_vars: m,
            rounds,
            final_vars: rem,
        })
    }

    /// Every round-by-round term as `(name, bits)`.
    pub fn terms(&self) -> Vec<(String, f64)> {
        let k = ext_field_bits();
        let mut out = Vec::new();
        for (i, rd) in self.rounds.iter().enumerate() {
            let prox = proximity(rd.regime, rd.log_inv_rate, rd.log_domain);
            if rd.ood > 0 || prox.log_list > 0.0 {
                out.push((
                    format!("ood_{i}"),
                    -ood_log_err(prox.log_list, rd.num_vars, rd.ood),
                ));
            }
            if i == 0 && rd.ood > 0 {
                out.push(("comb_0".into(), k - prox.log_list - (rd.ood as f64).log2()));
            }
            if i > 0 {
                let prev = self.rounds[i - 1];
                let c = prox.log_list + ((prev.queries + rd.ood) as f64).log2() - k;
                let e = log2_add(prev.query_log_err(), c);
                out.push((format!("shift_{i}"), -e + f64::from(prev.query_pow)));
            }
            let fe = fold_log_err(rd.regime, rd.log_inv_rate, rd.log_domain, rd.fold);
            out.push((format!("fold_{i}"), -fe + f64::from(rd.fold_pow)));
        }
        let last = self.rounds.last().expect("at least one round");
        out.push((
            "fin".into(),
            -last.query_log_err() + f64::from(last.query_pow),
        ));
        out
    }

    /// Proven round-by-round soundness in bits.
    pub fn security_bits(&self) -> f64 {
        self.terms()
            .into_iter()
            .map(|(_, b)| b)
            .fold(f64::INFINITY, f64::min)
    }

    /// The same schedule's bound with every grinding term removed: what
    /// the queries, OOD samples and field size prove on their own. Stated
    /// beside [`Self::security_bits`] in the ledgers, never instead of it.
    pub fn security_bits_without_grinding(&self) -> f64 {
        let mut bare = self.clone();
        for round in &mut bare.rounds {
            round.query_pow = 0;
            round.fold_pow = 0;
        }
        bare.security_bits()
    }

    /// Hemera evaluations the honest prover expects to spend grinding:
    /// `2^bits` per grinding challenge (each folding challenge with a
    /// positive `fold_pow`, each round's query challenge).
    pub fn expected_grinding_hashes(&self) -> f64 {
        self.rounds
            .iter()
            .map(|r| {
                let fold = if r.fold_pow > 0 { r.fold as f64 * f64::from(r.fold_pow).exp2() } else { 0.0 };
                let query = if r.query_pow > 0 { f64::from(r.query_pow).exp2() } else { 0.0 };
                fold + query
            })
            .sum()
    }

    /// Offset of round `i`'s first folding challenge in the full α vector.
    pub fn alpha_offset(&self, i: usize) -> usize {
        self.rounds[..i].iter().map(|r| r.fold).sum()
    }
}
