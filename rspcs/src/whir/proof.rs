//! WHIR proof and its canonical wire layout.
//!
//! ```text
//! header        8 B    WhirParams::header
//! ood0          exts   answers f̂_0(z_{0,j})
//! sumcheck0     exts   (h(0), h(2)) per folding round of round 0
//! fold_nonces0  u64s   one per folding round if fold_pow_0 > 0, else none
//! rounds        u32 R, then per round i = 1..R:
//!   root          32 B   Merkle root of f_i
//!   ood           exts   answers f̂_i(z_{i,j})
//!   query_nonce   u64s   0|1
//!   opening       Opening of f_{i−1} (base symbols iff i = 1)
//!   sumcheck      exts
//!   fold_nonces   u64s
//! final_poly    exts   monomial coefficients of f̂_M
//! final_nonce   u64s   0|1
//! final_opening Opening of f_{M−1} (base symbols iff R = 0)
//!
//! Opening:   u32 width, u32 count + count symbols (u64 or 3×u64),
//!            u32 d + d × 32 B siblings
//! ```

use cyber_hemera::Hash;
use cyber_lens_core::PcsError;
use nebu::Fp3;

use super::WhirParams;
use crate::wire::{Reader, Symbols, Writer};

/// Opened folding cosets with their Merkle multi-opening.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opening {
    pub width: usize,
    pub symbols: Symbols,
    pub siblings: Vec<Hash>,
}

impl Opening {
    /// Canonical bytes: `u32 width ‖ symbols ‖ u32 d ‖ d × 32 B`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::default();
        self.write(&mut w);
        w.buf
    }
    /// Parse exactly one opening whose symbols are Fp3 iff `ext`.
    pub fn from_bytes(bytes: &[u8], ext: bool) -> Result<Self, PcsError> {
        let mut r = Reader::new(bytes);
        let o = Self::read(&mut r, ext)?;
        r.finish()?;
        Ok(o)
    }
    fn write(&self, w: &mut Writer) {
        w.u32(self.width);
        self.symbols.write(w);
        w.digests(&self.siblings);
    }
    fn read(r: &mut Reader<'_>, ext: bool) -> Result<Self, PcsError> {
        Ok(Self {
            width: r.u32()?,
            symbols: Symbols::read(r, ext)?,
            siblings: r.digests()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhirRoundProof {
    pub root: Hash,
    pub ood: Vec<Fp3>,
    pub query_nonce: Vec<u64>,
    pub opening: Opening,
    pub sumcheck: Vec<Fp3>,
    pub fold_nonces: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhirProof {
    pub params: WhirParams,
    pub ood0: Vec<Fp3>,
    pub sumcheck0: Vec<Fp3>,
    pub fold_nonces0: Vec<u64>,
    pub rounds: Vec<WhirRoundProof>,
    pub final_poly: Vec<Fp3>,
    pub final_nonce: Vec<u64>,
    pub final_opening: Opening,
}

impl WhirProof {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.buf.extend_from_slice(&self.params.header());
        w.exts(&self.ood0);
        w.exts(&self.sumcheck0);
        w.u64s(&self.fold_nonces0);
        w.u32(self.rounds.len());
        for rd in &self.rounds {
            w.digest(&rd.root);
            w.exts(&rd.ood);
            w.u64s(&rd.query_nonce);
            rd.opening.write(&mut w);
            w.exts(&rd.sumcheck);
            w.u64s(&rd.fold_nonces);
        }
        w.exts(&self.final_poly);
        w.u64s(&self.final_nonce);
        self.final_opening.write(&mut w);
        w.buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PcsError> {
        Self::from_bytes_with(bytes, false)
    }

    /// Whether the openings of `f_0` carry Fp3 symbols (a proof of an Fp3
    /// commitment).
    pub fn opens_ext0(&self) -> bool {
        let first = self
            .rounds
            .first()
            .map_or(&self.final_opening, |r| &r.opening);
        matches!(first.symbols, Symbols::Ext(_))
    }

    /// Parse a proof of an Fp3 commitment (`ext0 = true`: the openings of
    /// `f_0` carry `3×u64` symbols) or of a base one.
    pub fn from_bytes_with(bytes: &[u8], ext0: bool) -> Result<Self, PcsError> {
        let mut r = Reader::new(bytes);
        let mut h = [0u8; 8];
        for b in &mut h {
            *b = r.u8()?;
        }
        let params = WhirParams::from_header(h)?;
        let ood0 = r.exts()?;
        let sumcheck0 = r.exts()?;
        let fold_nonces0 = r.u64s()?;
        let n = r.count(32)?;
        let mut rounds = Vec::with_capacity(n);
        for i in 0..n {
            rounds.push(WhirRoundProof {
                root: r.digest()?,
                ood: r.exts()?,
                query_nonce: r.u64s()?,
                opening: Opening::read(&mut r, i > 0 || ext0)?,
                sumcheck: r.exts()?,
                fold_nonces: r.u64s()?,
            });
        }
        let final_poly = r.exts()?;
        let final_nonce = r.u64s()?;
        let final_opening = Opening::read(&mut r, !rounds.is_empty() || ext0)?;
        r.finish()?;
        Ok(Self {
            params,
            ood0,
            sumcheck0,
            fold_nonces0,
            rounds,
            final_poly,
            final_nonce,
            final_opening,
        })
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::WhirProof;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Serde form: the wire bytes for a base proof (unchanged), and
    /// `EXT_MARK ‖ wire bytes` for a proof of an Fp3 commitment — a base
    /// proof starts with the header's version byte 1, never the mark.
    const EXT_MARK: u8 = 0xE3;

    impl Serialize for WhirProof {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut bytes = Vec::new();
            if self.opens_ext0() {
                bytes.push(EXT_MARK);
            }
            bytes.extend(self.to_bytes());
            s.serialize_bytes(&bytes)
        }
    }

    impl<'de> Deserialize<'de> for WhirProof {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let bytes = crate::wire::deserialize_bytes(d)?;
            match bytes.split_first() {
                Some((&EXT_MARK, rest)) => WhirProof::from_bytes_with(rest, true),
                _ => WhirProof::from_bytes(&bytes),
            }
            .map_err(D::Error::custom)
        }
    }
}
