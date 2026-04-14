//! Byte ↔ tower element encoding.
//!
//! Converts between raw byte sequences and F₂ tower field elements.
//! Each tower level has a natural byte width:
//!   F₂¹²⁸ = 16 bytes, F₂⁶⁴ = 8 bytes, F₂³² = 4 bytes, etc.
//!
//! All encodings are little-endian.

use crate::tower::*;

/// Encode 16 bytes as an F₂¹²⁸ element (little-endian).
pub fn encode_128(bytes: &[u8; 16]) -> F2_128 {
    let mut v: u128 = 0;
    let mut i = 0;
    while i < 16 {
        v |= (bytes[i] as u128) << (i * 8);
        i += 1;
    }
    F2_128(v)
}

/// Decode an F₂¹²⁸ element to 16 bytes (little-endian).
pub fn decode_128(el: F2_128) -> [u8; 16] {
    let mut out = [0u8; 16];
    let v = el.0;
    let mut i = 0;
    while i < 16 {
        out[i] = (v >> (i * 8)) as u8;
        i += 1;
    }
    out
}

/// Encode 8 bytes as an F₂⁶⁴ element (little-endian).
pub fn encode_64(bytes: &[u8; 8]) -> F2_64 {
    let mut v: u64 = 0;
    let mut i = 0;
    while i < 8 {
        v |= (bytes[i] as u64) << (i * 8);
        i += 1;
    }
    F2_64(v)
}

/// Decode an F₂⁶⁴ element to 8 bytes (little-endian).
pub fn decode_64(el: F2_64) -> [u8; 8] {
    let mut out = [0u8; 8];
    let v = el.0;
    let mut i = 0;
    while i < 8 {
        out[i] = (v >> (i * 8)) as u8;
        i += 1;
    }
    out
}

/// Encode 4 bytes as an F₂³² element (little-endian).
pub fn encode_32(bytes: &[u8; 4]) -> F2_32 {
    let mut v: u32 = 0;
    let mut i = 0;
    while i < 4 {
        v |= (bytes[i] as u32) << (i * 8);
        i += 1;
    }
    F2_32(v)
}

/// Decode an F₂³² element to 4 bytes (little-endian).
pub fn decode_32(el: F2_32) -> [u8; 4] {
    let mut out = [0u8; 4];
    let v = el.0;
    let mut i = 0;
    while i < 4 {
        out[i] = (v >> (i * 8)) as u8;
        i += 1;
    }
    out
}

/// Encode 1 byte as an F₂⁸ element.
pub fn encode_8(byte: u8) -> F2_8 { F2_8(byte) }

/// Decode an F₂⁸ element to 1 byte.
pub fn decode_8(el: F2_8) -> u8 { el.0 }
