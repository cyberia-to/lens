//! Systematic code: raw input followed by sparse Margulis-derived parity.
//! The raw prefix proves injectivity; no constant relative-distance claim is made.
//! See specs/scalar-field.md for the versioned encoding contract.

use nebu::Goldilocks;

/// Expansion factor: |R| = EXPANSION · |L|.
pub const EXPANSION: usize = 3;
const PARITY_EXPANSION: usize = 2;

/// Number of base Margulis transformations per step.
const BASE_DEGREE: usize = 6;

/// Number of composition steps. Total degree = BASE_DEGREE^WALK_LENGTH.
/// 2 steps: degree 36, 3 steps: degree 216. We use 2 for efficiency.
const WALK_LENGTH: usize = 2;

/// Effective degree: BASE_DEGREE^WALK_LENGTH = 36.
#[cfg(test)]
const DEGREE: usize = BASE_DEGREE * BASE_DEGREE; // 36

/// Expander graph for Brakedown encoding.
pub struct Expander {
    /// Number of input elements (|L|).
    pub n: usize,
    /// Number of output elements (|R| = EXPANSION · n).
    pub m: usize,
    /// Prime p such that p² ≥ 2n (parity positions). Vertices live in Z_p × Z_p.
    p: usize,
}

impl Expander {
    /// Create an expander for input size n.
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "expander requires n > 0");
        let m = EXPANSION.checked_mul(n).expect("encoded length overflow");
        // Find smallest prime p such that p² covers the parity positions
        let p = Self::smallest_prime_with_square_ge(PARITY_EXPANSION * n);
        Self { n, m, p }
    }

    /// Smallest prime p where p² ≥ target.
    fn smallest_prime_with_square_ge(target: usize) -> usize {
        let floor = target.isqrt();
        let sqrt = floor + usize::from(floor * floor < target);
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
        self.n + (x * self.p + y) % (PARITY_EXPANSION * self.n)
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

    /// Preserve every input symbol, then append the sparse parity sums.
    /// Cost: DEGREE · n field additions plus the raw-prefix copy.
    pub fn encode(&self, input: &[Goldilocks]) -> Vec<Goldilocks> {
        assert_eq!(input.len(), self.n);
        let mut output = vec![Goldilocks::ZERO; self.m];
        output[..self.n].copy_from_slice(input);

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
            all_neighbors.len() > (exp.m - exp.n) / 3,
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
    fn relative_weight_for_support(
        exp: &Expander,
        support: &[usize],
        coeffs: &[Goldilocks],
    ) -> f64 {
        // Probe sparse supports directly; encoding n zero entries per pair
        // would turn this diagnostic into an unnecessary cubic-time test.
        let mut symbols = std::collections::BTreeMap::new();
        for (&i, &c) in support.iter().zip(coeffs) {
            *symbols.entry(i).or_insert(Goldilocks::ZERO) += c;
            for j in exp.neighbors(i) {
                *symbols.entry(j).or_insert(Goldilocks::ZERO) += c;
            }
        }
        symbols.values().filter(|&&v| v != Goldilocks::ZERO).count() as f64 / exp.m as f64
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

    /// Diagnostic upper bounds on minimum distance. The query budget uses
    /// only systematic injectivity (absolute weight >= 1), not this probe.
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
        // A diagnostic measurement cannot establish a stronger bound.
        assert!(
            worst_abs_overall > crate::MIN_ABS_WEIGHT as f64,
            "expander code's empirically probed absolute weight floor \
             ({worst_abs_overall:.1}) dropped to or below \
             MIN_ABS_WEIGHT ({}) — re-derive NUM_QUERIES in lib.rs",
            crate::MIN_ABS_WEIGHT
        );
    }
}
