//! Fiat-Shamir transcript via hemera sponge.
//!
//! All challenges in the commitment protocol derive from hemera.
//! No other hash function appears anywhere in the protocol.

use cyber_hemera::{Hash, Hasher};
use nebu::field::P;
use nebu::{Fp3, Goldilocks};

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
    /// Reduces hash output bytes into a field element.
    /// The reduction method depends on the field:
    /// - Goldilocks (64-bit): take low 8 bytes, reduce mod p
    /// - F₂¹²⁸ (128-bit): take low 16 bytes
    /// - F_q (512-bit): use all available bytes
    pub fn squeeze_field<F: Reduce>(&mut self) -> F {
        let hash = self.squeeze();
        F::reduce(hash.as_bytes())
    }

    /// Absorb a Goldilocks element as its canonical little-endian u64.
    pub fn absorb_goldilocks(&mut self, x: Goldilocks) {
        self.absorb(&x.as_u64().to_le_bytes());
    }

    /// Absorb an Fp3 element as three canonical little-endian u64 limbs
    /// (c0, c1, c2).
    pub fn absorb_fp3(&mut self, x: Fp3) {
        let mut buf = [0u8; 24];
        buf[..8].copy_from_slice(&x.c0.as_u64().to_le_bytes());
        buf[8..16].copy_from_slice(&x.c1.as_u64().to_le_bytes());
        buf[16..].copy_from_slice(&x.c2.as_u64().to_le_bytes());
        self.absorb(&buf);
    }

    /// Absorb a slice of Fp3 elements.
    pub fn absorb_fp3_slice(&mut self, xs: &[Fp3]) {
        for &x in xs {
            self.absorb_fp3(x);
        }
    }

    /// Absorb a u64 as little-endian bytes (lengths, nonces, sizes).
    pub fn absorb_u64(&mut self, x: u64) {
        self.absorb(&x.to_le_bytes());
    }

    /// Squeeze uniformly distributed Goldilocks limbs.
    ///
    /// A hemera squeeze is 32 bytes: the canonical little-endian encoding
    /// of four output elements of the Poseidon2 state, each in `[0, p)`.
    /// In the ideal-permutation model each limb is uniform on `F_p` and the
    /// limbs are independent, so taking a limb as a field element has
    /// statistical distance 0 from uniform. A limb `>= p` cannot occur for
    /// hemera output; should it ever, it is rejected (rejection sampling),
    /// never reduced, so the output distribution stays exactly uniform.
    pub fn squeeze_goldilocks_limbs(&mut self, out: &mut [Goldilocks]) {
        let mut filled = 0;
        while filled < out.len() {
            let hash = self.squeeze();
            for chunk in hash.as_bytes().chunks_exact(8) {
                if filled == out.len() {
                    break;
                }
                let v = u64::from_le_bytes(chunk.try_into().expect("8-byte chunk"));
                if v < P {
                    out[filled] = Goldilocks::new(v);
                    filled += 1;
                }
            }
        }
    }

    /// Squeeze an Fp3 challenge: three independent uniform Goldilocks
    /// limbs `(c0, c1, c2)` from one hemera squeeze (see
    /// `squeeze_goldilocks_limbs`; bias 0 in the ideal-permutation model).
    pub fn squeeze_fp3(&mut self) -> Fp3 {
        let mut l = [Goldilocks::ZERO; 3];
        self.squeeze_goldilocks_limbs(&mut l);
        Fp3::new(l[0], l[1], l[2])
    }

    /// Squeeze `count` independent uniform indices in `[0, 2^log_n)`.
    ///
    /// Each index comes from one uniform limb `v ∈ [0, p)` by rejection:
    /// accept iff `v < ⌊p / 2^log_n⌋ · 2^log_n`, output `v mod 2^log_n`.
    /// Accepted values are exactly uniform; the rejection probability is
    /// below `2^log_n / p ≤ 2^-31` for `log_n ≤ 32`.
    pub fn squeeze_indices(&mut self, count: usize, log_n: u32) -> Vec<usize> {
        assert!(log_n <= 32, "index range above 2^32");
        let n = 1u64 << log_n;
        let limit = (P / n) * n;
        let mut out = Vec::with_capacity(count);
        let mut buf = [Goldilocks::ZERO; 4];
        while out.len() < count {
            self.squeeze_goldilocks_limbs(&mut buf);
            for x in buf {
                if out.len() == count {
                    break;
                }
                let v = x.as_u64();
                if v < limit {
                    out.push((v % n) as usize);
                }
            }
        }
        out
    }
}
