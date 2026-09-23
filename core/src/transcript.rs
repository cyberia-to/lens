//! Fiat-Shamir transcript via hemera sponge.
//!
//! All challenges in the commitment protocol derive from hemera.
//! No other hash function appears anywhere in the protocol.

use cyber_hemera::{Hash, Hasher};

use strata_core::Codec;
use strata_proof::Reduce;

/// Fiat-Shamir transcript for non-interactive proofs.
///
/// Wraps a hemera hasher in sponge mode: absorb data, squeeze challenges.
#[derive(Clone)]
pub struct Transcript {
    hasher: Hasher,
}

impl Transcript {
    /// Create a new transcript with domain separation.
    pub fn new(domain: &[u8]) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(domain);
        Self { hasher }
    }

    /// Absorb data into the transcript.
    pub fn absorb(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    /// Squeeze a 32-byte challenge from the transcript.
    pub fn squeeze(&mut self) -> Hash {
        let hash = self.hasher.finalize();
        // re-seed: absorb the output to chain the state
        self.hasher = Hasher::new();
        self.hasher.update(hash.as_bytes());
        hash
    }

    /// Squeeze a field element challenge.
    ///
    /// Reduces hash output bytes into a field element. `squeeze` yields
    /// 32 bytes per call; fields whose canonical encoding needs more than
    /// that (`F::byte_len()`, e.g. porphyry's 512-bit F_q at 64 bytes) are
    /// fed the concatenation of as many squeezes as are needed, chained
    /// through the same re-seeded sponge state.
    pub fn squeeze_field<F: Reduce + Codec>(&mut self) -> F {
        let needed = F::byte_len();
        let mut bytes = Vec::with_capacity(needed.max(32));
        while bytes.len() < needed {
            bytes.extend_from_slice(self.squeeze().as_bytes());
        }
        F::reduce(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genies::Fq;
    use kuro::F2_128;
    use nebu::Goldilocks;

    // Fq::byte_len() is 64, more than one 32-byte squeeze can supply — this
    // is exactly the shape that used to panic with "need at least 64 bytes
    // for Fq" (strata-genies Reduce::reduce's own bound check).
    #[test]
    fn squeeze_field_wide_field_does_not_panic() {
        let mut t = Transcript::new(b"test");
        let _: Fq = t.squeeze_field();
    }

    #[test]
    fn squeeze_field_wide_field_is_deterministic() {
        let a: Fq = Transcript::new(b"test").squeeze_field();
        let b: Fq = Transcript::new(b"test").squeeze_field();
        assert_eq!(a, b);
    }

    #[test]
    fn squeeze_field_successive_wide_challenges_differ() {
        let mut t = Transcript::new(b"test");
        let a: Fq = t.squeeze_field();
        let b: Fq = t.squeeze_field();
        assert_ne!(a, b);
    }

    // Narrower fields (byte_len() <= 32) must still consume exactly one
    // squeeze, matching pre-fix behavior for Goldilocks and F2_128 exactly.
    #[test]
    fn squeeze_field_narrow_field_matches_single_squeeze() {
        let mut t = Transcript::new(b"test");
        let hash = t.clone().squeeze();
        let direct = Goldilocks::reduce(hash.as_bytes());
        let via_squeeze_field: Goldilocks = t.squeeze_field();
        assert_eq!(direct, via_squeeze_field);
    }

    #[test]
    fn squeeze_field_f2_128_does_not_panic_and_is_deterministic() {
        let a: F2_128 = Transcript::new(b"test").squeeze_field();
        let b: F2_128 = Transcript::new(b"test").squeeze_field();
        assert_eq!(a, b);
    }
}
