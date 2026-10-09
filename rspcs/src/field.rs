//! Goldilocks / Fp3 helpers: eq tables, multilinear and univariate
//! evaluation, Möbius transform, coefficient folding.
//!
//! Variable order everywhere: coordinate `k` of a point is bit `k` of a
//! table index (the `MultilinearPoly` convention of lens core). The
//! univariate polynomial of a multilinear `f` with monomial coefficients
//! `c` is `f̂(X) = Σ_i c_i X^i`, so `f̂(z) = f(z, z², z⁴, …)`.

use nebu::{Fp3, Goldilocks};

/// `a · b` for `a ∈ Fp3`, `b ∈ F_p` (three base multiplications).
#[inline]
pub fn mul_base(a: Fp3, b: Goldilocks) -> Fp3 {
    Fp3::new(a.c0 * b, a.c1 * b, a.c2 * b)
}

/// `1/2` in Goldilocks.
#[inline]
pub fn half() -> Goldilocks {
    Goldilocks::new(2).inv()
}

/// Generator of the multiplicative subgroup of order `2^log_n`.
pub fn root_of_unity(log_n: u32) -> Goldilocks {
    assert!(log_n <= 32, "Goldilocks has 2-adicity 32");
    Goldilocks::new(7).exp((nebu::field::P - 1) >> log_n)
}

/// `eq(point, b)` for every `b ∈ {0,1}^len`, bit `k` of the index ↔
/// `point[k]`.
pub fn eq_table(point: &[Fp3]) -> Vec<Fp3> {
    let mut table = Vec::with_capacity(1 << point.len());
    table.push(Fp3::ONE);
    for &r in point {
        let len = table.len();
        let one_minus = Fp3::ONE - r;
        table.extend_from_within(..);
        for i in 0..len {
            let t = table[i];
            table[i] = t * one_minus;
            table[i + len] = t * r;
        }
    }
    table
}

/// Add `scale · eq(point, ·)` into `acc` (`acc.len() = 2^point.len()`).
pub fn add_scaled_eq(acc: &mut [Fp3], point: &[Fp3], scale: Fp3) {
    debug_assert_eq!(acc.len(), 1 << point.len());
    let mut table = eq_table(point);
    if scale != Fp3::ONE {
        for t in &mut table {
            *t *= scale;
        }
    }
    for (a, t) in acc.iter_mut().zip(table) {
        *a += t;
    }
}

/// `eq(a, b) = Π (a_i b_i + (1 − a_i)(1 − b_i))`.
pub fn eq_eval(a: &[Fp3], b: &[Fp3]) -> Fp3 {
    assert_eq!(a.len(), b.len());
    let mut acc = Fp3::ONE;
    for (&x, &y) in a.iter().zip(b) {
        acc *= x * y + (Fp3::ONE - x) * (Fp3::ONE - y);
    }
    acc
}

/// Multilinear evaluation of a base-field evaluation table at an Fp3
/// point.
pub fn ml_eval_base(evals: &[Goldilocks], point: &[Fp3]) -> Fp3 {
    assert_eq!(evals.len(), 1 << point.len());
    if point.is_empty() {
        return Fp3::from_base(evals[0]);
    }
    let r = point[0];
    let mut cur: Vec<Fp3> = evals
        .chunks_exact(2)
        .map(|p| Fp3::from_base(p[0]) + mul_base(r, p[1] - p[0]))
        .collect();
    fold_evals_in_place(&mut cur, &point[1..]);
    cur[0]
}

/// Multilinear evaluation of an Fp3 evaluation table at an Fp3 point.
pub fn ml_eval_ext(evals: &[Fp3], point: &[Fp3]) -> Fp3 {
    assert_eq!(evals.len(), 1 << point.len());
    let mut cur = evals.to_vec();
    fold_evals_in_place(&mut cur, point);
    cur[0]
}

fn fold_evals_in_place(cur: &mut Vec<Fp3>, point: &[Fp3]) {
    for &r in point {
        let half = cur.len() / 2;
        for i in 0..half {
            let a = cur[2 * i];
            let b = cur[2 * i + 1];
            cur[i] = a + r * (b - a);
        }
        cur.truncate(half);
    }
}

/// Evaluation table → monomial coefficients (Möbius transform over the
/// subset lattice), in place.
pub fn mobius_base(a: &mut [Goldilocks]) {
    let n = a.len();
    assert!(n.is_power_of_two());
    let mut step = 1;
    while step < n {
        for block in a.chunks_exact_mut(2 * step) {
            let (lo, hi) = block.split_at_mut(step);
            for (h, l) in hi.iter_mut().zip(lo.iter()) {
                *h -= *l;
            }
        }
        step *= 2;
    }
}

/// Multilinear evaluation of a monomial-coefficient table at a point:
/// `f(x) = f_even(x_2..) + x_1 · f_odd(x_2..)`.
pub fn coeff_ml_eval(coeffs: &[Fp3], point: &[Fp3]) -> Fp3 {
    assert_eq!(coeffs.len(), 1 << point.len());
    let mut cur = coeffs.to_vec();
    for &r in point {
        let half = cur.len() / 2;
        for i in 0..half {
            cur[i] = cur[2 * i] + r * cur[2 * i + 1];
        }
        cur.truncate(half);
    }
    cur[0]
}

/// Fold the first variable of a base-field coefficient table:
/// `c'_i = c_{2i} + α · c_{2i+1}`.
pub fn fold_coeffs_base(c: &[Goldilocks], alpha: Fp3) -> Vec<Fp3> {
    c.chunks_exact(2)
        .map(|p| Fp3::from_base(p[0]) + mul_base(alpha, p[1]))
        .collect()
}

/// Fold the first variable of an Fp3 coefficient table in place.
pub fn fold_coeffs_ext(c: &mut Vec<Fp3>, alpha: Fp3) {
    let half = c.len() / 2;
    for i in 0..half {
        c[i] = c[2 * i] + alpha * c[2 * i + 1];
    }
    c.truncate(half);
}

/// Fold the first variable of an Fp3 evaluation table in place:
/// `e'_i = e_{2i} + α (e_{2i+1} − e_{2i})`.
pub fn fold_evals_ext(e: &mut Vec<Fp3>, alpha: Fp3) {
    let half = e.len() / 2;
    for i in 0..half {
        let a = e[2 * i];
        e[i] = a + alpha * (e[2 * i + 1] - a);
    }
    e.truncate(half);
}

/// `Σ c_i z^i` (Horner) for Fp3 coefficients at an Fp3 point.
pub fn univariate_ext(coeffs: &[Fp3], z: Fp3) -> Fp3 {
    coeffs.iter().rev().fold(Fp3::ZERO, |acc, &c| acc * z + c)
}

/// `Σ c_i z^i` for base coefficients at an Fp3 point.
pub fn univariate_base(coeffs: &[Goldilocks], z: Fp3) -> Fp3 {
    coeffs
        .iter()
        .rev()
        .fold(Fp3::ZERO, |acc, &c| acc * z + Fp3::from_base(c))
}

/// `Σ c_i x^i` for Fp3 coefficients at a base point.
pub fn univariate_ext_at_base(coeffs: &[Fp3], x: Goldilocks) -> Fp3 {
    coeffs
        .iter()
        .rev()
        .fold(Fp3::ZERO, |acc, &c| mul_base(acc, x) + c)
}

/// `(z, z², z⁴, …, z^{2^{m−1}})`.
pub fn pow_point(z: Fp3, m: usize) -> Vec<Fp3> {
    let mut out = Vec::with_capacity(m);
    let mut cur = z;
    for _ in 0..m {
        out.push(cur);
        cur = cur * cur;
    }
    out
}

/// Interpolate the degree-≤2 polynomial through `(0,h0) (1,h1) (2,h2)`
/// and evaluate at `x`.
pub fn quadratic_at(h0: Fp3, h1: Fp3, h2: Fp3, x: Fp3) -> Fp3 {
    let one = Fp3::ONE;
    let two = Fp3::from_base(Goldilocks::new(2));
    let inv2 = half();
    let l0 = mul_base((x - one) * (x - two), inv2);
    let l1 = -(x * (x - two));
    let l2 = mul_base(x * (x - one), inv2);
    h0 * l0 + h1 * l1 + h2 * l2
}

/// Canonical u64 of a base element.
#[inline]
pub fn canon(x: Goldilocks) -> u64 {
    x.as_u64()
}

/// Fp3 equality on canonical limbs.
#[inline]
pub fn fp3_eq(a: Fp3, b: Fp3) -> bool {
    a.c0.as_u64() == b.c0.as_u64()
        && a.c1.as_u64() == b.c1.as_u64()
        && a.c2.as_u64() == b.c2.as_u64()
}
