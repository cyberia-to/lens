//! Field trait implementations for algebra crates.
//!
//! Each impl is gated behind an optional dependency feature.
//! This avoids orphan rule issues (Field is defined here, algebra types are foreign).

use crate::Field;

#[cfg(feature = "nebu")]
impl Field for nebu::Goldilocks {
    const ZERO: Self = nebu::Goldilocks::ZERO;
    const ONE: Self = nebu::Goldilocks::ONE;

    #[inline]
    fn inv(self) -> Self {
        nebu::Goldilocks::inv(self)
    }

    fn from_hash(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= 8, "need at least 8 bytes for Goldilocks");
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&bytes[..8]);
        let val = u64::from_le_bytes(buf);
        nebu::Goldilocks::new(val).canonicalize()
    }
}

#[cfg(feature = "kuro")]
impl Field for kuro::F2_128 {
    const ZERO: Self = kuro::F2_128::ZERO;
    const ONE: Self = kuro::F2_128::ONE;

    #[inline]
    fn inv(self) -> Self {
        kuro::F2_128::inv(self)
    }

    fn from_hash(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= 16, "need at least 16 bytes for F2_128");
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&bytes[..16]);
        kuro::F2_128(u128::from_le_bytes(buf))
    }
}

#[cfg(feature = "genies")]
impl Field for genies::Fq {
    const ZERO: Self = genies::Fq::ZERO;
    const ONE: Self = genies::Fq::ONE;

    #[inline]
    fn inv(self) -> Self {
        genies::Fq::inv(&self)
    }

    fn from_hash(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= 64, "need at least 64 bytes for Fq");
        let mut limbs = [0u64; 8];
        for (i, chunk) in bytes[..64].chunks_exact(8).enumerate() {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(chunk);
            limbs[i] = u64::from_le_bytes(buf);
        }
        genies::Fq::reduce(&genies::Fq::from_limbs(limbs).limbs)
    }
}
