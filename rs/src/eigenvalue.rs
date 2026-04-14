// ---
// tags: trop, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Tropical eigenvalue (minimum mean cycle weight).
//!
//! The tropical eigenvalue of a matrix A is
//!   lambda = min over all elementary cycles C of (weight(C) / |C|).
//!
//! This implementation uses Karp's algorithm:
//!   1. Compute D\[k\]\[v\] = min-weight k-edge walk from source 0 to v, for k = 0..n.
//!   2. lambda = min_v max_{0 <= k < n} (D\[n\]\[v\] - D\[k\]\[v\]) / (n - k).
//!
//! Since we work in u64, we return the numerator and denominator separately
//! in a rational form, then produce the floor as a Tropical value.

use crate::element::Tropical;
use crate::matrix::{TropMatrix, MAX_DIM};

/// Compute the tropical eigenvalue (minimum mean cycle weight).
///
/// Returns `Tropical::INF` if the graph is acyclic (no cycles).
///
/// For graphs with cycles, returns `Tropical(floor(lambda))` where
/// lambda is the minimum mean cycle weight.
///
/// Uses Karp's algorithm: O(n^2 * n) = O(n^3).
pub fn eigenvalue(a: &TropMatrix) -> Tropical {
    let n = a.n;
    if n == 0 {
        return Tropical::INF;
    }

    // D[k][v] = min weight of a k-edge walk from vertex 0 to vertex v.
    // We store (n+1) rows of size MAX_DIM.
    // D[0][0] = 0, D[0][v] = INF for v != 0.
    let mut d = [[Tropical::INF; MAX_DIM]; MAX_DIM + 1];
    d[0][0] = Tropical::ONE; // weight 0

    for k in 0..n {
        for v in 0..n {
            if d[k][v].is_inf() {
                continue;
            }
            for w in 0..n {
                let edge = a.get(v, w);
                let candidate = d[k][v].mul(edge);
                d[k + 1][w] = d[k + 1][w].add(candidate);
            }
        }
    }

    // Karp's formula: lambda = min_v max_{k<n} (D[n][v] - D[k][v]) / (n - k)
    // We find the minimum rational value best_num / best_den.
    let mut best_num: u64 = u64::MAX;
    let mut best_den: u64 = 1;
    let mut found = false;

    for v in 0..n {
        if d[n][v].is_inf() {
            continue;
        }
        // For this v, compute max_{k<n} (D[n][v] - D[k][v]) / (n - k)
        // We want the maximum over k. Initialize to -infinity (skip).
        let mut worst_num: u64 = 0;
        let mut worst_den: u64 = 1;
        let mut v_valid = false;

        for k in 0..n {
            if d[k][v].is_inf() {
                continue;
            }
            let dn = d[n][v].as_u64();
            let dk = d[k][v].as_u64();
            if dn < dk {
                // Negative cycle contribution: (D[n][v] - D[k][v]) < 0
                // In tropical semiring over u64, this shouldn't happen in
                // standard use, but handle gracefully.
                continue;
            }
            let num = dn - dk;
            let den = (n - k) as u64;

            if !v_valid || num * worst_den > worst_num * den {
                worst_num = num;
                worst_den = den;
                v_valid = true;
            }
        }

        if v_valid {
            if !found || worst_num * best_den < best_num * worst_den {
                best_num = worst_num;
                best_den = worst_den;
                found = true;
            }
        }
    }

    if !found {
        return Tropical::INF;
    }

    // Return floor(best_num / best_den)
    Tropical::from_u64(best_num / best_den)
}
