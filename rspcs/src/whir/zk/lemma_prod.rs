//! The masking lemma's rank condition at the shipped shapes (zheng's
//! device parameters at 2^14…2^17), on the block alone.
//!
//! Every round-0 message is linear in the block `r` (`lemma.rs` has the
//! whole-table version at a small size, with the off-block side). Here
//! the block-restricted matrix is built in closed form — a block slot `y`
//! enters `ĉ(x)` with weight `eq(pow(x), y)`, `Fold` with `eq(α, y_low)`
//! at `y_high` — and its rank must be `q₀'·2^{k₀} + 3·(E − 1)`, `q₀'` the
//! distinct opened leaves, `E = 2 + s₀ + 2k₀` Fp3 messages (one relation
//! automatic). A rank this test computes for uniform coins is a nonzero
//! minor, so the rank polynomial is not identically zero at this shape;
//! Schwartz–Zippel over the Fp3 coins bounds the failure probability
//! (ledger row, zheng `specs/soundness.md`).
use super::*;
use crate::field::{eq_table, fold_evals_ext, pow_point, root_of_unity};
use crate::tests::common::Rng;

/// Rank of the block-restricted round-0 system and the rank the lemma needs.
fn rank_at(params: &WhirParams, n: usize, seed: u64) -> (usize, usize, usize) {
    let mut rng = Rng::new(seed);
    let cfg = WhirConfig::derive(params, n).unwrap();
    let r0 = cfg.rounds[0];
    let (k0, s0) = (r0.fold, r0.ood);
    assert_eq!(s0, 1, "one OOD sample at these shapes");
    let rho = (((r0.queries + 3 * (1 + s0)) << k0).next_power_of_two()).trailing_zeros() as usize;
    let big_r = 1usize << rho;
    let mut leaves: Vec<usize> = (0..r0.queries).map(|_| (rng.u64() as usize) & ((1 << r0.log_leaves()) - 1)).collect();
    leaves.sort_unstable();
    leaves.dedup();
    let (beta, z_o, gamma, mu) = (rng.e(), rng.e(), rng.e(), rng.e());
    let alphas = rng.point(k0);
    let xi: Vec<Fp3> = (0..big_r).map(|_| rng.e()).collect();
    let po = pow_point(z_o, n);
    // block-local index k: global bs + k; low k₀ bits, high bits k >> k₀
    let top = |p: &[Fp3]| p[rho..].iter().fold(Fp3::ONE, |a, &x| a * x);
    let eq_po = {
        let t = top(&po);
        eq_table(&po[..rho]).into_iter().map(|e| e * t).collect::<Vec<_>>()
    };
    let w: Vec<Fp3> = (0..big_r).map(|k| mu * xi[k] + gamma * eq_po[k]).collect();
    let eq_alpha = eq_table(&alphas);
    let low = (1usize << k0) - 1;
    // Σ_{y_low} ξ at each high position
    let xi_hi: Vec<Fp3> = (0..big_r >> k0).map(|h| (0..1 << k0).fold(Fp3::ZERO, |a, l| a + xi[(h << k0) | l])).collect();
    let po_g = pow_point(po[k0], n - k0);
    let g_top = po_g[rho - k0..].iter().fold(Fp3::ONE, |a, &x| a * x);
    let eq_pog = eq_table(&po_g[..rho - k0]);
    let msgs = |k: usize| -> Vec<Fp3> {
        let ea = eq_alpha[k & low];
        let kh = k >> k0;
        let mut out = vec![xi[k]];
        out.push(-(mu * beta.inv()) * ea * xi_hi[kh]);
        out.push(eq_po[k] - ea * eq_pog[kh] * g_top);
        // sumcheck on the block: Δh = δ_k − eq(α, k_low)·[y_high = k_high]
        let mut f: Vec<Fp3> = (0..big_r).map(|y| if y == k { Fp3::ONE } else { Fp3::ZERO }).collect();
        for l in 0..1 << k0 {
            f[(kh << k0) | l] -= ea;
        }
        let mut ww = w.clone();
        let two = Fp3::from_base(Goldilocks::new(2));
        for &a in &alphas {
            let (mut h0, mut h2) = (Fp3::ZERO, Fp3::ZERO);
            for i in 0..f.len() / 2 {
                h0 += f[2 * i] * ww[2 * i];
                h2 += (two * f[2 * i + 1] - f[2 * i]) * (two * ww[2 * i + 1] - ww[2 * i]);
            }
            out.push(h0);
            out.push(h2);
            fold_evals_ext(&mut f, a);
            fold_evals_ext(&mut ww, a);
        }
        out
    };
    let e = 2 + s0 + 2 * k0;
    let omega = root_of_unity(r0.log_domain);
    let zeta = omega.exp(1u64 << r0.log_leaves());
    let points: Vec<Goldilocks> = leaves
        .iter()
        .flat_map(|&j| (0..1u64 << k0).map(move |t| omega.exp(j as u64) * zeta.exp(t)))
        .collect();
    let coset_rows: Vec<Vec<Goldilocks>> = points
        .iter()
        .map(|&x| {
            let p = pow_point(Fp3::from_base(x), n);
            let t = top(&p);
            eq_table(&p[..rho]).into_iter().map(|v| (v * t).c0).collect()
        })
        .collect();
    let cols: Vec<Vec<Fp3>> = (0..big_r).map(msgs).collect();
    let mut a: Vec<Vec<Goldilocks>> = coset_rows;
    for m in 0..e {
        for limb in 0..3 {
            a.push(cols.iter().map(|c| [c[m].c0, c[m].c1, c[m].c2][limb]).collect());
        }
    }
    let rank = rank_of(a);
    (rank, leaves.len() * (1 << k0) + 3 * (e - 1), rho)
}

fn rank_of(mut a: Vec<Vec<Goldilocks>>) -> usize {
    let rows = a.len();
    let cols = a[0].len();
    let mut r = 0;
    for col in 0..cols {
        let Some(p) = (r..rows).find(|&i| a[i][col] != Goldilocks::ZERO) else { continue };
        a.swap(r, p);
        let inv = a[r][col].inv();
        let pivot: Vec<Goldilocks> = a[r][col..].iter().map(|&v| v * inv).collect();
        let below: Vec<usize> = (r + 1..rows).filter(|&i| a[i][col] != Goldilocks::ZERO).collect();
        for i in below {
            let f = a[i][col];
            for (k, &v) in pivot.iter().enumerate() {
                a[i][col + k] -= f * v;
            }
        }
        r += 1;
        if r == rows {
            break;
        }
    }
    r
}

#[test]
fn the_rank_condition_holds_at_the_device_shapes() {
    for (n, k) in [(14usize, 4u8), (16, 4), (17, 5)] {
        let p = WhirParams { log_inv_rate: 6, folding_factor: k, pow_bits: 16, ..WhirParams::default() };
        for seed in 0..2 {
            let (rank, need, rho) = rank_at(&p, n, seed);
            println!("2^{n} k={k}: R = 2^{rho}, rank {rank}, lemma needs {need}");
            assert_eq!(rank, need, "2^{n} seed {seed}");
        }
    }
}
