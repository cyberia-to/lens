//! Field abstraction for the cli.
//!
//! Each construction commits over a different field. `CliField` captures the
//! per-field operations the cli needs — file encoding, canonical byte I/O, and
//! coordinate parsing — so the command logic stays generic over the algebra.

use lens::{Field, Reduce};
use nebu::Goldilocks;
use nebu::encoding::encode_7;

use genies::Fq;
use kuro::F2_128;

/// Per-field byte widths, file encoding, and coordinate parsing.
/// `Reduce` lets the transcript squeeze challenges of this field.
pub trait CliField: Field + Reduce + Copy {
    /// Canonical serialized width of one element, in bytes.
    const WIDTH: usize;
    /// File bytes consumed per element — kept below the modulus so every
    /// chunk encodes canonically.
    const INPUT_CHUNK: usize;

    /// Encode up to `INPUT_CHUNK` file bytes into one element.
    fn encode_chunk(chunk: &[u8]) -> Self;
    /// Canonical little-endian bytes (`WIDTH` long).
    fn to_le(&self) -> Vec<u8>;
    /// Read one element from `WIDTH` little-endian bytes.
    fn from_le(bytes: &[u8]) -> Self;
    /// Parse one coordinate from a decimal (Goldilocks only) or hex literal.
    fn parse(s: &str) -> Result<Self, String>;
}

impl CliField for Goldilocks {
    const WIDTH: usize = 8;
    const INPUT_CHUNK: usize = 7;

    fn encode_chunk(chunk: &[u8]) -> Self {
        encode_7(chunk)
    }

    fn to_le(&self) -> Vec<u8> {
        self.as_u64().to_le_bytes().to_vec()
    }

    fn from_le(bytes: &[u8]) -> Self {
        let mut b = [0u8; 8];
        b.copy_from_slice(&bytes[..8]);
        Goldilocks::new(u64::from_le_bytes(b))
    }

    fn parse(s: &str) -> Result<Self, String> {
        let v = if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            u64::from_str_radix(h, 16).map_err(|_| format!("invalid hex '{s}'"))?
        } else {
            s.parse::<u64>()
                .map_err(|_| format!("invalid number '{s}'"))?
        };
        if v >= nebu::field::P {
            return Err(format!("'{s}' is not canonical (≥ p)"));
        }
        Ok(Goldilocks::new(v))
    }
}

impl CliField for F2_128 {
    const WIDTH: usize = 16;
    const INPUT_CHUNK: usize = 16;

    fn encode_chunk(chunk: &[u8]) -> Self {
        let mut buf = [0u8; 16];
        let n = chunk.len().min(16);
        buf[..n].copy_from_slice(&chunk[..n]);
        F2_128(u128::from_le_bytes(buf))
    }

    fn to_le(&self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }

    fn from_le(bytes: &[u8]) -> Self {
        let mut b = [0u8; 16];
        b.copy_from_slice(&bytes[..16]);
        F2_128(u128::from_le_bytes(b))
    }

    fn parse(s: &str) -> Result<Self, String> {
        // A binary-tower element has no modulus — any 128-bit value is valid.
        let le = hex_number_le(s, 16)?;
        Ok(Self::from_le(&le))
    }
}

impl CliField for Fq {
    const WIDTH: usize = 64;
    const INPUT_CHUNK: usize = 63; // 63·8 = 504 bits < 511-bit prime → canonical

    fn encode_chunk(chunk: &[u8]) -> Self {
        let mut buf = [0u8; 64];
        let n = chunk.len().min(63);
        buf[..n].copy_from_slice(&chunk[..n]);
        Self::from_le(&buf)
    }

    fn to_le(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64);
        for limb in self.limbs {
            out.extend_from_slice(&limb.to_le_bytes());
        }
        out
    }

    fn from_le(bytes: &[u8]) -> Self {
        let mut limbs = [0u64; 8];
        for (i, chunk) in bytes[..64].chunks_exact(8).enumerate() {
            let mut b = [0u8; 8];
            b.copy_from_slice(chunk);
            limbs[i] = u64::from_le_bytes(b);
        }
        Fq::from_limbs(limbs)
    }

    fn parse(s: &str) -> Result<Self, String> {
        let le = hex_number_le(s, 64)?;
        let mut limbs = [0u64; 8];
        for (i, chunk) in le.chunks_exact(8).enumerate() {
            limbs[i] = u64::from_le_bytes(chunk.try_into().unwrap());
        }
        // Canonicalize into [0, q); a hand-typed value ≥ q is reduced.
        Ok(Fq::reduce(&limbs))
    }
}

/// Parse a hex number into `width` little-endian bytes.
/// The literal is a number (big-endian hex digits); the result is its
/// canonical little-endian byte encoding, zero-padded to `width`.
fn hex_number_le(s: &str, width: usize) -> Result<Vec<u8>, String> {
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    let s = if s.is_empty() { "0" } else { s };
    let padded = if s.len().is_multiple_of(2) {
        s.to_string()
    } else {
        format!("0{s}")
    };
    let mut be = Vec::with_capacity(padded.len() / 2);
    for i in (0..padded.len()).step_by(2) {
        be.push(
            u8::from_str_radix(&padded[i..i + 2], 16).map_err(|_| format!("invalid hex '{s}'"))?,
        );
    }
    if be.len() > width {
        return Err(format!("value too wide for {width}-byte field"));
    }
    let mut le: Vec<u8> = be.into_iter().rev().collect();
    le.resize(width, 0);
    Ok(le)
}
