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
    /// Reduces hash output bytes into a field element. A single 32-byte
    /// squeeze is enough for Goldilocks (needs 8) and F₂¹²⁸ (needs 16), but
    /// F_q needs 64: squeeze repeatedly, chaining the sponge state each
    /// time, until `F::byte_len()` bytes are collected, then reduce exactly
    /// that many. This matches `Reduce::reduce`'s documented per-field input
    /// length instead of assuming every field fits in one squeeze.
    pub fn squeeze_field<F: Reduce + Codec>(&mut self) -> F {
        let need = F::byte_len();
        let mut buf = Vec::with_capacity(need);
        while buf.len() < need {
            buf.extend_from_slice(self.squeeze().as_bytes());
        }
        buf.truncate(need);
        F::reduce(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genies::Fq;
    use nebu::Goldilocks;

    /// A single squeeze (32 bytes) covers Fq's 64-byte requirement only
    /// after this fix chains a second squeeze; before it, this panicked
    /// with "need at least 64 bytes for Fq" (the bug `cyber-lens-porphyry`'s
    /// `batch_roundtrip` test hit on a clean checkout).
    #[test]
    fn squeeze_field_does_not_panic_for_a_wide_field() {
        let mut t = Transcript::new(b"test");
        let _: Fq = t.squeeze_field();
    }

    #[test]
    fn squeeze_field_is_deterministic_for_a_wide_field() {
        let mut a = Transcript::new(b"test");
        let mut b = Transcript::new(b"test");
        let fa: Fq = a.squeeze_field();
        let fb: Fq = b.squeeze_field();
        assert_eq!(fa, fb);
    }

    /// Narrow fields (Goldilocks needs 8 of 32 available bytes) still
    /// resolve in one squeeze — this fix must not change their challenges.
    #[test]
    fn squeeze_field_matches_manual_single_squeeze_for_a_narrow_field() {
        let mut a = Transcript::new(b"narrow");
        let mut b = Transcript::new(b"narrow");
        let expected = Goldilocks::reduce(a.squeeze().as_bytes());
        let actual: Goldilocks = b.squeeze_field();
        assert_eq!(actual, expected);
    }
}
