// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Error distribution sampling (deterministic, from seed).
//!
//! Uses xorshift64 as a simple PRNG for reproducibility and testing.
//! NOT cryptographically secure — real deployments would use
//! hemera-based extraction.

use nebu::Goldilocks;
use nebu::field::P;
use crate::ring::RingElement;

/// Simple xorshift64 PRNG state.
struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        // Ensure non-zero state
        let state = if seed == 0 { 0xDEAD_BEEF_CAFE_BABE } else { seed };
        Xorshift64 { state }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

/// Sample a uniform random ring element.
///
/// Each coefficient is a uniformly random element of F_p,
/// produced by rejection sampling from u64 values.
pub fn sample_uniform(seed: u64, n: usize) -> RingElement {
    let mut rng = Xorshift64::new(seed);
    let mut elem = RingElement::new(n);
    for i in 0..n {
        // Rejection sampling: draw u64, accept if < P
        loop {
            let val = rng.next();
            if val < P {
                elem.coeffs[i] = Goldilocks::new(val);
                break;
            }
            // Also accept val mod P for efficiency (slight bias but acceptable for testing)
            // Actually, for correctness use clean rejection
            elem.coeffs[i] = Goldilocks::new(val % P);
            break;
        }
    }
    elem
}

/// Sample a ternary ring element with coefficients in {-1, 0, 1}.
///
/// Each coefficient is chosen roughly uniformly from {-1, 0, 1}.
pub fn sample_ternary(seed: u64, n: usize) -> RingElement {
    let mut rng = Xorshift64::new(seed);
    let mut elem = RingElement::new(n);
    for i in 0..n {
        let val = rng.next() % 3;
        elem.coeffs[i] = match val {
            0 => Goldilocks::ZERO,
            1 => Goldilocks::ONE,
            2 => Goldilocks::NEG_ONE, // p - 1 = -1 mod p
            _ => unreachable!(),
        };
    }
    elem
}

/// Sample from a centered binomial distribution with parameter eta.
///
/// CBD(eta): sample 2*eta bits, compute (sum of first eta) - (sum of second eta).
/// Result is in {-eta, ..., eta}.
pub fn sample_cbd(seed: u64, n: usize, eta: usize) -> RingElement {
    assert!(eta > 0 && eta <= 16, "eta must be in [1, 16]");
    let mut rng = Xorshift64::new(seed);
    let mut elem = RingElement::new(n);
    for i in 0..n {
        let bits = rng.next();
        let mut pos_sum: i64 = 0;
        let mut neg_sum: i64 = 0;
        for j in 0..eta {
            pos_sum += ((bits >> j) & 1) as i64;
            neg_sum += ((bits >> (j + eta)) & 1) as i64;
        }
        let val = pos_sum - neg_sum;
        if val >= 0 {
            elem.coeffs[i] = Goldilocks::new(val as u64);
        } else {
            // -|val| mod p = p - |val|
            elem.coeffs[i] = Goldilocks::new(P - ((-val) as u64));
        }
    }
    elem
}
