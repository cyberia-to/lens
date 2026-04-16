//! Tensor reduction for recursive polynomial opening.
//!
//! tensor_reduce halves a polynomial's evaluation table using a challenge:
//!   g'[i] = g[2i] + r · (g[2i+1] - g[2i])
//!
//! This is the multilinear extension evaluated at one variable set to r,
//! reducing ν variables to ν-1 variables.

use cyb_lens_core::Field;

/// Halve an evaluation table using challenge r.
///
/// Input: 2n evaluations, challenge r.
/// Output: n evaluations where g'[i] = g[2i] + r · (g[2i+1] - g[2i]).
pub fn tensor_reduce<F: Field>(evals: &[F], challenge: F) -> Vec<F> {
    let half = evals.len() / 2;
    let mut result = Vec::with_capacity(half);
    for i in 0..half {
        let even = evals[2 * i];
        let odd = evals[2 * i + 1];
        result.push(even + challenge * (odd - even));
    }
    result
}

/// Evaluate a small polynomial (≤ λ elements) at a composed point.
///
/// Used by the verifier to check the final polynomial in the opening proof.
/// The "point" here is the sequence of Fiat-Shamir challenges from the
/// tensor reduction rounds, composed in order.
pub fn evaluate_small<F: Field>(evals: &[F], challenges: &[F]) -> F {
    let mut current = evals.to_vec();
    for &c in challenges {
        current = tensor_reduce(&current, c);
    }
    assert_eq!(current.len(), 1, "challenges should reduce to single value");
    current[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use nebu::Goldilocks;

    #[test]
    fn tensor_reduce_halves() {
        let evals: Vec<Goldilocks> = (0..8).map(|i| Goldilocks::new(i + 1)).collect();
        let reduced = tensor_reduce(&evals, Goldilocks::ZERO);
        assert_eq!(reduced.len(), 4);
        // At r=0, g'[i] = g[2i] (the even elements)
        assert_eq!(reduced[0], Goldilocks::new(1));
        assert_eq!(reduced[1], Goldilocks::new(3));
        assert_eq!(reduced[2], Goldilocks::new(5));
        assert_eq!(reduced[3], Goldilocks::new(7));
    }

    #[test]
    fn tensor_reduce_at_one() {
        let evals: Vec<Goldilocks> = (0..8).map(|i| Goldilocks::new(i + 1)).collect();
        let reduced = tensor_reduce(&evals, Goldilocks::ONE);
        assert_eq!(reduced.len(), 4);
        // At r=1, g'[i] = g[2i+1] (the odd elements)
        assert_eq!(reduced[0], Goldilocks::new(2));
        assert_eq!(reduced[1], Goldilocks::new(4));
        assert_eq!(reduced[2], Goldilocks::new(6));
        assert_eq!(reduced[3], Goldilocks::new(8));
    }

    #[test]
    fn evaluate_small_single() {
        let evals = vec![Goldilocks::new(42)];
        let result = evaluate_small(&evals, &[]);
        assert_eq!(result, Goldilocks::new(42));
    }

    #[test]
    fn evaluate_small_matches_multilinear() {
        // f(x₁, x₂) with evals [1, 2, 3, 4]
        // f(0,0)=1, f(1,0)=2, f(0,1)=3, f(1,1)=4
        let evals = vec![
            Goldilocks::new(1),
            Goldilocks::new(2),
            Goldilocks::new(3),
            Goldilocks::new(4),
        ];

        // Evaluate at (r₁, r₂) = (0, 0) → should be 1
        let r = evaluate_small(&evals, &[Goldilocks::ZERO, Goldilocks::ZERO]);
        assert_eq!(r, Goldilocks::new(1));

        // Evaluate at (1, 1) → should be 4
        let r = evaluate_small(&evals, &[Goldilocks::ONE, Goldilocks::ONE]);
        assert_eq!(r, Goldilocks::new(4));
    }
}
