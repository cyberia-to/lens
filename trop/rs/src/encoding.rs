// ---
// tags: trop, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Binary encoding for tropical elements and matrices.
//!
//! All values are encoded as little-endian u64.

use crate::element::Tropical;
use crate::matrix::TropMatrix;

/// Decode 8 bytes (little-endian) into a tropical element.
pub fn encode_element(bytes: &[u8; 8]) -> Tropical {
    Tropical::from_u64(u64::from_le_bytes(*bytes))
}

/// Encode a tropical element as 8 bytes (little-endian).
pub fn decode_element(t: Tropical) -> [u8; 8] {
    t.as_u64().to_le_bytes()
}

/// Decode a byte slice into an n x n tropical matrix.
///
/// The slice must contain exactly `n * n * 8` bytes, where each consecutive
/// 8-byte chunk is a little-endian u64 in row-major order.
///
/// # Panics
/// Panics if the slice length does not match `n * n * 8`.
pub fn encode_matrix(n: usize, data: &[u8]) -> TropMatrix {
    assert_eq!(
        data.len(),
        n * n * 8,
        "encode_matrix: expected {} bytes, got {}",
        n * n * 8,
        data.len()
    );
    let mut m = TropMatrix::new(n);
    for i in 0..n {
        for j in 0..n {
            let offset = (i * n + j) * 8;
            let bytes: [u8; 8] = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ];
            m.set(i, j, encode_element(&bytes));
        }
    }
    m
}

/// Serialize an n x n tropical matrix to bytes (little-endian, row-major).
///
/// Returns a vector of `n * n * 8` bytes. Since we are `no_std`, this
/// writes into a caller-provided buffer.
///
/// # Panics
/// Panics if `buf.len() < n * n * 8`.
pub fn decode_matrix(m: &TropMatrix, buf: &mut [u8]) {
    let n = m.n;
    let needed = n * n * 8;
    assert!(
        buf.len() >= needed,
        "decode_matrix: buffer too small ({} < {})",
        buf.len(),
        needed
    );
    for i in 0..n {
        for j in 0..n {
            let bytes = decode_element(m.get(i, j));
            let offset = (i * n + j) * 8;
            buf[offset..offset + 8].copy_from_slice(&bytes);
        }
    }
}
