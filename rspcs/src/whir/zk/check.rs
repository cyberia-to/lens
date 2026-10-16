//! The prover-side rank check of the round-0 masking lemma (zheng
//! `docs/explanation/zk-small.md` § the design).
//!
//! The caller names the masking block `[2^n − R, 2^n)`; after the opening
//! the prover knows every round-0 coin. Each round-0 Fp3 message — `Λ` on
//! the block (the caller's `u_ψ` row), `u = Λ(g)`, the OOD answers of
//! `h`, the `2k₀` sumcheck values — is a linear form in the block once
//! `ΔG = −Fold(Δc)/β` keeps `f_1`; the opened cosets fix `ρ_j(y_t)` for
//! every sub-polynomial `ρ_j` (low `k₀` bits). The check reduces every
//! message modulo the cosets — on the basis `Z·Y^i`, `Z = Π_t (Y − y_t)`,
//! of each `ρ_j`'s kernel — and requires rank `3·(E − 1)` over `F_p` (one
//! relation, `σ_{k₀}` fixed by `f_1`, is automatic). A proof whose coins
//! fail is never output: the prover recommits with fresh masks and salts.
//! The event depends on public coins only, so a simulator conditions the
//! same way and the zero-knowledge argument needs no bound on it.

use nebu::{Fp3, Goldilocks};

use super::super::config::RoundSpec;
use super::super::fold::shift_point;
use super::super::prover::Round0Coins;
use crate::field::{eq_table, fold_evals_ext, pow_point};

fn eq_bits(p: &[Fp3], b: usize) -> Fp3 {
    p.iter().enumerate().fold(Fp3::ONE, |a, (l, &x)| if (b >> l) & 1 == 1 { a * x } else { a * (Fp3::ONE - x) })
}

/// `Λ` on the block, `R = 2^rho` entries (`lambda(y)` for `y = 2^n − R + k`).
pub(crate) fn rank_ok(spec0: &RoundSpec, n: usize, rho: usize, lambda: &[Fp3], beta: Fp3, c: &Round0Coins) -> bool {
    let k0 = spec0.fold;
    let big_r = 1usize << rho;
    let bs = (1usize << n) - big_r;
    let low = (1usize << k0) - 1;
    let d = big_r >> k0;
    let q = c.leaves.len();
    if rho < k0 || d <= q {
        return false;
    }
    let eq_alpha = eq_table(&c.alphas);
    let binv = beta.inv();
    // Σ over the low bits of Λ at each high position of the block
    let lam_hi: Vec<Fp3> = (0..big_r >> k0).map(|h| (0..1 << k0).fold(Fp3::ZERO, |a, l| a + lambda[(h << k0) | l])).collect();
    // OOD weights on the block, and h's two-part answer
    let pows: Vec<Vec<Fp3>> = c.ood.iter().map(|&z| pow_point(z, n)).collect();
    let mut w = lambda.to_vec();
    let mut g = c.gamma;
    for p in &pows {
        for (k, wk) in w.iter_mut().enumerate() {
            *wk += g * eq_bits(p, bs + k);
        }
        g *= c.gamma;
    }
    let ood_c: Vec<Vec<Fp3>> = pows.iter().map(|p| (0..big_r).map(|k| eq_bits(p, bs + k)).collect()).collect();
    let ood_g: Vec<Vec<Fp3>> = pows.iter().map(|p| (0..big_r >> k0).map(|h| eq_bits(&p[k0..], (bs >> k0) + h)).collect()).collect();
    let e = 2 + c.ood.len() + 2 * k0;
    let two = Fp3::from_base(Goldilocks::new(2));
    // rows[e][k]: message e on the block's evaluation basis vector k
    let mut rows = vec![vec![Fp3::ZERO; big_r]; e];
    for k in 0..big_r {
        let ea = eq_alpha[k & low];
        let kh = k >> k0;
        rows[0][k] = lambda[k];
        rows[1][k] = -(binv * ea * lam_hi[kh]);
        for (j, (oc, og)) in ood_c.iter().zip(&ood_g).enumerate() {
            rows[2 + j][k] = oc[k] - ea * og[kh];
        }
        let mut f = vec![Fp3::ZERO; big_r];
        f[k] = Fp3::ONE;
        for l in 0..1 << k0 {
            f[(kh << k0) | l] -= ea;
        }
        let mut ww = w.clone();
        let mut r = 2 + c.ood.len();
        for &a in &c.alphas {
            let (mut h0, mut h2) = (Fp3::ZERO, Fp3::ZERO);
            for i in 0..f.len() / 2 {
                h0 += f[2 * i] * ww[2 * i];
                h2 += (two * f[2 * i + 1] - f[2 * i]) * (two * ww[2 * i + 1] - ww[2 * i]);
            }
            rows[r][k] = h0;
            rows[r + 1][k] = h2;
            r += 2;
            fold_evals_ext(&mut f, a);
            fold_evals_ext(&mut ww, a);
        }
    }
    // evaluation basis → coefficient basis on the block: φ'[b] = Σ_{y ⊇ b} φ[y]
    for row in &mut rows {
        let mut step = 1;
        while step < big_r {
            for blk in row.chunks_exact_mut(2 * step) {
                let (lo, hi) = blk.split_at_mut(step);
                for (l, h) in lo.iter_mut().zip(hi.iter()) {
                    *l += *h;
                }
            }
            step *= 2;
        }
    }
    // Z = Π (Y − y_t) over the opened shift points
    let mut z = vec![Goldilocks::ONE];
    for &j in &c.leaves {
        let y = shift_point(spec0, j);
        let mut next = vec![Goldilocks::ZERO; z.len() + 1];
        for (i, &zi) in z.iter().enumerate() {
            next[i + 1] += zi;
            next[i] -= zi * y;
        }
        z = next;
    }
    // reduce: entry (j, i) = Σ_l Z_l·φ'[((l + i) << k0) | j]
    let cols = (1usize << k0) * (d - q);
    let mut m: Vec<Vec<Goldilocks>> = Vec::with_capacity(3 * e);
    for row in &rows {
        let mut red = vec![Fp3::ZERO; cols];
        for j in 0..1usize << k0 {
            for i in 0..d - q {
                red[j * (d - q) + i] = z.iter().enumerate().fold(Fp3::ZERO, |a, (l, &zl)| {
                    let v = row[((l + i) << k0) | j];
                    a + Fp3::new(v.c0 * zl, v.c1 * zl, v.c2 * zl)
                });
            }
        }
        for limb in 0..3 {
            m.push(red.iter().map(|x| [x.c0, x.c1, x.c2][limb]).collect());
        }
    }
    rank(m, cols) == 3 * (e - 1)
}

fn rank(mut a: Vec<Vec<Goldilocks>>, cols: usize) -> usize {
    let rows = a.len();
    let mut r = 0;
    for col in 0..cols {
        let Some(p) = (r..rows).find(|&i| a[i][col] != Goldilocks::ZERO) else { continue };
        a.swap(r, p);
        let inv = a[r][col].inv();
        let pivot: Vec<Goldilocks> = a[r][col..].iter().map(|&v| v * inv).collect();
        for row in a.iter_mut().skip(r + 1) {
            let f = row[col];
            if f != Goldilocks::ZERO {
                for (x, &v) in row[col..].iter_mut().zip(&pivot) {
                    *x -= f * v;
                }
            }
        }
        r += 1;
        if r == rows {
            break;
        }
    }
    r
}
