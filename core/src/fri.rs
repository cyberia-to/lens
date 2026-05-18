// ---
// tags: lens, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! FRI folding primitive — core step of Fast Reed-Solomon IOP.
//!
//! [`fri_fold`] reduces a power-of-two evaluation table to a single field
//! element by repeatedly applying the linear interpolation:
//!
//! ```text
//! result = left + r * (right − left)
//! ```
//!
//! across every adjacent pair, halving the table each round until one value
//! remains. This is the folding operation used at each round of a FRI proof.
//! It belongs in lens-core as a foundational primitive; future FRI-based
//! lenses (e.g. a FRI-based successor to Brakedown) will call it directly.

use nebu::Goldilocks;

/// Fold a power-of-two evaluation table to a single Goldilocks element.
///
/// # Algorithm
///
/// Each round applies the linear interpolation to every adjacent pair:
/// `out[i] = evals[2i] + r * (evals[2i+1] - evals[2i])`
///
/// The fold repeats until one element remains. The computation is in-place:
/// results overwrite the left half of the working slice each round, so the
/// only allocation is the caller's original slice.
///
/// # Panics
///
/// Panics if `evals` is empty or its length is not a power of two.
///
/// # Examples
///
/// ```
/// use nebu::Goldilocks;
/// use cyb_lens_core::fri::fri_fold;
///
/// let a = Goldilocks::new(3);
/// let b = Goldilocks::new(7);
/// let r = Goldilocks::new(2);
/// // a + r*(b-a) = 3 + 2*(7-3) = 3 + 8 = 11
/// let evals = vec![a, b];
/// assert_eq!(fri_fold(&evals, r), Goldilocks::new(11));
/// ```
pub fn fri_fold(evals: &[Goldilocks], r: Goldilocks) -> Goldilocks {
    let n = evals.len();
    assert!(n >= 1 && n.is_power_of_two(), "fri_fold: evals length must be a non-empty power of two");

    if n == 1 {
        return evals[0];
    }

    // Copy into a working buffer so we can fold in-place without mutating caller's slice.
    // The buffer shrinks conceptually each round; we only use the first `width` elements.
    let mut buf: Vec<Goldilocks> = evals.to_vec();
    let mut width = n;

    while width > 1 {
        let half = width / 2;
        for i in 0..half {
            let left  = buf[2 * i];
            let right = buf[2 * i + 1];
            buf[i] = left + r * (right - left);
        }
        width = half;
    }

    buf[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(v: u64) -> Goldilocks { Goldilocks::new(v) }

    /// Goldilocks prime p = 2^64 - 2^32 + 1.
    const P: u64 = 0xFFFF_FFFF_0000_0001;

    #[test]
    fn single_element_returns_itself() {
        assert_eq!(fri_fold(&[g(7)], g(3)), g(7));
        assert_eq!(fri_fold(&[g(0)], g(0)), g(0));
    }

    #[test]
    fn two_elements_r_zero_returns_left() {
        // r=0: left + 0*(right-left) = left
        assert_eq!(fri_fold(&[g(3), g(7)], g(0)), g(3));
    }

    #[test]
    fn two_elements_r_one_returns_right() {
        // r=1: left + 1*(right-left) = right
        assert_eq!(fri_fold(&[g(3), g(7)], g(1)), g(7));
    }

    #[test]
    fn two_elements_linear_interpolation() {
        // a=3, b=7, r=2: 3 + 2*(7-3) = 3 + 8 = 11
        assert_eq!(fri_fold(&[g(3), g(7)], g(2)), g(11));
    }

    #[test]
    fn four_elements_two_levels_at_r_half() {
        // evals=[0, 4, 8, 12], r=1/2 in Goldilocks
        // level-1 pairs: (0,4) and (8,12)
        //   0 + half*(4-0) = 2
        //   8 + half*(12-8) = 10
        // level-2: 2 + half*(10-2) = 2 + 4 = 6
        let half = Goldilocks::new((P + 1) / 2);
        assert_eq!(fri_fold(&[g(0), g(4), g(8), g(12)], half), g(6));
    }

    #[test]
    fn four_elements_r_zero_returns_first() {
        // r=0 collapses every pair to its left element, so result = evals[0]
        assert_eq!(fri_fold(&[g(5), g(9), g(13), g(17)], g(0)), g(5));
    }

    #[test]
    fn four_elements_r_one_returns_last() {
        // r=1 collapses every pair to its right element
        // level-1: [9, 17]
        // level-2: 17
        assert_eq!(fri_fold(&[g(5), g(9), g(13), g(17)], g(1)), g(17));
    }

    #[test]
    fn caller_slice_unchanged() {
        // fri_fold must not mutate the caller's buffer
        let evals = vec![g(1), g(2), g(3), g(4)];
        let snapshot = evals.clone();
        let _ = fri_fold(&evals, g(5));
        assert_eq!(evals, snapshot);
    }

    #[test]
    #[should_panic(expected = "fri_fold: evals length must be a non-empty power of two")]
    fn empty_panics() {
        fri_fold(&[], g(1));
    }

    #[test]
    #[should_panic(expected = "fri_fold: evals length must be a non-empty power of two")]
    fn non_power_of_two_panics() {
        fri_fold(&[g(1), g(2), g(3)], g(1));
    }
}
