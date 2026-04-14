// ---
// tags: genies, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Montgomery curve operations over F_q.
//!
//! All curves have the form E_A: y^2 = x^3 + Ax^2 + x over F_q.
//! Points are represented in projective XZ coordinates for efficiency.

use crate::fq::Fq;

/// A Montgomery curve E_A: y^2 = x^3 + Ax^2 + x.
/// Determined by the single coefficient A in F_q (B = 1 assumed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MontCurve {
    pub a: Fq,
}

/// A point on a Montgomery curve in projective XZ coordinates.
/// Represents the affine x-coordinate x = X/Z.
/// The point at infinity is (0 : 0) by convention (Z = 0).
#[derive(Clone, Copy, Debug)]
pub struct MontPoint {
    pub x: Fq,
    pub z: Fq,
}

impl MontCurve {
    /// The starting curve E_0: y^2 = x^3 + x (A = 0).
    pub fn e0() -> MontCurve {
        MontCurve { a: Fq::ZERO }
    }

    /// Construct a curve from its A coefficient.
    pub fn from_a(a: Fq) -> MontCurve {
        MontCurve { a }
    }

    /// Evaluate the right-hand side of the curve equation: x^3 + A*x^2 + x.
    /// This is used to test if an x-coordinate yields a point on the curve.
    pub fn rhs(&self, x: &Fq) -> Fq {
        // x^3 + A*x^2 + x = x * (x^2 + A*x + 1)
        let x2 = Fq::square(x);
        let ax = Fq::mul(&self.a, x);
        let inner = Fq::add(&Fq::add(&x2, &ax), &Fq::ONE);
        Fq::mul(x, &inner)
    }

    /// Check if x gives a point on this curve (i.e., rhs is a QR).
    pub fn is_on_curve(&self, x: &Fq) -> bool {
        Fq::legendre(&self.rhs(x)) == 1
    }

    /// j-invariant: j = 256 * (A^2 - 3)^3 / (A^2 - 4).
    pub fn j_invariant(&self) -> Fq {
        let a2 = Fq::square(&self.a);
        let three = Fq::from_u64(3);
        let four = Fq::from_u64(4);
        let num_base = Fq::sub(&a2, &three); // A^2 - 3
        let num_sq = Fq::square(&num_base);
        let num_cu = Fq::mul(&num_sq, &num_base); // (A^2 - 3)^3
        let c256 = Fq::from_u64(256);
        let num = Fq::mul(&c256, &num_cu); // 256 * (A^2 - 3)^3
        let den = Fq::sub(&a2, &four); // A^2 - 4
        let den_inv = Fq::inv(&den);
        Fq::mul(&num, &den_inv)
    }
}

impl MontPoint {
    /// The point at infinity (identity element).
    pub fn inf() -> MontPoint {
        MontPoint {
            x: Fq::ONE,
            z: Fq::ZERO,
        }
    }

    /// Construct from an affine x-coordinate (Z = 1).
    pub fn from_x(x: Fq) -> MontPoint {
        MontPoint { x, z: Fq::ONE }
    }

    /// Check if this is the point at infinity (Z = 0).
    pub fn is_inf(&self) -> bool {
        self.z.is_zero()
    }

    /// Normalize to affine: return X/Z, or None if point at infinity.
    pub fn to_affine(&self) -> Option<Fq> {
        if self.z.is_zero() {
            None
        } else {
            Some(Fq::mul(&self.x, &Fq::inv(&self.z)))
        }
    }

    /// X-only point doubling on Montgomery curve E_A.
    ///
    /// Uses the formula from the spec:
    ///   S = (X + Z)^2
    ///   D = (X - Z)^2
    ///   X_{2P} = S * D
    ///   Z_{2P} = (S - D) * (S + ((A+2)/4) * (S - D))
    pub fn xdbl(&self, a24: &Fq) -> MontPoint {
        let s = Fq::square(&Fq::add(&self.x, &self.z)); // (X + Z)^2
        let d = Fq::square(&Fq::sub(&self.x, &self.z)); // (X - Z)^2
        let diff = Fq::sub(&s, &d); // S - D = 4*X*Z
        let new_x = Fq::mul(&s, &d); // S * D
        // (A+2)/4 * (S - D) + D  (note: a24 = (A+2)/4)
        let tmp = Fq::add(&Fq::mul(a24, &diff), &d);
        let new_z = Fq::mul(&diff, &tmp);
        MontPoint { x: new_x, z: new_z }
    }

    /// Differential addition: given P, Q, and P-Q, compute P+Q.
    ///
    /// Formula:
    ///   U = (X_P - Z_P)(X_Q + Z_Q)
    ///   V = (X_P + Z_P)(X_Q - Z_Q)
    ///   X_{P+Q} = Z_{P-Q} * (U + V)^2
    ///   Z_{P+Q} = X_{P-Q} * (U - V)^2
    pub fn xadd(p: &MontPoint, q: &MontPoint, pmq: &MontPoint) -> MontPoint {
        let u = Fq::mul(
            &Fq::sub(&p.x, &p.z),
            &Fq::add(&q.x, &q.z),
        );
        let v = Fq::mul(
            &Fq::add(&p.x, &p.z),
            &Fq::sub(&q.x, &q.z),
        );
        let sum = Fq::add(&u, &v);
        let dif = Fq::sub(&u, &v);
        let new_x = Fq::mul(&pmq.z, &Fq::square(&sum));
        let new_z = Fq::mul(&pmq.x, &Fq::square(&dif));
        MontPoint { x: new_x, z: new_z }
    }

    /// Montgomery ladder: compute [k]P on curve with coefficient A.
    /// k is given as an arbitrary-length big-endian byte slice.
    pub fn ladder(&self, k: &[u64], a: &Fq) -> MontPoint {
        // Precompute (A+2)/4
        let two = Fq::from_u64(2);
        let four = Fq::from_u64(4);
        let a24 = Fq::mul(&Fq::add(a, &two), &Fq::inv(&four));

        Self::ladder_a24(self, k, &a24)
    }

    /// Montgomery ladder with precomputed a24 = (A+2)/4.
    /// k is a scalar given as little-endian u64 limbs.
    pub fn ladder_a24(&self, k: &[u64], a24: &Fq) -> MontPoint {
        let mut r0 = MontPoint::inf();
        let mut r1 = *self;

        // Find the highest set bit
        let mut started = false;
        let mut i = k.len();
        while i > 0 {
            i -= 1;
            let mut bit = 63u32;
            loop {
                if started || (k[i] >> bit) & 1 == 1 {
                    started = true;
                    let b = (k[i] >> bit) & 1;
                    if b == 1 {
                        r0 = MontPoint::xadd(&r0, &r1, self);
                        r1 = r1.xdbl(a24);
                    } else {
                        r1 = MontPoint::xadd(&r0, &r1, self);
                        r0 = r0.xdbl(a24);
                    }
                }
                if bit == 0 {
                    break;
                }
                bit -= 1;
            }
        }
        r0
    }

    /// Scalar multiplication by a single u64 value.
    pub fn scalar_mul(&self, k: u64, a: &Fq) -> MontPoint {
        self.ladder(&[k], a)
    }
}
