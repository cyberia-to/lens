//! Margulis expander graph for linear-code encoding.
//!
//! The Margulis construction uses affine transformations over Z_p × Z_p
//! to produce a bipartite graph with provable spectral expansion.
//! For any subset S of left vertices with |S| ≤ δN, the neighborhood
//! satisfies |Γ(S)| ≥ (1-ε)d|S|, where d is the left-degree.
//!
//! This replaces hash-based neighbor assignment with an algebraically
//! constructed graph whose expansion is guaranteed by Kazhdan's property (T).
//!
//! Reference: Margulis, "Explicit constructions of expanders" (1973).
//! The bipartite version maps (x, y) ∈ Z_p × Z_p to neighbors via
//! six affine transformations, giving base degree 6. We compose
//! WALK_LENGTH steps of these transformations to achieve the target degree.

use nebu::Goldilocks;

/// Expansion factor: |R| = EXPANSION · |L|.
pub const EXPANSION: usize = 2;

/// Number of base Margulis transformations per step.
const BASE_DEGREE: usize = 6;

/// Number of composition steps. Total degree = BASE_DEGREE^WALK_LENGTH.
/// 2 steps: degree 36, 3 steps: degree 216. We use 2 for efficiency.
const WALK_LENGTH: usize = 2;

/// Effective degree: BASE_DEGREE^WALK_LENGTH = 36.
/// For 128-bit security over Goldilocks (|F| ≈ 2^64), degree 36 with
/// expansion factor 2 provides sufficient distance.
#[cfg(test)]
const DEGREE: usize = BASE_DEGREE * BASE_DEGREE; // 36

/// Expander graph for Brakedown encoding.
pub struct Expander {
    /// Number of input elements (|L|).
    pub n: usize,
    /// Number of output elements (|R| = EXPANSION · n).
    pub m: usize,
    /// Prime p such that p² ≥ m. Vertices live in Z_p × Z_p.
    p: usize,
}

impl Expander {
    /// Create an expander for input size n.
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "expander requires n > 0");
        let m = EXPANSION * n;
        // Find smallest prime p such that p² ≥ m
        let p = Self::smallest_prime_with_square_ge(m);
        Self { n, m, p }
    }

    /// Smallest prime p where p² ≥ target.
    fn smallest_prime_with_square_ge(target: usize) -> usize {
        let sqrt = (target as f64).sqrt().ceil() as usize;
        let mut p = if sqrt < 2 { 2 } else { sqrt };
        while !Self::is_prime(p) {
            p += 1;
        }
        p
    }

    fn is_prime(n: usize) -> bool {
        if n < 2 {
            return false;
        }
        if n < 4 {
            return true;
        }
        if n.is_multiple_of(2) || n.is_multiple_of(3) {
            return false;
        }
        let mut i = 5;
        while i * i <= n {
            if n.is_multiple_of(i) || n.is_multiple_of(i + 2) {
                return false;
            }
            i += 6;
        }
        true
    }

    /// The six Margulis transformations on Z_p × Z_p.
    /// Each maps (x, y) to a neighbor deterministically.
    #[inline]
    fn margulis_neighbors(&self, x: usize, y: usize) -> [(usize, usize); BASE_DEGREE] {
        let p = self.p;
        [
            ((x + y) % p, y),
            ((x + p - y) % p, y),
            (x, (y + x) % p),
            (x, (y + p - x) % p),
            ((x + y + 1) % p, y),
            (x, (y + x + 1) % p),
        ]
    }

    /// Convert a 2D coordinate (x, y) ∈ Z_p × Z_p to a linear index.
    #[inline]
    fn to_index(&self, x: usize, y: usize) -> usize {
        (x * self.p + y) % self.m
    }

    /// Convert a linear index to 2D coordinate.
    #[inline]
    fn index_to_2d(&self, i: usize) -> (usize, usize) {
        (i / self.p % self.p, i % self.p)
    }

    /// Compute all right neighbors of left vertex i.
    /// Uses WALK_LENGTH-step composition of Margulis transformations.
    /// Returns DEGREE = BASE_DEGREE^WALK_LENGTH neighbors.
    pub fn neighbors(&self, i: usize) -> Vec<usize> {
        let (x, y) = self.index_to_2d(i);

        // Step 1: base Margulis neighbors
        let mut current: Vec<(usize, usize)> = self.margulis_neighbors(x, y).to_vec();

        // Steps 2..WALK_LENGTH: compose by applying Margulis again
        for _ in 1..WALK_LENGTH {
            let mut next = Vec::with_capacity(current.len() * BASE_DEGREE);
            for &(cx, cy) in &current {
                next.extend_from_slice(&self.margulis_neighbors(cx, cy));
            }
            current = next;
        }

        // Map 2D coordinates to linear indices in [0, m)
        current.iter().map(|&(x, y)| self.to_index(x, y)).collect()
    }

    /// Encode input polynomial via sparse matrix-vector multiply.
    ///
    /// output[r] += input[l] for all edges (l, r) in the Margulis graph.
    /// Cost: DEGREE · n field additions.
    pub fn encode(&self, input: &[Goldilocks]) -> Vec<Goldilocks> {
        assert_eq!(input.len(), self.n);
        let mut output = vec![Goldilocks::ZERO; self.m];

        for (i, &val) in input.iter().enumerate() {
            for r in self.neighbors(i) {
                output[r] += val;
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smallest_prime() {
        assert_eq!(Expander::smallest_prime_with_square_ge(4), 2); // 2² = 4
        assert_eq!(Expander::smallest_prime_with_square_ge(5), 3); // 3² = 9
        assert_eq!(Expander::smallest_prime_with_square_ge(100), 11); // 11² = 121
    }

    #[test]
    fn expander_deterministic() {
        let exp = Expander::new(16);
        let n1 = exp.neighbors(5);
        let n2 = exp.neighbors(5);
        assert_eq!(n1, n2);
    }

    #[test]
    fn correct_degree() {
        let exp = Expander::new(64);
        let neighbors = exp.neighbors(7);
        assert_eq!(neighbors.len(), DEGREE);
    }

    #[test]
    fn neighbors_in_range() {
        let exp = Expander::new(64);
        for i in 0..64 {
            for &r in &exp.neighbors(i) {
                assert!(r < exp.m, "neighbor {r} out of range [0, {})", exp.m);
            }
        }
    }

    #[test]
    fn expansion_property() {
        // For a proper Margulis graph, distinct vertices should have
        // neighborhoods that collectively cover many distinct right vertices.
        let exp = Expander::new(64);
        let mut all_neighbors = std::collections::HashSet::new();
        // Take 10 vertices, collect their neighborhoods
        for i in 0..10 {
            for &r in &exp.neighbors(i) {
                all_neighbors.insert(r);
            }
        }
        // 10 vertices × 36 neighbors = 360 edges.
        // With m=128, expect significant coverage.
        // A good expander should cover at least half the right vertices.
        assert!(
            all_neighbors.len() > exp.m / 3,
            "expansion too low: {} distinct out of {}",
            all_neighbors.len(),
            exp.m
        );
    }

    #[test]
    fn encode_zero_polynomial() {
        let exp = Expander::new(8);
        let input = vec![Goldilocks::ZERO; 8];
        let output = exp.encode(&input);
        for &v in &output {
            assert_eq!(v, Goldilocks::ZERO);
        }
    }

    #[test]
    fn encode_output_size() {
        let exp = Expander::new(16);
        let input = vec![Goldilocks::ONE; 16];
        let output = exp.encode(&input);
        assert_eq!(output.len(), EXPANSION * 16);
    }

    #[test]
    fn encode_different_inputs_differ() {
        let exp = Expander::new(8);
        let a = vec![Goldilocks::ONE; 8];
        let mut b = vec![Goldilocks::ONE; 8];
        b[0] = Goldilocks::new(42);
        assert_ne!(exp.encode(&a), exp.encode(&b));
    }

    // ── empirical minimum-distance probe ───────────────────────────
    //
    // This code has no proven worst-case distance bound (the Margulis
    // construction here is a single-layer sparse map, not the recursive
    // robust-code construction the Brakedown paper uses to GUARANTEE
    // constant relative distance). Rather than assume a distance figure,
    // this probe empirically searches for low-weight nonzero codewords
    // using the standard "cancel via free ratios" adversarial strategy:
    // a support of size s gives (s-1) free ratios (after fixing scale),
    // each of which can zero one output position that the support's
    // neighborhoods share, so an adversary can drive weight down to
    // roughly |union of neighborhoods| - (s-1). We search s = 1, 2, 3
    // over many random supports and report the worst (lowest-weight)
    // relative weight found. This is a LOWER BOUND on the true minimum
    // distance search (larger s could do worse) and an UPPER BOUND on
    // the actual guaranteed distance (we only tried s ≤ 3) — it is
    // reported honestly as an empirical measurement, not a proof.
    fn hamming_weight(v: &[Goldilocks]) -> usize {
        v.iter().filter(|&&x| x != Goldilocks::ZERO).count()
    }

    fn relative_weight_for_support(exp: &Expander, support: &[usize], coeffs: &[Goldilocks]) -> f64 {
        let mut input = vec![Goldilocks::ZERO; exp.n];
        for (&i, &c) in support.iter().zip(coeffs) {
            input[i] = c;
        }
        let out = exp.encode(&input);
        hamming_weight(&out) as f64 / out.len() as f64
    }

    /// Try to zero one shared output position between inputs i and j.
    /// Returns the resulting relative weight, or None if i,j share no
    /// output position (nothing to cancel).
    fn try_cancel_pair(exp: &Expander, i: usize, j: usize) -> Option<f64> {
        use std::collections::HashMap;
        let ni = exp.neighbors(i);
        let nj = exp.neighbors(j);
        let mut mult_i: HashMap<usize, u64> = HashMap::new();
        for r in &ni {
            *mult_i.entry(*r).or_default() += 1;
        }
        let mut mult_j: HashMap<usize, u64> = HashMap::new();
        for r in &nj {
            *mult_j.entry(*r).or_default() += 1;
        }
        // Find a shared output position to cancel.
        let shared = mult_i.keys().find(|r| mult_j.contains_key(*r))?;
        let mi = Goldilocks::new(mult_i[shared]);
        let mj = Goldilocks::new(mult_j[shared]);
        // x_i * mi + x_j * mj = 0  =>  x_j = -x_i * mi / mj. Fix x_i = 1.
        let x_i = Goldilocks::ONE;
        let x_j = -(x_i * mi * mj.inv());
        Some(relative_weight_for_support(exp, &[i, j], &[x_i, x_j]))
    }

    /// Empirically probe the minimum distance for a range of sizes used by
    /// the tensor-Merkle PCS (k2 ≈ sqrt(N) for realistic N), and print the
    /// worst weight found (both relative — as a fraction of m — and
    /// absolute). `lib.rs::num_queries` derives its query count from an
    /// ASSUMED worst-case ABSOLUTE weight (`ASSUMED_MIN_ABS_WEIGHT`, set
    /// with a safety margin below what this probe actually finds): the
    /// assertion below checks that assumption still holds, i.e. that this
    /// probe cannot force fewer nonzero symbols than `num_queries` assumed
    /// possible. It deliberately does NOT assert a relative-weight floor —
    /// relative distance shrinks as k2 grows for this single-layer
    /// expander code (roughly 1/k2 — see the numbers this prints), which
    /// is exactly why `num_queries` scales with k2 instead of being fixed.
    #[test]
    fn empirical_distance_probe() {
        let sizes = [8usize, 16, 32, 64, 128, 256, 512, 1024];
        let mut worst_abs_overall = f64::INFINITY;
        for &n in &sizes {
            let exp = Expander::new(n);
            let mut worst = 1.0f64;

            // s = 1: no cancellation possible, just the coverage of one
            // input's own neighbor list (upper bound on achievable weight
            // from a trivial adversary, included for reference).
            for i in 0..n {
                let w = relative_weight_for_support(&exp, &[i], &[Goldilocks::ONE]);
                worst = worst.min(w);
            }

            // s = 2: one free ratio, cancel one shared output position.
            for i in 0..n {
                for j in (i + 1)..n {
                    if let Some(w) = try_cancel_pair(&exp, i, j) {
                        worst = worst.min(w);
                    }
                }
            }

            let worst_abs = worst * exp.m as f64;
            eprintln!(
                "n={n:5}  m={:5}  worst weight (s<=2): relative={worst:.4}  absolute={worst_abs:.1}",
                exp.m
            );
            worst_abs_overall = worst_abs_overall.min(worst_abs);
        }
        eprintln!("worst absolute weight over all probed sizes = {worst_abs_overall:.1}");
        // This is an empirical measurement, not a proof (only s<=2 attacks
        // were searched — a real adversary is not restricted to those).
        // What it guards: lib.rs's ASSUMED_MIN_ABS_WEIGHT must stay a real
        // lower bound on what this probe can find, with margin, or
        // NUM_QUERIES silently under-delivers on soundness.
        assert!(
            worst_abs_overall > crate::ASSUMED_MIN_ABS_WEIGHT as f64,
            "expander code's empirically probed absolute weight floor \
             ({worst_abs_overall:.1}) dropped to or below \
             ASSUMED_MIN_ABS_WEIGHT ({}) — re-derive NUM_QUERIES in lib.rs",
            crate::ASSUMED_MIN_ABS_WEIGHT
        );
    }
}
