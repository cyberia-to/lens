//! TensorRs proof and its canonical wire layout.
//!
//! ```text
//! header        6 B   TensorRsParams::header
//! row_test      u32 n, n × Fp3        u = Σ α^i U_i
//! row_eval      u32 n, n × Fp3        y = Σ eq(z_hi, i) U_i
//! pow_nonce     u32 c (0|1), c × u64
//! height        u32                   k1
//! columns       u32 s, s × u64        opened columns, k1 symbols each,
//!                                     in increasing column index
//! siblings      u32 d, d × 32 B       Merkle multi-opening
//! ```

use cyber_hemera::Hash;
use cyber_lens_core::PcsError;
use nebu::{Fp3, Goldilocks};

use super::TensorRsParams;
use crate::wire::{Reader, Writer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TensorRsProof {
    pub params: TensorRsParams,
    pub row_test: Vec<Fp3>,
    pub row_eval: Vec<Fp3>,
    pub pow_nonce: Option<u64>,
    pub height: usize,
    pub columns: Vec<Goldilocks>,
    pub siblings: Vec<Hash>,
}

impl TensorRsProof {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.buf.extend_from_slice(&self.params.header());
        w.exts(&self.row_test);
        w.exts(&self.row_eval);
        w.u64s(self.pow_nonce.as_slice());
        w.u32(self.height);
        w.u32(self.columns.len());
        for &c in &self.columns {
            w.base(c);
        }
        w.digests(&self.siblings);
        w.buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PcsError> {
        let mut r = Reader::new(bytes);
        let mut h = [0u8; 6];
        for b in &mut h {
            *b = r.u8()?;
        }
        let params = TensorRsParams::from_header(h)?;
        let row_test = r.exts()?;
        let row_eval = r.exts()?;
        let nonces = r.u64s()?;
        if nonces.len() > 1 {
            return Err(PcsError::Malformed);
        }
        let height = r.u32()?;
        let n = r.count(8)?;
        let columns = (0..n).map(|_| r.base()).collect::<Result<Vec<_>, _>>()?;
        let siblings = r.digests()?;
        r.finish()?;
        Ok(Self {
            params,
            row_test,
            row_eval,
            pow_nonce: nonces.first().copied(),
            height,
            columns,
            siblings,
        })
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::TensorRsProof;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for TensorRsProof {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_bytes(&self.to_bytes())
        }
    }

    impl<'de> Deserialize<'de> for TensorRsProof {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let bytes = crate::wire::deserialize_bytes(d)?;
            TensorRsProof::from_bytes(&bytes).map_err(D::Error::custom)
        }
    }
}
