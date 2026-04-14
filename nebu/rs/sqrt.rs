// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Square root and Legendre symbol over the Goldilocks field.
//!
//! Uses Tonelli-Shanks with z = 7 (quadratic non-residue).
//! p - 1 = 2^32 × s where s = ε = 2^32 - 1.

use crate::field::{Goldilocks, EPSILON, P};

const HALF_P: u64 = (P - 1) / 2; // 0x7FFFFFFF80000000

/// Legendre symbol: returns 0, 1, or p-1.
pub fn legendre(a: Goldilocks) -> Goldilocks {
    a.exp(HALF_P)
}

/// Compute the square root of `n` in the Goldilocks field.
///
/// Returns `Some(r)` where r <= (p-1)/2 (canonical sign),
/// or `None` if `n` is a quadratic non-residue.
pub fn sqrt(n: Goldilocks) -> Option<Goldilocks> {
    if n.is_zero() {
        return Some(Goldilocks::ZERO);
    }

    let leg = legendre(n);
    if leg == Goldilocks::NEG_ONE {
        return None;
    }

    let s: u64 = EPSILON; // odd part of p-1

    let mut big_m: u32 = 32;                         // two-adicity
    let mut c = Goldilocks::new(7).exp(s);            // 7^s, a 2^M-th root of unity
    let mut t = n.exp(s);                             // n^s
    let mut r = n.exp((s + 1) / 2);                   // n^((s+1)/2) = n^(2^31)

    loop {
        if t == Goldilocks::ONE {
            // Apply canonical sign convention: r <= (p-1)/2
            if r.as_u64() > HALF_P {
                r = -r;
            }
            return Some(r);
        }

        // Find least i > 0 such that t^(2^i) = 1
        let mut i: u32 = 1;
        let mut tmp = t.square();
        while tmp != Goldilocks::ONE {
            tmp = tmp.square();
            i += 1;
        }

        // b = c^(2^(M-i-1))
        let mut b = c;
        for _ in 0..(big_m - i - 1) {
            b = b.square();
        }

        big_m = i;
        c = b.square();
        t = t * c;
        r = r * b;
    }
}
