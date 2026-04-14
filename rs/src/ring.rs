// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! R_q element type: polynomials in F_p[x]/(x^n+1).

use nebu::Goldilocks;
use crate::ntt;

/// Maximum supported polynomial dimension.
pub const MAX_N: usize = 4096;

/// A polynomial in R_q = F_p[x]/(x^n+1).
///
/// Coefficients are stored in a fixed-size array of length MAX_N.
/// Only the first `n` coefficients are meaningful.
/// The `is_ntt` flag tracks whether the element is in NTT domain.
#[derive(Clone)]
pub struct RingElement {
    pub coeffs: [Goldilocks; MAX_N],
    pub n: usize,
    pub is_ntt: bool,
}

impl RingElement {
    /// Create the zero polynomial of dimension n.
    pub fn new(n: usize) -> Self {
        assert!(n.is_power_of_two() && n <= MAX_N, "n must be a power of 2 <= {}", MAX_N);
        RingElement {
            coeffs: [Goldilocks::ZERO; MAX_N],
            n,
            is_ntt: false,
        }
    }

    /// Create a ring element from a coefficient slice.
    /// The slice length must equal n.
    pub fn from_coeffs(coeffs: &[Goldilocks], n: usize) -> Self {
        assert!(n.is_power_of_two() && n <= MAX_N, "n must be a power of 2 <= {}", MAX_N);
        assert!(coeffs.len() == n, "coefficient slice length must equal n");
        let mut elem = RingElement::new(n);
        elem.coeffs[..n].copy_from_slice(coeffs);
        elem
    }

    /// Coefficient-wise addition.
    pub fn add(&self, other: &Self) -> Self {
        assert_eq!(self.n, other.n, "dimension mismatch");
        assert_eq!(self.is_ntt, other.is_ntt, "NTT domain mismatch");
        let mut result = self.clone();
        for i in 0..self.n {
            result.coeffs[i] = self.coeffs[i] + other.coeffs[i];
        }
        result
    }

    /// Coefficient-wise subtraction.
    pub fn sub(&self, other: &Self) -> Self {
        assert_eq!(self.n, other.n, "dimension mismatch");
        assert_eq!(self.is_ntt, other.is_ntt, "NTT domain mismatch");
        let mut result = self.clone();
        for i in 0..self.n {
            result.coeffs[i] = self.coeffs[i] - other.coeffs[i];
        }
        result
    }

    /// Coefficient-wise negation.
    pub fn neg(&self) -> Self {
        let mut result = self.clone();
        for i in 0..self.n {
            result.coeffs[i] = -self.coeffs[i];
        }
        result
    }

    /// Scalar multiplication: multiply every coefficient by s.
    pub fn scalar_mul(&self, s: Goldilocks) -> Self {
        let mut result = self.clone();
        for i in 0..self.n {
            result.coeffs[i] = self.coeffs[i] * s;
        }
        result
    }

    /// Polynomial multiplication in R_q via NTT.
    ///
    /// Both operands must be in coefficient form (is_ntt == false).
    /// Computes: to_ntt -> pointwise_mul -> from_ntt.
    pub fn mul(&self, other: &Self) -> Self {
        assert_eq!(self.n, other.n, "dimension mismatch");
        assert!(!self.is_ntt && !other.is_ntt, "operands must be in coefficient form");
        let mut a = self.clone();
        let mut b = other.clone();
        ntt::to_ntt(&mut a);
        ntt::to_ntt(&mut b);
        let mut c = a.pointwise_mul(&b);
        ntt::from_ntt(&mut c);
        c
    }

    /// Pointwise (coefficient-wise) multiplication.
    ///
    /// Both operands should be in NTT domain.
    pub fn pointwise_mul(&self, other: &Self) -> Self {
        assert_eq!(self.n, other.n, "dimension mismatch");
        assert!(self.is_ntt && other.is_ntt, "operands must be in NTT form");
        let mut result = Self::new(self.n);
        result.is_ntt = true;
        for i in 0..self.n {
            result.coeffs[i] = self.coeffs[i] * other.coeffs[i];
        }
        result
    }

    /// Galois automorphism: x -> x^(5^k mod 2n).
    ///
    /// Operates on coefficient form. The automorphism permutes and
    /// possibly negates coefficients according to the map x^i -> x^(i*t mod 2n)
    /// where t = 5^k mod 2n.
    pub fn automorphism(&self, k: usize) -> Self {
        assert!(!self.is_ntt, "automorphism requires coefficient form");
        let n = self.n;
        let two_n = 2 * n;

        // Compute t = 5^k mod 2n
        let mut t: usize = 1;
        for _ in 0..k {
            t = (t * 5) % two_n;
        }

        let mut result = Self::new(n);
        // For polynomial f(x) = sum a_i x^i, automorphism gives f(x^t) = sum a_i x^(i*t).
        // In R_q = F_p[x]/(x^n+1), x^n = -1, so x^j with j >= n means negate and reduce.
        for i in 0..n {
            let j = (i * t) % two_n;
            if j < n {
                result.coeffs[j] = result.coeffs[j] + self.coeffs[i];
            } else {
                // x^j = x^(j-n) * x^n = -x^(j-n)
                let idx = j - n;
                result.coeffs[idx] = result.coeffs[idx] - self.coeffs[i];
            }
        }
        result
    }

    /// Check if this polynomial is zero (all coefficients zero).
    pub fn is_zero(&self) -> bool {
        for i in 0..self.n {
            if !self.coeffs[i].is_zero() {
                return false;
            }
        }
        true
    }

    /// Ring equality: compare canonical forms of the first n coefficients.
    pub fn eq_ring(&self, other: &Self) -> bool {
        if self.n != other.n || self.is_ntt != other.is_ntt {
            return false;
        }
        for i in 0..self.n {
            if self.coeffs[i].as_u64() != other.coeffs[i].as_u64() {
                return false;
            }
        }
        true
    }

    /// Access a coefficient slice of the active region.
    pub fn active_coeffs(&self) -> &[Goldilocks] {
        &self.coeffs[..self.n]
    }

    /// Access a mutable coefficient slice of the active region.
    pub fn active_coeffs_mut(&mut self) -> &mut [Goldilocks] {
        &mut self.coeffs[..self.n]
    }
}

impl core::fmt::Debug for RingElement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "RingElement(n={}, ntt={}, [", self.n, self.is_ntt)?;
        let show = self.n.min(8);
        for i in 0..show {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", self.coeffs[i])?;
        }
        if self.n > show {
            write!(f, ", ...")?;
        }
        write!(f, "])")
    }
}
