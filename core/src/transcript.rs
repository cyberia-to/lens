//! Fiat-Shamir transcript via hemera sponge.
//!
//! All challenges in the commitment protocol derive from hemera.
//! No other hash function appears anywhere in the protocol.

use cyber_hemera::{Hash, Hasher};

use cyb_algebra_proof::Reduce;

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
    /// Reduces hash output bytes into a field element.
    /// The reduction method depends on the field:
    /// - Goldilocks (64-bit): take low 8 bytes, reduce mod p
    /// - F₂¹²⁸ (128-bit): take low 16 bytes
    /// - F_q (512-bit): use all available bytes
    pub fn squeeze_field<F: Reduce>(&mut self) -> F {
        let hash = self.squeeze();
        F::reduce(hash.as_bytes())
    }
}
