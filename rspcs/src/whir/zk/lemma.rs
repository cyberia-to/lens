//! The round-0 masking lemma, checked by exact linear algebra on a small
//! instance (`docs/explanation/zk-small.md` § the design, zheng).
//!
//! Coins fixed (`β`, one OOD point, `γ`, the `k₀` folding challenges, the
//! queried leaves, `z_ψ`, `μ`). The round-0 messages are affine in the
//! committed table `c` and the mask table `G`: `u_ψ = Ψ(c)`, `u = Λ(g)`,
//! the OOD answer of `h = c + β·g`, the `2k₀` sumcheck values, the opened
//! `c` cosets and `Ĝ(y_j)`. For a change `Δ` of `c` off the block with
//! `F'(Δ) = 0` (the claim's value is the Spartan side's), the test sets
//! `ΔG = −Fold(Δc)/β` (so `f_1` is unchanged and every later message
//! with it) and solves for the block change `x ∈ F^R` that keeps every
//! round-0 message: the lemma says a solution exists — the system on `x`
//! has the full rank `q₀·2^{k₀} + 3·(E − 1)` (one Fp3 relation is
//! automatic: `f_1` fixes `σ_{k₀}`) — once `D = R/2^{k₀} ≥ q₀ + 3·(1 + s₀)`
//! and `Ψ` weighs the block with uniform coefficients `ξ`; below the bound,
//! or with `Ψ` at a geometric point (attempt 1), it fails.
use super::*;
use crate::field::{fold_evals_ext, mobius_ext, pow_point};
use crate::tests::common::Rng;

const M: usize = 10;
const K0: usize = 4;
const LOG_DOMAIN: u32 = M as u32 + 2;
const Q0: usize = 8;

struct Coins {
    beta: Fp3,
    z_ood: Fp3,
    gamma: Fp3,
    alphas: Vec<Fp3>,
    leaves: Vec<usize>,
    psi: (Vec<Fp3>, Fp3),
    /// Random block weights for `Ψ` (empty: the geometric point).
    xi: Vec<Fp3>,
    weight: Weight,
}

/// `∏_{i ≥ ρ} q_i` as one eq term at `(q_{<ρ}, 1…1)`.
fn block_term(q: &[Fp3], rho: usize, scale: Fp3) -> (Vec<Fp3>, Fp3) {
    let coef = q[rho..].iter().fold(scale, |a, &x| a * x);
    (q[..rho].iter().copied().chain(core::iter::repeat_n(Fp3::ONE, q.len() - rho)).collect(), coef)
}

fn coins_with(rng: &mut Rng, rho: usize, random_psi: bool) -> Coins {
    let q = rng.point(M);
    let a0 = rng.e();
    let z_psi = rng.e();
    let mu = rng.e();
    let psi = block_term(&pow_point(z_psi, M), rho, Fp3::ONE);
    let mut weight = Weight { eq: vec![(q.clone(), a0), block_term(&q, rho, -a0)], explicit: vec![] };
    let block_start = (1 << M) - (1 << rho);
    let xi: Vec<Fp3> = if random_psi { (0..1 << rho).map(|_| rng.e()).collect() } else { vec![] };
    if random_psi {
        for (k, &x) in xi.iter().enumerate() {
            weight.explicit.push((block_start + k, mu * x));
        }
    } else {
        weight.eq.push((psi.0.clone(), mu * psi.1));
    }
    // explicit entries off the block (the Libra slots' weights)
    for i in 0..6 {
        weight.explicit.push((block_start - 1 - 3 * i, rng.e()));
    }
    let mut leaves: Vec<usize> = Vec::new();
    while leaves.len() < Q0 {
        let j = (rng.u64() as usize) & ((1 << (LOG_DOMAIN as usize - K0)) - 1);
        if !leaves.contains(&j) {
            leaves.push(j);
        }
    }
    leaves.sort_unstable();
    Coins {
        beta: rng.e(),
        z_ood: rng.e(),
        gamma: rng.e(),
        alphas: rng.point(K0),
        leaves,
        psi,
        xi,
        weight,
    }
}

/// Round-0 messages as base limbs, and `f_1`'s coefficients.
fn messages(c: &[Goldilocks], g: &[Fp3], co: &Coins, rho: usize) -> (Vec<Goldilocks>, Vec<Fp3>) {
    let mut out: Vec<Fp3> = Vec::new();
    let block = &c[(1 << M) - (1 << rho)..];
    out.push(if co.xi.is_empty() {
        co.psi.1 * ml_eval_base(block, &co.psi.0[..rho])
    } else {
        co.xi.iter().zip(block).fold(Fp3::ZERO, |a, (&x, &v)| a + x * Fp3::from_base(v))
    });
    out.push(co.weight.apply_lifted(g, K0));
    let mut cc = c.to_vec();
    mobius_base(&mut cc);
    let mut gc = g.to_vec();
    mobius_ext(&mut gc);
    let zk = (0..K0).fold(co.z_ood, |a, _| a * a);
    out.push(univariate_base(&cc, co.z_ood) + co.beta * univariate_ext(&gc, zk));
    // sumcheck on h·w, w = Λ + γ·eq(pow(z_ood))
    let mut f: Vec<Fp3> = (0..1 << M).map(|b| Fp3::from_base(c[b]) + co.beta * g[b >> K0]).collect();
    let mut w = co.weight.table(M);
    add_scaled_eq(&mut w, &pow_point(co.z_ood, M), co.gamma);
    let two = Fp3::from_base(Goldilocks::new(2));
    for &a in &co.alphas {
        let (mut h0, mut h2) = (Fp3::ZERO, Fp3::ZERO);
        for i in 0..f.len() / 2 {
            h0 += f[2 * i] * w[2 * i];
            h2 += (two * f[2 * i + 1] - f[2 * i]) * (two * w[2 * i + 1] - w[2 * i]);
        }
        out.push(h0);
        out.push(h2);
        fold_evals_ext(&mut f, a);
        fold_evals_ext(&mut w, a);
    }
    let mut limbs: Vec<Goldilocks> = out.iter().flat_map(|x| [x.c0, x.c1, x.c2]).collect();
    // opened cosets of c and Ĝ at the shift points
    let cw = encode_base(&cc, LOG_DOMAIN);
    let gw = encode_ext(&gc, LOG_DOMAIN - K0 as u32);
    let leaves = 1usize << (LOG_DOMAIN as usize - K0);
    for &j in &co.leaves {
        limbs.extend((0..1 << K0).map(|t| cw[j + t * leaves]));
        limbs.extend([gw[j].c0, gw[j].c1, gw[j].c2]);
    }
    // f_1 = Fold(h): coefficients folded on the low k₀ variables
    let mut h: Vec<Fp3> = cc.iter().map(|&x| Fp3::from_base(x)).collect();
    for (i, &x) in gc.iter().enumerate() {
        h[i << K0] += co.beta * x;
    }
    for &a in &co.alphas {
        h = (0..h.len() / 2).map(|i| h[2 * i] + a * h[2 * i + 1]).collect();
    }
    (limbs, h)
}

/// `G` whose `Ĝ` coefficients are `−Fold(Δc)/β`, for `Δc` a base table.
fn compensation(dc: &[Goldilocks], co: &Coins) -> Vec<Fp3> {
    let mut cc = dc.to_vec();
    mobius_base(&mut cc);
    let mut h: Vec<Fp3> = cc.iter().map(|&x| Fp3::from_base(x)).collect();
    for &a in &co.alphas {
        h = (0..h.len() / 2).map(|i| h[2 * i] + a * h[2 * i + 1]).collect();
    }
    let s = -co.beta.inv();
    let mut coeffs: Vec<Fp3> = h.into_iter().map(|x| s * x).collect();
    // coefficients → table (zeta transform)
    let n = coeffs.len();
    let mut step = 1;
    while step < n {
        for blk in coeffs.chunks_exact_mut(2 * step) {
            let (lo, hi) = blk.split_at_mut(step);
            for (h, l) in hi.iter_mut().zip(lo.iter()) {
                *h += *l;
            }
        }
        step *= 2;
    }
    coeffs
}

/// Solve `A x = b` over F_p; `None` if inconsistent. Returns `(x, rank)`.
fn solve(mut a: Vec<Vec<Goldilocks>>, mut b: Vec<Goldilocks>, cols: usize) -> (Option<Vec<Goldilocks>>, usize) {
    let rows = a.len();
    let mut piv = Vec::new();
    let mut r = 0;
    for col in 0..cols {
        let Some(p) = (r..rows).find(|&i| a[i][col] != Goldilocks::ZERO) else { continue };
        a.swap(r, p);
        b.swap(r, p);
        let inv = a[r][col].inv();
        for x in &mut a[r][col..] {
            *x *= inv;
        }
        b[r] *= inv;
        let pivot = a[r][col..].to_vec();
        let pb = b[r];
        for (i, (row, bi)) in a.iter_mut().zip(b.iter_mut()).enumerate() {
            if i != r && row[col] != Goldilocks::ZERO {
                let f = row[col];
                for (x, &v) in row[col..].iter_mut().zip(&pivot) {
                    *x -= f * v;
                }
                *bi -= f * pb;
            }
        }
        piv.push(col);
        r += 1;
    }
    if (r..rows).any(|i| b[i] != Goldilocks::ZERO) {
        return (None, r);
    }
    let mut x = vec![Goldilocks::ZERO; cols];
    for (i, &col) in piv.iter().enumerate() {
        x[col] = b[i];
    }
    (Some(x), r)
}

/// Run the lemma's system for block size `2^rho`; returns `(solved, rank,
/// rows that must be matched)`.
fn run(seed: u64, rho: usize) -> (bool, usize, usize) {
    run_with(seed, rho, false)
}

fn run_with(seed: u64, rho: usize, random_psi: bool) -> (bool, usize, usize) {
    let mut rng = Rng::new(seed);
    let co = coins_with(&mut rng, rho, random_psi);
    let n = 1usize << M;
    let start = n - (1 << rho);
    let c = rng.evals(M);
    let g: Vec<Fp3> = (0..n >> K0).map(|_| rng.e()).collect();
    // Δ off the block with F'(Δ) = 0
    let d = kernel_delta(&co, rho, seed ^ 0xD17A);
    let (base, f1) = messages(&c, &g, &co, rho);
    let at = |x: &[Goldilocks]| {
        let mut dc = d.clone();
        dc[start..].copy_from_slice(x);
        let cc: Vec<Goldilocks> = c.iter().zip(&dc).map(|(&a, &b)| a + b).collect();
        let dg = compensation(&dc, &co);
        let gg: Vec<Fp3> = g.iter().zip(&dg).map(|(&a, &b)| a + b).collect();
        messages(&cc, &gg, &co, rho)
    };
    let zero = vec![Goldilocks::ZERO; 1 << rho];
    let (m0, f10) = at(&zero);
    assert_eq!(f10, f1, "ΔG keeps f_1");
    let rhs: Vec<Goldilocks> = base.iter().zip(&m0).map(|(&a, &b)| a - b).collect();
    let mut a = vec![vec![Goldilocks::ZERO; 1 << rho]; base.len()];
    for k in 0..1 << rho {
        let mut e = zero.clone();
        e[k] = Goldilocks::ONE;
        let (mk, _) = at(&e);
        for (row, (&v, &v0)) in mk.iter().zip(&m0).enumerate() {
            a[row][k] = v - v0;
        }
    }
    let needed = a.iter().filter(|r| r.iter().any(|&v| v != Goldilocks::ZERO)).count();
    let (x, rank) = solve(a, rhs, 1 << rho);
    let Some(x) = x else { return (false, rank, needed) };
    let (mx, f1x) = at(&x);
    assert_eq!(mx, base, "every round-0 message kept");
    assert_eq!(f1x, f1, "f_1 kept");
    (true, rank, needed)
}

/// Rank the lemma needs: every opened coset value of `c` and every Fp3
/// message but one (`σ_{k₀}` is fixed by `f_1`, an automatic relation).
fn expected_rank() -> usize {
    Q0 * (1 << K0) + 3 * (3 + 2 * K0 - 1)
}

#[test]
fn a_geometric_block_weight_leaks() {
    // attempt 1 (`docs/explanation/zk-small.md`): Ψ = eq(pow(z_ψ), ·) on
    // the block. The sumcheck's block parts telescope onto T(P) and T(α)
    // at each of the two points, so u_ψ, u and the OOD answer add one
    // dimension instead of three: a combination of revealed values is a
    // function of the witness alone.
    for seed in 0..3 {
        let (ok, rank, _) = run(seed, 8);
        assert!(!ok, "seed {seed}");
        assert_eq!(rank, expected_rank() - 3, "one Fp3 short");
    }
}

#[test]
fn random_block_weights_mask_every_round0_message() {
    // the shipped design: Ψ = Σ ξ_y·[· = y] over the block, ξ uniform
    for seed in 0..6 {
        let (ok, rank, _) = run_with(seed, 8, true);
        assert!(ok, "seed {seed}: rank {rank}");
        assert_eq!(rank, expected_rank());
    }
}

#[test]
fn below_the_bound_the_masks_do_not_suffice() {
    // R = 128: D = 8 < q₀ + 6 — more round-0 messages than block freedom
    let (ok, rank, needed) = run_with(11, 7, true);
    assert!(!ok, "rank {rank} of {needed}");
}

/// A random change off the block with `F'(Δ) = 0`.
fn kernel_delta(co: &Coins, rho: usize, seed: u64) -> Vec<Goldilocks> {
    let mut rng = Rng::new(seed);
    let n = 1usize << M;
    let start = n - (1 << rho);
    let fprime = Weight { eq: co.weight.eq[..2].to_vec(), explicit: co.weight.explicit.iter().copied().filter(|&(b, _)| b < (1 << M) - (1 << rho)).collect() };
    let mut d: Vec<Goldilocks> = (0..n).map(|i| if i < start { rng.g() } else { Goldilocks::ZERO }).collect();
    let slots = [5usize, 77, 301];
    for &s in &slots {
        d[s] = Goldilocks::ZERO;
    }
    let target = fprime.apply_base(&d);
    let cols: Vec<Fp3> = slots
        .iter()
        .map(|&s| {
            let mut e = vec![Goldilocks::ZERO; n];
            e[s] = Goldilocks::ONE;
            fprime.apply_base(&e)
        })
        .collect();
    let a3: Vec<Vec<Goldilocks>> = (0..3).map(|l| cols.iter().map(|x| [x.c0, x.c1, x.c2][l]).collect()).collect();
    let (sol, _) = solve(a3, vec![-target.c0, -target.c1, -target.c2], 3);
    for (k, &s) in slots.iter().enumerate() {
        d[s] = sol.as_ref().expect("F' slots invertible")[k];
    }
    assert_eq!(fprime.apply_base(&d), Fp3::ZERO);
    d
}


/// The model above is the prover, and the rank check's rows are the
/// model's derivative: one real `open_zk` at this file's shape, its coins
/// read back, then (1) every round-0 message the model computes equals the
/// proof's, (2) the model's change per block slot (with `Ĝ` absorbing it)
/// equals `check::rows` entry by entry.
#[test]
fn the_rank_check_is_the_provers_own_round0() {
    let mut rng = Rng::new(42);
    let p = WhirParams {
        log_inv_rate: (LOG_DOMAIN as usize - M) as u8,
        folding_factor: K0 as u8,
        pow_bits: 0,
        security_target: 16,
        max_final_vars: (M - K0) as u8,
        ..WhirParams::default()
    };
    let cfg = WhirConfig::derive(&p, M).unwrap();
    let spec0 = cfg.rounds[0];
    assert_eq!((spec0.fold, spec0.ood, cfg.rounds.len()), (K0, 1, 1), "one fold, one OOD sample");
    let rho = (((spec0.queries + 6) << K0).next_power_of_two()).trailing_zeros() as usize;
    assert!(rho < M, "queries {}", spec0.queries);
    let n = 1usize << M;
    let start = n - (1 << rho);
    let c = rng.evals(M);
    let g: Vec<Fp3> = (0..n >> K0).map(|_| rng.e()).collect();
    let mask = ZkMask {
        g: g.clone(),
        salts: (0..SALT_LIMBS << spec0.log_leaves()).map(|_| rng.g()).collect(),
        block: Some(rho),
    };
    let (_, data) = commit_zk(&p, &c, mask).unwrap();
    let q = rng.point(M);
    let a0 = rng.e();
    let mut weight = Weight { eq: vec![(q.clone(), a0), block_term(&q, rho, -a0)], explicit: vec![] };
    let xi: Vec<Fp3> = (0..1 << rho).map(|_| rng.e()).collect();
    weight.explicit.extend(xi.iter().enumerate().map(|(k, &x)| (start + k, x)));
    let (vals, proof) = match open_zk(&p, &data, &weight, &mut Transcript::new(b"lemma-real")).unwrap() {
        ZkOutcome::Opened(v, pr) => (v, pr),
        ZkOutcome::Resample => panic!("resample at seed 42"),
    };
    let (beta, coins0) = super::LAST_COINS.with(|l| l.borrow().clone()).expect("coins");
    let co = Coins {
        beta,
        z_ood: coins0.ood[0],
        gamma: coins0.gamma,
        alphas: coins0.alphas.clone(),
        leaves: coins0.leaves.clone(),
        psi: (vec![], Fp3::ZERO),
        xi: xi.clone(),
        weight: weight.clone(),
    };
    // (1) the model's messages are the proof's
    let (limbs, _) = messages(&c, &g, &co, rho);
    let e = 3 + 2 * K0;
    let model: Vec<Fp3> = limbs[..3 * e].chunks_exact(3).map(|x| Fp3::new(x[0], x[1], x[2])).collect();
    assert_eq!(model[1], vals.u, "u");
    assert_eq!(model[2], proof.ood0[0], "OOD answer");
    assert_eq!(&model[3..], &proof.sumcheck0[..], "round-0 sumcheck values");
    let Symbols::Base(sym) = &proof.final_opening.symbols else { panic!("base round-0 symbols") };
    let width = (1 << K0) + EXTRA;
    let cos = &limbs[3 * e..];
    for (leaf, (opened, modeled)) in sym.chunks_exact(width).zip(cos.chunks_exact((1 << K0) + 3)).enumerate() {
        assert_eq!(&opened[..(1 << K0) + 3], modeled, "leaf {leaf}: coset and Ĝ");
    }
    // (2) the check's rows are the model's derivative
    let rows = check::rows(&spec0, M, rho, &xi, beta, &coins0);
    let zero = vec![Goldilocks::ZERO; n];
    let (base, _) = messages(&c, &g, &co, rho);
    for k in 0..1 << rho {
        let mut dc = zero.clone();
        dc[start + k] = Goldilocks::ONE;
        let cc: Vec<Goldilocks> = c.iter().zip(&dc).map(|(&a, &b)| a + b).collect();
        let dg = compensation(&dc, &co);
        let gg: Vec<Fp3> = g.iter().zip(&dg).map(|(&a, &b)| a + b).collect();
        let (mk, _) = messages(&cc, &gg, &co, rho);
        for (r, row) in rows.iter().enumerate() {
            let d = Fp3::new(mk[3 * r] - base[3 * r], mk[3 * r + 1] - base[3 * r + 1], mk[3 * r + 2] - base[3 * r + 2]);
            assert_eq!(row[k], d, "row {r}, block slot {k}");
        }
    }
}
