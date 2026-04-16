// ---
// tags: trop, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Kleene star (shortest-path closure) for tropical matrices.
//!
//! Computes A* = I + A + A^2 + ... + A^(n-1) using the
//! Floyd-Warshall algorithm, which is exactly the Kleene star
//! in the (min, +) semiring. Runs in O(n^3).

use crate::element::Tropical;
use crate::matrix::{MAX_DIM, TropMatrix};

/// Compute the Kleene star of a tropical matrix.
///
/// A* = I (min) A (min) A^2 (min) ... (min) A^(n-1)
///
/// Equivalently, A*\[i\]\[j\] is the shortest-path distance from i to j
/// in the weighted digraph represented by A.
///
/// Uses Floyd-Warshall: O(n^3) time, O(n^2) space.
pub fn kleene_star(a: &TropMatrix) -> TropMatrix {
    let n = a.n;
    // Initialize d = I (min) A: diagonal gets min(0, A[i][i]), off-diagonal = A[i][j].
    let mut d = [Tropical::INF; MAX_DIM * MAX_DIM];
    for i in 0..n {
        for j in 0..n {
            d[i * MAX_DIM + j] = a.get(i, j);
        }
        // Merge with identity: diagonal entry = min(current, 0)
        let idx = i * MAX_DIM + i;
        d[idx] = d[idx].add(Tropical::ONE);
    }

    // Floyd-Warshall relaxation
    for k in 0..n {
        for i in 0..n {
            let d_ik = d[i * MAX_DIM + k];
            if d_ik.is_inf() {
                continue;
            }
            for j in 0..n {
                let d_kj = d[k * MAX_DIM + j];
                let candidate = d_ik.mul(d_kj);
                let idx = i * MAX_DIM + j;
                d[idx] = d[idx].add(candidate);
            }
        }
    }

    let mut result = TropMatrix::new(n);
    result.data = d;
    result
}
