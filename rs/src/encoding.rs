// ---
// tags: genies, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Serialization for genies types.
//!
//! - Fq: 64 bytes, little-endian limbs, each limb in LE byte order.
//! - MontCurve: 64 bytes (the A coefficient as an Fq element).
//! - Ideal: 74 bytes, each exponent shifted by MAX_EXPONENT to make unsigned.

use crate::fq::{Fq, PRIME};
use crate::curve::MontCurve;
use crate::action::{Ideal, NUM_PRIMES, MAX_EXPONENT};

/// Encode an Fq element as 64 bytes (little-endian).
pub fn encode_fq(a: &Fq) -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut i = 0;
    while i < 8 {
        let bytes = a.limbs[i].to_le_bytes();
        let mut j = 0;
        while j < 8 {
            out[i * 8 + j] = bytes[j];
            j += 1;
        }
        i += 1;
    }
    out
}

/// Decode an Fq element from 64 bytes (little-endian).
/// Returns None if the value is >= q (non-canonical).
pub fn decode_fq(bytes: &[u8; 64]) -> Option<Fq> {
    let mut limbs = [0u64; 8];
    let mut i = 0;
    while i < 8 {
        let mut buf = [0u8; 8];
        let mut j = 0;
        while j < 8 {
            buf[j] = bytes[i * 8 + j];
            j += 1;
        }
        limbs[i] = u64::from_le_bytes(buf);
        i += 1;
    }

    // Check that value < q
    // Compare from most significant limb down
    i = 8;
    while i > 0 {
        i -= 1;
        if limbs[i] > PRIME[i] {
            return None;
        }
        if limbs[i] < PRIME[i] {
            return Some(Fq::from_limbs(limbs));
        }
    }
    // All limbs equal means value == q, which is not canonical
    None
}

/// Encode a MontCurve as 64 bytes (the A coefficient).
pub fn encode_curve(curve: &MontCurve) -> [u8; 64] {
    encode_fq(&curve.a)
}

/// Decode a MontCurve from 64 bytes.
pub fn decode_curve(bytes: &[u8; 64]) -> Option<MontCurve> {
    decode_fq(bytes).map(|a| MontCurve { a })
}

/// Encode an Ideal as 74 bytes.
/// Each exponent e_i in [-m, m] is stored as (e_i + m) as an unsigned byte.
pub fn encode_ideal(ideal: &Ideal) -> [u8; NUM_PRIMES] {
    let mut out = [0u8; NUM_PRIMES];
    let mut i = 0;
    while i < NUM_PRIMES {
        out[i] = (ideal.exponents[i] + MAX_EXPONENT) as u8;
        i += 1;
    }
    out
}

/// Decode an Ideal from 74 bytes.
/// Returns None if any byte exceeds 2*MAX_EXPONENT.
pub fn decode_ideal(bytes: &[u8; NUM_PRIMES]) -> Option<Ideal> {
    let mut exponents = [0i8; NUM_PRIMES];
    let max_val = (2 * MAX_EXPONENT) as u8;
    let mut i = 0;
    while i < NUM_PRIMES {
        if bytes[i] > max_val {
            return None;
        }
        exponents[i] = (bytes[i] as i8) - MAX_EXPONENT;
        i += 1;
    }
    Some(Ideal { exponents })
}
