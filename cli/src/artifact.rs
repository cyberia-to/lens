//! Opening artifact codec.
//!
//! The self-describing file written by `open`/`open-batch` and read by
//! `verify`: `magic · version · algo · num_vars · points · opening`. The
//! commitment is NOT in the artifact — the verifier supplies it independently
//! (specs/cli.md §6). A single opening carries one point; a batch opening
//! carries many. Hand-rolled so no crate gains a serde requirement. Every
//! read is bounds-checked (quality passes 6, 7).

use cyber_hemera::Hash;
use lens::{Commitment, Opening};

use crate::field::CliField;

const MAGIC: &[u8; 4] = b"LENS";
const VERSION: u8 = 2;

pub const TAG_BRAKEDOWN: u8 = 0;
pub const TAG_IKAT: u8 = 1;
pub const TAG_BINIUS: u8 = 2;
pub const TAG_PORPHYRY: u8 = 3;

const OPENING_TENSOR: u8 = 0;
const OPENING_FOLDING: u8 = 1;

/// A decoded artifact over field `F`: everything `verify` needs but the commitment.
pub struct Artifact<F: CliField> {
    pub num_vars: usize,
    /// True for a batch opening (`batch_open`/`batch_verify`), false for a
    /// single opening (`open`/`verify`). The two verify paths differ per
    /// construction, so the mode must be recorded, not inferred from count.
    pub batch: bool,
    /// One `(point, claimed value)` pair per opening — length 1 for a single
    /// open, `m` for a batch.
    pub points: Vec<(Vec<F>, F)>,
    pub opening: Opening,
}

/// Read the construction tag without decoding the whole artifact, so `verify`
/// can pick the right field before parsing the field-typed body.
pub fn peek_algo(bytes: &[u8]) -> Result<u8, String> {
    if bytes.len() < 6 {
        return Err("artifact truncated".into());
    }
    if &bytes[0..4] != MAGIC {
        return Err("not a lens opening artifact (bad magic)".into());
    }
    if bytes[4] != VERSION {
        return Err("unsupported artifact version".into());
    }
    Ok(bytes[5])
}

pub fn encode<F: CliField>(
    algo_tag: u8,
    num_vars: usize,
    batch: bool,
    points: &[(Vec<F>, F)],
    opening: &Opening,
) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(algo_tag);
    out.push(u8::try_from(num_vars).map_err(|_| "num_vars too large".to_string())?);
    out.push(batch as u8);

    put_u32(&mut out, points.len());
    for (point, value) in points {
        if point.len() != num_vars {
            return Err("point dimension mismatch".into());
        }
        for coord in point {
            out.extend_from_slice(&coord.to_le());
        }
        out.extend_from_slice(&value.to_le());
    }

    encode_opening(&mut out, opening)?;
    Ok(out)
}

pub fn decode<F: CliField>(bytes: &[u8]) -> Result<Artifact<F>, String> {
    let mut r = Reader::new(bytes);
    if r.take(4)? != MAGIC {
        return Err("not a lens opening artifact (bad magic)".into());
    }
    if r.u8()? != VERSION {
        return Err("unsupported artifact version".into());
    }
    let _algo = r.u8()?;
    let num_vars = r.u8()? as usize;
    let batch = r.u8()? != 0;

    let n_points = r.u32()?;
    let mut points = Vec::with_capacity(n_points);
    for _ in 0..n_points {
        let mut point = Vec::with_capacity(num_vars);
        for _ in 0..num_vars {
            point.push(F::from_le(r.take(F::WIDTH)?));
        }
        let value = F::from_le(r.take(F::WIDTH)?);
        points.push((point, value));
    }

    let opening = decode_opening(&mut r)?;
    Ok(Artifact {
        num_vars,
        batch,
        points,
        opening,
    })
}

// ── opening (variant-tagged) ─────────────────────────────────────

fn encode_opening(out: &mut Vec<u8>, opening: &Opening) -> Result<(), String> {
    match opening {
        Opening::Tensor {
            round_commitments,
            final_poly,
            query_responses,
        } => {
            out.push(OPENING_TENSOR);
            put_commitments(out, round_commitments);
            put_len_bytes(out, final_poly);
            put_u32(out, query_responses.len());
            for (idx, val) in query_responses {
                put_u32(out, *idx);
                put_len_bytes(out, val);
            }
        }
        Opening::Folding {
            round_commitments,
            merkle_paths,
            final_value,
        } => {
            out.push(OPENING_FOLDING);
            put_commitments(out, round_commitments);
            put_u32(out, merkle_paths.len());
            for path in merkle_paths {
                put_u32(out, path.len());
                for h in path {
                    out.extend_from_slice(h.as_bytes());
                }
            }
            put_len_bytes(out, final_value);
        }
        Opening::Witness { .. } => {
            return Err("Witness openings are handled by the assayer commands".into());
        }
    }
    Ok(())
}

fn decode_opening(r: &mut Reader) -> Result<Opening, String> {
    match r.u8()? {
        OPENING_TENSOR => {
            let round_commitments = take_commitments(r)?;
            let final_poly = r.len_bytes()?.to_vec();
            let n_q = r.u32()?;
            let mut query_responses = Vec::with_capacity(n_q);
            for _ in 0..n_q {
                let idx = r.u32()?;
                let val = r.len_bytes()?.to_vec();
                query_responses.push((idx, val));
            }
            Ok(Opening::Tensor {
                round_commitments,
                final_poly,
                query_responses,
            })
        }
        OPENING_FOLDING => {
            let round_commitments = take_commitments(r)?;
            let n_paths = r.u32()?;
            let mut merkle_paths = Vec::with_capacity(n_paths);
            for _ in 0..n_paths {
                let n = r.u32()?;
                let mut path = Vec::with_capacity(n);
                for _ in 0..n {
                    path.push(take_hash(r)?);
                }
                merkle_paths.push(path);
            }
            let final_value = r.len_bytes()?.to_vec();
            Ok(Opening::Folding {
                round_commitments,
                merkle_paths,
                final_value,
            })
        }
        other => Err(format!("unknown opening variant {other}")),
    }
}

fn put_commitments(out: &mut Vec<u8>, cs: &[Commitment]) {
    put_u32(out, cs.len());
    for c in cs {
        out.extend_from_slice(c.as_bytes());
    }
}

fn take_commitments(r: &mut Reader) -> Result<Vec<Commitment>, String> {
    let n = r.u32()?;
    let mut cs = Vec::with_capacity(n);
    for _ in 0..n {
        cs.push(Commitment(take_hash(r)?));
    }
    Ok(cs)
}

fn take_hash(r: &mut Reader) -> Result<Hash, String> {
    let h: [u8; 32] = r
        .take(32)?
        .try_into()
        .map_err(|_| "short hash".to_string())?;
    Ok(Hash::from_bytes(h))
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

    fn len_bytes(&mut self) -> Result<&'a [u8], String> {
        let n = self.u32()?;
        self.take(n)
    }
}
