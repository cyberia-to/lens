// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Byte ↔ field element encoding.
//!
//! Input: 7-byte LE chunks → field elements.
//! Output: field elements → 8-byte LE canonical u64 values.

use crate::field::Goldilocks;

/// Encode up to 7 bytes (little-endian) into one field element.
/// Shorter slices are zero-padded implicitly.
#[inline]
pub fn encode_7(chunk: &[u8]) -> Goldilocks {
    let mut buf = [0u8; 8];
    let len = chunk.len().min(7);
    buf[..len].copy_from_slice(&chunk[..len]);
    Goldilocks::new(u64::from_le_bytes(buf))
}

/// Decode a field element to 8 canonical LE bytes.
#[inline]
pub fn decode_8(element: Goldilocks) -> [u8; 8] {
    element.as_u64().to_le_bytes()
}

/// Encode a byte slice into field elements using 7-byte chunks.
/// Returns the number of elements written.
pub fn bytes_to_field_elements(bytes: &[u8], out: &mut [Goldilocks]) -> usize {
    if bytes.is_empty() {
        return 0;
    }
    let n_chunks = (bytes.len() + 6) / 7;
    let n = n_chunks.min(out.len());
    for i in 0..n {
        let start = i * 7;
        let end = (start + 7).min(bytes.len());
        out[i] = encode_7(&bytes[start..end]);
    }
    n
}

/// Serialize field elements to bytes (8 bytes per element, canonical LE).
/// Returns the number of bytes written.
pub fn field_elements_to_bytes(elements: &[Goldilocks], out: &mut [u8]) -> usize {
    let n = elements.len().min(out.len() / 8);
    for i in 0..n {
        let bytes = decode_8(elements[i]);
        out[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }
    n * 8
}
