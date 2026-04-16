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

// Future: impl Field for kuro::F2_128, genies::Fq
