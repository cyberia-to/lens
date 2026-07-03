//! Opening artifact codec.
//!
//! The self-describing file written by `open` and read by `verify`:
//! `magic · version · algo · num_vars · point[ν] · value · opening`.
//! The commitment is NOT in the artifact — the verifier supplies it
//! independently (specs/cli.md §6). Hand-rolled here so no dependency
//! gains a serde requirement.

use cyber_hemera::Hash;
use lens::{Commitment, Opening};
use nebu::Goldilocks;

const MAGIC: &[u8; 4] = b"LENS";
const VERSION: u8 = 1;

/// Construction tag stored in the artifact. Kept in sync with `Algo`.
pub const TAG_BRAKEDOWN: u8 = 0;
pub const TAG_IKAT: u8 = 1;

/// A decoded artifact: everything `verify` needs except the commitment.
pub struct Artifact {
    pub algo_tag: u8,
    pub num_vars: usize,
    pub point: Vec<Goldilocks>,
    pub value: Goldilocks,
    pub opening: Opening,
}

/// Serialize an artifact. Only the `Tensor` opening is supported (the
/// Goldilocks constructions); a non-Tensor opening is a caller bug.
pub fn encode(
    algo_tag: u8,
    point: &[Goldilocks],
    value: Goldilocks,
    opening: &Opening,
) -> Result<Vec<u8>, String> {
    let Opening::Tensor {
        round_commitments,
        final_poly,
        query_responses,
    } = opening
    else {
        return Err("cli only serializes Tensor openings".into());
    };

    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(algo_tag);
    out.push(u8::try_from(point.len()).map_err(|_| "point too long".to_string())?);
    for p in point {
        out.extend_from_slice(&p.as_u64().to_le_bytes());
    }
    out.extend_from_slice(&value.as_u64().to_le_bytes());

    put_u32(&mut out, round_commitments.len());
    for c in round_commitments {
        out.extend_from_slice(c.as_bytes());
    }
    put_len_bytes(&mut out, final_poly);
    put_u32(&mut out, query_responses.len());
    for (idx, val) in query_responses {
        put_u32(&mut out, *idx);
        put_len_bytes(&mut out, val);
    }
    Ok(out)
}

/// Parse an artifact. Every read is bounds-checked — malformed input
/// returns an error rather than panicking (quality pass 6, 7).
pub fn decode(bytes: &[u8]) -> Result<Artifact, String> {
    let mut r = Reader::new(bytes);
    if r.take(4)? != MAGIC {
        return Err("not a lens opening artifact (bad magic)".into());
    }
    if r.u8()? != VERSION {
        return Err("unsupported artifact version".into());
    }
    let algo_tag = r.u8()?;
    let num_vars = r.u8()? as usize;
    let mut point = Vec::with_capacity(num_vars);
    for _ in 0..num_vars {
        point.push(r.field()?);
    }
    let value = r.field()?;

    let n_rc = r.u32()?;
    let mut round_commitments = Vec::with_capacity(n_rc);
    for _ in 0..n_rc {
        let h: [u8; 32] = r
            .take(32)?
            .try_into()
            .map_err(|_| "short commitment".to_string())?;
        round_commitments.push(Commitment(Hash::from_bytes(h)));
    }
    let final_poly = r.len_bytes()?.to_vec();
    let n_q = r.u32()?;
    let mut query_responses = Vec::with_capacity(n_q);
    for _ in 0..n_q {
        let idx = r.u32()?;
        let val = r.len_bytes()?.to_vec();
        query_responses.push((idx, val));
    }

    Ok(Artifact {
        algo_tag,
        num_vars,
        point,
        value,
        opening: Opening::Tensor {
            round_commitments,
            final_poly,
            query_responses,
        },
    })
}

fn put_u32(out: &mut Vec<u8>, v: usize) {
    out.extend_from_slice(&(v as u32).to_le_bytes());
}

fn put_len_bytes(out: &mut Vec<u8>, b: &[u8]) {
    put_u32(out, b.len());
    out.extend_from_slice(b);
}

/// Bounds-checked forward reader over a byte slice.
struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).ok_or("length overflow")?;
        if end > self.b.len() {
            return Err("artifact truncated".into());
        }
        let s = &self.b[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<usize, String> {
        let a: [u8; 4] = self.take(4)?.try_into().unwrap();
        Ok(u32::from_le_bytes(a) as usize)
    }

    fn field(&mut self) -> Result<Goldilocks, String> {
        let a: [u8; 8] = self.take(8)?.try_into().unwrap();
        Ok(Goldilocks::new(u64::from_le_bytes(a)))
    }

    fn len_bytes(&mut self) -> Result<&'a [u8], String> {
        let n = self.u32()?;
        self.take(n)
    }
}
