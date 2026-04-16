// ---
// tags: trop, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Tropical determinant (minimum-weight perfect matching).
//!
//! trop_det(A) = min over all permutations sigma of sum_i A\[i\]\[sigma(i)\].
//!
//! This is equivalent to the minimum-weight perfect matching in a bipartite
//! graph, solvable in O(n^3) by the Hungarian algorithm. Here we implement
//! the naive O(n!) enumeration for n <= 10.

use crate::element::Tropical;
use crate::matrix::TropMatrix;

/// Maximum dimension for the naive determinant algorithm.
const MAX_NAIVE_DIM: usize = 10;

/// Compute the tropical determinant of a square matrix.
///
/// trop_det(A) = min_{sigma in S_n} sum_i A\[i\]\[sigma(i)\]
///
/// # Panics
/// Panics if `n > 10` (use the Hungarian algorithm for larger matrices).
pub fn determinant(a: &TropMatrix) -> Tropical {
    let n = a.n;
    if n == 0 {
        return Tropical::ONE; // empty product = multiplicative identity
    }
    assert!(
        n <= MAX_NAIVE_DIM,
        "determinant: n={} exceeds naive limit {}; use Hungarian algorithm",
        n,
        MAX_NAIVE_DIM
    );

    let mut perm: [usize; MAX_NAIVE_DIM] = [0; MAX_NAIVE_DIM];
    for (i, elem) in perm.iter_mut().take(n).enumerate() {
        *elem = i;
    }

    let mut best = Tropical::INF;
    permutation_search(a, &mut perm, n, n, &mut best);
    best
}

/// Evaluate the cost of the current permutation and update best.
fn eval_perm(a: &TropMatrix, perm: &[usize], n: usize, best: &mut Tropical) {
    let mut cost = Tropical::ONE; // 0, the multiplicative identity
    for (i, &col) in perm.iter().enumerate().take(n) {
        cost = cost.mul(a.get(i, col));
        if cost.is_inf() {
            return; // early exit: can't improve
        }
    }
    *best = best.add(cost);
}

/// Generate all permutations via Heap's algorithm and evaluate each.
fn permutation_search(
    a: &TropMatrix,
    perm: &mut [usize; 10],
    k: usize,
    n: usize,
    best: &mut Tropical,
) {
    if k == 1 {
        eval_perm(a, perm, n, best);
        return;
    }
    for i in 0..k {
        permutation_search(a, perm, k - 1, n, best);
        if k.is_multiple_of(2) {
            perm.swap(i, k - 1);
        } else {
            perm.swap(0, k - 1);
        }
    }
}
