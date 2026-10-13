//! Proven soundness terms for Reed–Solomon proximity, as code.
//!
//! All errors are probabilities over a challenge drawn uniformly from
//! `K = Fp3` (`|K| = p³`). Logarithms are base 2. Every bound here is a
//! published theorem; nothing conjectured is used.
//!
//! Notation: a code `RS[n, k]` of rate `ρ = k/n = 2^-r` (`r = log_inv_rate`).
//!
//! Regimes:
//! - Unique decoding: `δ < (1 − ρ)/2`, list size 1.
//! - Johnson (integer `m ≥ 3`): `δ = 1 − (1 + 1/(2m))·√ρ`. List size
//!   `≤ 1/(2η√ρ) = m/ρ` with `η = √ρ/(2m)` (Johnson bound, WHIR Theorem 4.3).

/// `log2 |Fp3| = 3·log2(p)`, `p = 2^64 − 2^32 + 1`.
pub fn ext_field_bits() -> f64 {
    3.0 * (nebu::field::P as f64).log2()
}

/// `log2(2^a + 2^b)`.
pub fn log2_add(a: f64, b: f64) -> f64 {
    if a == f64::NEG_INFINITY {
        return b;
    }
    if b == f64::NEG_INFINITY {
        return a;
    }
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    hi + (1.0 + (lo - hi).exp2()).log2()
}

/// Decoding regime of a WHIR code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Regime {
    /// `δ < (1 − ρ)/2`: unique decoding (WHIR Corollary 4.11).
    Unique,
    /// Up to the Johnson bound with the given integer trade-off `m ≥ 3`
    /// (BCGM, eprint 2025/2051, Definition 9.1 / Lemma 9.3).
    Johnson(u32),
}

/// Proximity data for one code.
#[derive(Clone, Copy, Debug)]
pub struct Proximity {
    /// `log2(1 − δ)`: per-query acceptance of a δ-far word.
    pub log_one_minus_delta: f64,
    /// `log2` of the list-size bound at δ.
    pub log_list: f64,
}

/// Proximity parameters for a code of rate `2^-r` and length `2^log_n`.
pub fn proximity(regime: Regime, r: u32, log_n: u32) -> Proximity {
    let rho = (-(r as f64)).exp2();
    match regime {
        Regime::Unique => {
            // δ = (1 − ρ)/2 − 1/n, strictly inside the unique radius.
            let one_minus = (1.0 + rho) / 2.0 + (-(log_n as f64)).exp2();
            Proximity {
                log_one_minus_delta: one_minus.log2(),
                log_list: 0.0,
            }
        }
        Regime::Johnson(m) => {
            assert!(m >= 3, "Johnson trade-off m must be ≥ 3");
            let m = m as f64;
            Proximity {
                log_one_minus_delta: -(r as f64) / 2.0 + (1.0 + 1.0 / (2.0 * m)).log2(),
                log_list: m.log2() + r as f64,
            }
        }
    }
}

/// `log2 ε_mca` for the affine line generator `(1, α)` on `RS[n, k]` over
/// `K`, at the regime's δ.
///
/// - Unique: `(ℓ−1)·k/(ρ|K|)` with `ℓ = 2`, i.e. `n/|K|`
///   (WHIR Theorem 4.8 first case with Corollary 4.11).
/// - Johnson: `(m+½)^7/(3ρ^{3/2}) · d · n² / |K|` with generator degree
///   `d = 1` (BCGM eprint 2025/2051, Definition 9.1 and Lemma 9.3; the
///   affine-line case is Haböck eprint 2025/2110, Theorem 2).
pub fn mca_log2(regime: Regime, r: u32, log_n: u32) -> f64 {
    let k_bits = ext_field_bits();
    match regime {
        Regime::Unique => log_n as f64 - k_bits,
        Regime::Johnson(m) => {
            let m = m as f64;
            7.0 * (m + 0.5).log2() - 3f64.log2() + 1.5 * r as f64 + 2.0 * log_n as f64 - k_bits
        }
    }
}

/// `−log2` of an error given as a `log2`, plus grinding bits.
pub fn bits(log_err: f64, pow: u32) -> f64 {
    -log_err + pow as f64
}
