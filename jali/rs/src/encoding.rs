// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Serialization of ring elements.
//!
//! Each coefficient is stored as 8 bytes (little-endian canonical u64).
//! A ring element of dimension n serializes to n * 8 bytes.

use nebu::Goldilocks;
use crate::ring::RingElement;

/// Encode a ring element into a byte buffer.
///
/// Writes n * 8 bytes in little-endian format.
/// Returns the number of bytes written.
pub fn encode_ring(elem: &RingElement, out: &mut [u8]) -> usize {
    let n = elem.n;
    let needed = n * 8;
    assert!(out.len() >= needed, "output buffer too small");
    for i in 0..n {
        let bytes = elem.coeffs[i].as_u64().to_le_bytes();
        out[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }
    needed
}

/// Decode a ring element from a byte buffer.
///
/// Reads n * 8 bytes in little-endian format.
pub fn decode_ring(bytes: &[u8], n: usize) -> RingElement {
    let needed = n * 8;
    assert!(bytes.len() >= needed, "input buffer too small");
    let mut elem = RingElement::new(n);
    for i in 0..n {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
        elem.coeffs[i] = Goldilocks::new(u64::from_le_bytes(buf));
    }
    elem
}
