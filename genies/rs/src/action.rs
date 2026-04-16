// ---
// tags: genies, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Class group action: [a] * E for ideal class [a] and supersingular curve E.
//!
//! The action iterates through the 74 small primes in q+1, applying ℓ-isogenies
//! for each prime factor according to the exponent vector.

use crate::curve::{MontCurve, MontPoint};
use crate::fq::{Fq, PRIME};
use crate::isogeny;

/// The 74 small odd primes dividing q+1.
pub const PRIMES: [u64; 74] = [
    3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193,
    197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293, 307,
    311, 313, 317, 331, 337, 347, 349, 353, 359, 367, 373, 587,
];

/// Number of primes.
pub const NUM_PRIMES: usize = 74;

/// Maximum exponent magnitude for CSIDH-512.
pub const MAX_EXPONENT: i8 = 5;

/// An ideal class element, represented as an exponent vector.
/// Each exponent is in [-MAX_EXPONENT, MAX_EXPONENT].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ideal {
    pub exponents: [i8; NUM_PRIMES],
}

impl Ideal {
    /// The identity ideal (all exponents zero).
    pub fn identity() -> Self {
        Ideal {
            exponents: [0; NUM_PRIMES],
        }
    }

    /// Construct from an exponent slice.
    pub fn from_exponents(e: &[i8]) -> Self {
        let mut exponents = [0i8; NUM_PRIMES];
        let len = if e.len() < NUM_PRIMES {
            e.len()
        } else {
            NUM_PRIMES
        };
        let mut i = 0;
        while i < len {
            exponents[i] = e[i];
            i += 1;
        }
        Ideal { exponents }
    }
}

/// Compute (q+1) / ell as an 8-limb big integer.
/// Since q+1 = 4 * l_1 * ... * l_74, dividing by any l_i gives an exact result.
fn cofactor(ell: u64) -> [u64; 8] {
    // q + 1
    let mut qp1 = [0u64; 8];
    let mut carry = 1u64;
    let mut i = 0;
    while i < 8 {
        let (s, c) = PRIME[i].overflowing_add(carry);
        qp1[i] = s;
        carry = c as u64;
        i += 1;
    }

    // Divide by ell (single-limb divisor, long division)
    let mut result = [0u64; 8];
    let mut rem = 0u128;
    i = 8;
    while i > 0 {
        i -= 1;
        rem = (rem << 64) | (qp1[i] as u128);
        result[i] = (rem / (ell as u128)) as u64;
        rem %= ell as u128;
    }
    result
}

/// Simple deterministic x-coordinate sampling for point finding.
/// Returns successive x = 0, 1, 2, 3, ... (skipping those that don't yield
/// points of the desired type).
struct XSampler {
    counter: u64,
}

impl XSampler {
    fn new() -> Self {
        XSampler { counter: 0 }
    }

    fn next(&mut self) -> Fq {
        let x = Fq::from_u64(self.counter);
        self.counter += 1;
        x
    }
}

/// Compute the class group action: [ideal] * curve.
///
/// For each prime ℓ_i with exponent e_i:
///   - If e_i > 0: find a point on E_A of order ℓ_i, apply e_i ℓ_i-isogenies
///   - If e_i < 0: find a point on the twist of E_A, apply |e_i| ℓ_i-isogenies
///     (twist point: x where x^3 + Ax^2 + x is a non-residue)
pub fn action(ideal: &Ideal, curve: &MontCurve) -> MontCurve {
    let mut current = *curve;

    let mut i = 0;
    while i < NUM_PRIMES {
        let mut e = ideal.exponents[i];
        let ell = PRIMES[i];

        while e != 0 {
            let want_on_curve = e > 0;
            let cof = cofactor(ell);

            // Find a kernel point of order ell
            let mut sampler = XSampler::new();
            loop {
                let x = sampler.next();
                let rhs = current.rhs(&x);

                // Check if rhs is a QR (point on curve) or NQR (point on twist)
                let leg = Fq::legendre(&rhs);
                if leg == 0 {
                    continue; // x = 0 might give rhs = 0, skip
                }

                let on_curve = leg == 1;
                if on_curve != want_on_curve {
                    continue;
                }

                // We have a point (x, y) on E_A (or its twist).
                // For the Montgomery ladder, we only need the x-coordinate.
                // Cofactor multiply: Q = [(q+1)/ell] * (x : 1)
                let p = MontPoint::from_x(x);
                let q_pt = p.ladder(&cof, &current.a);

                // Check Q != O
                if q_pt.is_inf() {
                    continue;
                }

                // Q has order ell (or a divisor of ell; since ell is prime,
                // order is exactly ell unless Q = O which we checked).

                // Apply the ell-isogeny
                current = isogeny::isogeny_codomain(&current, &q_pt, ell);

                // Decrement exponent magnitude
                if e > 0 {
                    e -= 1;
                } else {
                    e += 1;
                }
                break;
            }
        }
        i += 1;
    }

    current
}

/// Diffie-Hellman convenience: action(secret, peer_curve).
/// The commutativity of the class group gives:
///   dh(a, dh(b, E_0)) = dh(b, dh(a, E_0))
pub fn dh(secret: &Ideal, peer_curve: &MontCurve) -> MontCurve {
    action(secret, peer_curve)
}
