//! Canonical byte encoding of proofs.
//!
//! Little-endian throughout. A Goldilocks element is its canonical u64
//! (`< p`, anything else is rejected). An Fp3 element is three such limbs
//! `(c0, c1, c2)`. A hemera digest is 32 bytes = four canonical limbs
//! (non-canonical limbs would alias under hemera's chaining-value decoding
//! and are rejected). Counts are u32. A reader must consume the input
//! exactly; trailing bytes are rejected.

use cyber_hemera::Hash;
use cyber_lens_core::PcsError;
use nebu::field::P;
use nebu::{Fp3, Goldilocks};

#[derive(Default)]
pub struct Writer {
    pub buf: Vec<u8>,
}

impl Writer {
    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    pub fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u32(&mut self, v: usize) {
        let v = u32::try_from(v).expect("count fits u32");
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn base(&mut self, x: Goldilocks) {
        self.u64(x.as_u64());
    }
    pub fn ext(&mut self, x: Fp3) {
        self.base(x.c0);
        self.base(x.c1);
        self.base(x.c2);
    }
    pub fn digest(&mut self, h: &Hash) {
        self.buf.extend_from_slice(h.as_bytes());
    }
    pub fn exts(&mut self, xs: &[Fp3]) {
        self.u32(xs.len());
        for &x in xs {
            self.ext(x);
        }
    }
    pub fn u64s(&mut self, xs: &[u64]) {
        self.u32(xs.len());
        for &x in xs {
            self.u64(x);
        }
    }
    pub fn digests(&mut self, hs: &[Hash]) {
        self.u32(hs.len());
        for h in hs {
            self.digest(h);
        }
    }
}

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

type R<T> = Result<T, PcsError>;

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    fn take(&mut self, n: usize) -> R<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(PcsError::Malformed)?;
        let s = self.buf.get(self.pos..end).ok_or(PcsError::Malformed)?;
        self.pos = end;
        Ok(s)
    }
    pub fn u8(&mut self) -> R<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> R<u16> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("2 bytes"),
        ))
    }
    pub fn u32(&mut self) -> R<usize> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")) as usize)
    }
    pub fn u64(&mut self) -> R<u64> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("8 bytes"),
        ))
    }
    pub fn base(&mut self) -> R<Goldilocks> {
        let v = self.u64()?;
        if v >= P {
            return Err(PcsError::Malformed);
        }
        Ok(Goldilocks::new(v))
    }
    pub fn ext(&mut self) -> R<Fp3> {
        Ok(Fp3::new(self.base()?, self.base()?, self.base()?))
    }
    pub fn digest(&mut self) -> R<Hash> {
        let s = self.take(32)?;
        for limb in s.as_chunks::<8>().0 {
            if u64::from_le_bytes(*limb) >= P {
                return Err(PcsError::Malformed);
            }
        }
        Ok(Hash::from_bytes(s.try_into().expect("32 bytes")))
    }
    /// A count, bounded by the remaining input so a corrupted length
    /// cannot trigger a huge allocation.
    pub fn count(&mut self, elem_bytes: usize) -> R<usize> {
        let n = self.u32()?;
        let need = n.checked_mul(elem_bytes).ok_or(PcsError::Malformed)?;
        if need > self.buf.len() - self.pos {
            return Err(PcsError::Malformed);
        }
        Ok(n)
    }
    pub fn exts(&mut self) -> R<Vec<Fp3>> {
        let n = self.count(24)?;
        (0..n).map(|_| self.ext()).collect()
    }
    pub fn u64s(&mut self) -> R<Vec<u64>> {
        let n = self.count(8)?;
        (0..n).map(|_| self.u64()).collect()
    }
    pub fn digests(&mut self) -> R<Vec<Hash>> {
        let n = self.count(32)?;
        (0..n).map(|_| self.digest()).collect()
    }
    pub fn finish(self) -> R<()> {
        if self.pos == self.buf.len() {
            Ok(())
        } else {
            Err(PcsError::Malformed)
        }
    }
}

/// Leaf values of one Merkle opening: `count` leaves of `width` symbols.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Symbols {
    Base(Vec<Goldilocks>),
    Ext(Vec<Fp3>),
}

impl Symbols {
    pub fn len(&self) -> usize {
        match self {
            Symbols::Base(v) => v.len(),
            Symbols::Ext(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Canonical leaf bytes of the symbols `[start, start + width)`.
    pub fn leaf_bytes(&self, start: usize, width: usize) -> Vec<u8> {
        match self {
            Symbols::Base(v) => base_bytes(&v[start..start + width]),
            Symbols::Ext(v) => ext_bytes(&v[start..start + width]),
        }
    }
    /// Symbol `i` lifted to Fp3.
    pub fn get(&self, i: usize) -> Fp3 {
        match self {
            Symbols::Base(v) => Fp3::from_base(v[i]),
            Symbols::Ext(v) => v[i],
        }
    }
    pub fn write(&self, w: &mut Writer) {
        match self {
            Symbols::Base(v) => {
                w.u32(v.len());
                for &x in v {
                    w.base(x);
                }
            }
            Symbols::Ext(v) => w.exts(v),
        }
    }
    pub fn read(r: &mut Reader<'_>, ext: bool) -> R<Self> {
        if ext {
            Ok(Symbols::Ext(r.exts()?))
        } else {
            let n = r.count(8)?;
            Ok(Symbols::Base((0..n).map(|_| r.base()).collect::<R<_>>()?))
        }
    }
}

pub fn base_bytes(xs: &[Goldilocks]) -> Vec<u8> {
    let mut b = Vec::with_capacity(xs.len() * 8);
    for &x in xs {
        b.extend_from_slice(&x.as_u64().to_le_bytes());
    }
    b
}

pub fn ext_bytes(xs: &[Fp3]) -> Vec<u8> {
    let mut b = Vec::with_capacity(xs.len() * 24);
    for &x in xs {
        b.extend_from_slice(&x.c0.as_u64().to_le_bytes());
        b.extend_from_slice(&x.c1.as_u64().to_le_bytes());
        b.extend_from_slice(&x.c2.as_u64().to_le_bytes());
    }
    b
}

/// Deserialize a byte string (accepts borrowed bytes, owned bytes or a
/// sequence of u8, depending on the serde format).
#[cfg(feature = "serde")]
pub fn deserialize_bytes<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    struct V;
    impl<'de> serde::de::Visitor<'de> for V {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("proof bytes")
        }
        fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Vec<u8>, E> {
            Ok(v.to_vec())
        }
        fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Vec<u8>, E> {
            Ok(v)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut s: A) -> Result<Vec<u8>, A::Error> {
            let mut out = Vec::new();
            while let Some(b) = s.next_element::<u8>()? {
                out.push(b);
            }
            Ok(out)
        }
    }
    d.deserialize_byte_buf(V)
}
