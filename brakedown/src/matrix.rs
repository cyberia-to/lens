//! Matrix reshape and row-combination helpers for the tensor-Merkle PCS.
//!
//! The evaluation table of a ν-variable multilinear polynomial (length
//! N = 2^ν) is reshaped into a k1×k2 matrix U with k1 = 2^⌈ν/2⌉,
//! k2 = 2^⌊ν/2⌉ (k1 ≥ k2, both powers of two, k1·k2 = N — see
//! specs/scalar-field.md).
//!
//! Index convention (must match `MultilinearPoly::evaluate` and
//! `tensor::evaluate_small`, both of which treat bit j of an index as
//! variable j, least-significant bit first):
//!   row(i) = i & (k1 - 1)   — the low `row_vars` bits of i
//!   col(i) = i >> row_vars  — the remaining high bits of i
//!   U[row][col] = evals[col*k1 + row]
//!
//! so point = (r_row, r_col) splits as r_row = point[..row_vars] (variables
//! 0..row_vars, i.e. the row bits) and r_col = point[row_vars..] (the
//! remaining variables, i.e. the column bits) — "first" and "remaining"
//! exactly as specs/scalar-field.md describes.

use cyber_lens_core::Field;

/// Split ν variables into (row_vars, col_vars) with row_vars = ⌈ν/2⌉ ≥
/// col_vars, so k1 = 2^row_vars ≥ k2 = 2^col_vars and k1·k2 = 2^ν.
pub fn split_vars(num_vars: usize) -> (usize, usize) {
    let row_vars = num_vars.div_ceil(2);
    (row_vars, num_vars - row_vars)
}

/// Reshape the evaluation table into k1 rows of length k2, per the
/// row(i)/col(i) convention above.
pub fn reshape<F: Field + Copy>(evals: &[F], row_vars: usize) -> Vec<Vec<F>> {
    let k1 = 1usize << row_vars;
    assert!(
        evals.len().is_multiple_of(k1),
        "evaluation table length must be a multiple of k1"
    );
    let k2 = evals.len() / k1;
    let mut rows: Vec<Vec<F>> = (0..k1).map(|_| Vec::with_capacity(k2)).collect();
    for (i, &v) in evals.iter().enumerate() {
        rows[i & (k1 - 1)].push(v);
    }
    rows
}

/// The multilinear equality table: `table[x] = eq(r, x)` for all x in
/// `0..2^r.len()`, built by doubling (`table[x] = Π_j ((x>>j)&1==1 ? r[j]
/// : 1-r[j])`) — the same basis `MultilinearPoly::evaluate` uses,
/// but for every boolean point at once rather than a single dot product.
pub fn eq_table<F: Field + Copy>(r: &[F]) -> Vec<F> {
    let mut table = vec![F::ONE];
    for &rj in r {
        let half = table.len();
        let mut next = vec![F::ZERO; half * 2];
        for (idx, &base) in table.iter().enumerate() {
            next[idx] = base * (F::ONE - rj);
            next[idx + half] = base * rj;
        }
        table = next;
    }
    table
}

/// v[col] = Σ_row weights[row] · rows[row][col] — the row-combination of
/// U using the eq-table of r_row. `weights.len()` must equal `rows.len()`.
pub fn row_combination<F: Field + Copy>(rows: &[Vec<F>], weights: &[F]) -> Vec<F> {
    assert_eq!(rows.len(), weights.len());
    let k2 = rows.first().map_or(0, Vec::len);
    let mut v = vec![F::ZERO; k2];
    for (row, &w) in rows.iter().zip(weights) {
        if w == F::ZERO {
            continue;
        }
        for (col, &u) in row.iter().enumerate() {
            v[col] += w * u;
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use nebu::Goldilocks;

    #[test]
    fn split_vars_balances_even() {
        assert_eq!(split_vars(2), (1, 1));
        assert_eq!(split_vars(4), (2, 2));
        assert_eq!(split_vars(6), (3, 3));
        assert_eq!(split_vars(8), (4, 4));
    }

    #[test]
    fn split_vars_favors_row_on_odd() {
        assert_eq!(split_vars(3), (2, 1));
        assert_eq!(split_vars(5), (3, 2));
    }

    #[test]
    fn reshape_round_trips_by_definition() {
        // N=8, row_vars=2 => k1=4, k2=2. i = col*k1 + row.
        let evals: Vec<Goldilocks> = (0..8).map(Goldilocks::new).collect();
        let rows = reshape(&evals, 2);
        assert_eq!(rows.len(), 4);
        for row in 0..4 {
            for col in 0..2 {
                let i = col * 4 + row;
                assert_eq!(rows[row][col], evals[i]);
            }
        }
    }

    #[test]
    fn eq_table_sums_to_one() {
        let r = [Goldilocks::new(7), Goldilocks::new(11), Goldilocks::new(3)];
        let table = eq_table(&r);
        assert_eq!(table.len(), 8);
        let sum: Goldilocks = table.iter().fold(Goldilocks::ZERO, |a, &b| a + b);
        assert_eq!(sum, Goldilocks::ONE);
    }

    #[test]
    fn eq_table_matches_direct_eq_at_each_boolean_point() {
        let r = [Goldilocks::new(5), Goldilocks::new(9)];
        let table = eq_table(&r);
        for x in 0..4usize {
            let bits: Vec<Goldilocks> = (0..2)
                .map(|j| {
                    if (x >> j) & 1 == 1 {
                        Goldilocks::ONE
                    } else {
                        Goldilocks::ZERO
                    }
                })
                .collect();
            let direct = crate::multilinear_eq(&r, &bits);
            assert_eq!(table[x], direct, "mismatch at x={x}");
        }
    }

    #[test]
    fn row_combination_matches_full_evaluation() {
        // f over 3 vars, evals[i] for i=0..8. row_vars=2 (k1=4,k2=2).
        let evals: Vec<Goldilocks> = (1..=8u64).map(Goldilocks::new).collect();
        let rows = reshape(&evals, 2);
        let r_row = [Goldilocks::new(3), Goldilocks::new(4)];
        let weights = eq_table(&r_row);
        let v = row_combination(&rows, &weights);
        assert_eq!(v.len(), 2);

        // Direct multilinear evaluation fixing the first two variables to
        // r_row and leaving the third free, evaluated at both boolean
        // values of the third variable, must match v[0], v[1].
        let full = crate::MultilinearPoly::new(evals);
        for (col, &expected) in v.iter().enumerate() {
            let mut point = r_row.to_vec();
            point.push(if col == 0 { Goldilocks::ZERO } else { Goldilocks::ONE });
            // point order must match evaluate()'s bit convention: bit0..bit(row_vars-1)
            // are the row vars, the remaining bit is the col var — same order used
            // by MultilinearPoly::evaluate (bit j <-> point[j]).
            assert_eq!(full.evaluate(&point), expected, "mismatch at col={col}");
        }
    }
}
