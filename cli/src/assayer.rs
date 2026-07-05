//! Assayer commands — tropical witness-verify (specs/cli.md §8).
//!
//! Assayer is not a `Lens`: the tropical semiring has no commit/open/verify.
//! It solves an optimization problem, packs the witness and LP dual
//! certificate as a Goldilocks polynomial, and commits that via Brakedown.
//! `assayer` reads a `problem.json`, checks it, and emits a witness artifact;
//! `assayer-verify` re-checks the tropical logic and the commitment.

use std::process::exit;

use lens::assayer::{Assayer, DualCertificate, Edge, TropicalWitness};
use lens::{Commitment, Opening, Transcript};
use nebu::Goldilocks;
use trop::Tropical;

use crate::commands::{derive_point, hex, hex_to_bytes};

const DOMAIN: &[u8] = b"lens/assayer/v0.1.0";
const MAGIC: &[u8; 4] = b"ASSY";
const VERSION: u8 = 1;

/// `assayer <problem.json> [-o witness.lens]`
pub fn solve(problem_path: &str, out: Option<&str>) -> Result<(), String> {
    let json = std::fs::read_to_string(problem_path).map_err(|e| format!("{problem_path}: {e}"))?;
    let (witness, cert) = parse_problem(&json)?;

    if !Assayer::verify_tropical(&witness, &cert) {
        return Err(
            "problem is not a valid tropical witness (structural / cost / dual check failed)"
                .into(),
        );
    }

    let (commitment, poly) = Assayer::commit_witness(&witness, &cert);
    let point = derive_point::<Goldilocks>(&commitment, poly.num_vars);
    let value = poly.evaluate(&point);
    let mut t = Transcript::new(DOMAIN);
    let opening = Assayer::open_witness(&poly, &point, &mut t);

    println!("commitment: {}", commitment.0.to_hex());
    println!("tropical:   valid");

    let artifact = encode_witness(&json, &point, value, &opening)?;
    match out {
        Some(path) => std::fs::write(path, &artifact).map_err(|e| format!("{path}: {e}"))?,
        None => println!("{}", hex(&artifact)),
    }
    Ok(())
}

/// `assayer-verify <commitment> <witness.lens>`
pub fn verify(commitment_hex: &str, witness_path: &str) -> Result<(), String> {
    let commitment = parse_commitment(commitment_hex)?;
    let bytes = std::fs::read(witness_path).map_err(|e| format!("{witness_path}: {e}"))?;
    let (json, point, value, opening) = decode_witness(&bytes)?;
    let (witness, cert) = parse_problem(&json)?;

    let tropical_ok = Assayer::verify_tropical(&witness, &cert);
    let mut t = Transcript::new(DOMAIN);
    let commit_ok = Assayer::verify_witness(&commitment, &point, value, &opening, &mut t);
    let ok = tropical_ok && commit_ok;

    println!("tropical:   {}", yes(tropical_ok));
    println!("commitment: {}", yes(commit_ok));
    println!("{}", if ok { "valid" } else { "invalid" });
    exit(if ok { 0 } else { 1 });
}

fn yes(b: bool) -> &'static str {
    if b { "valid" } else { "invalid" }
}

// ── problem.json ─────────────────────────────────────────────────

fn parse_problem(json: &str) -> Result<(TropicalWitness, DualCertificate), String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("problem json: {e}"))?;

    let num_vertices = field_u64(&v, "num_vertices")? as usize;
    let source = field_u64(&v, "source")? as usize;
    let target = field_u64(&v, "target")? as usize;
    let cost = Tropical::from_u64(field_u64(&v, "cost")?);
    let dual_objective = Goldilocks::new(field_u64(&v, "dual_objective")?);

    let edges = v["edges"]
        .as_array()
        .ok_or("problem json: 'edges' must be an array")?
        .iter()
        .map(|e| {
            Ok(Edge {
                from: field_u64(e, "from")? as usize,
                to: field_u64(e, "to")? as usize,
                weight: Tropical::from_u64(field_u64(e, "weight")?),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let assignment = u64_array(&v, "assignment")?
        .into_iter()
        .map(|x| x as usize)
        .collect();
    let dual_vars = u64_array(&v, "dual_vars")?
        .into_iter()
        .map(Goldilocks::new)
        .collect();

    Ok((
        TropicalWitness {
            num_vertices,
            edges,
            assignment,
            cost,
            source,
            target,
        },
        DualCertificate {
            dual_vars,
            dual_objective,
        },
    ))
}

fn field_u64(v: &serde_json::Value, key: &str) -> Result<u64, String> {
    v[key]
        .as_u64()
        .ok_or_else(|| format!("problem json: '{key}' must be a non-negative integer"))
}

fn u64_array(v: &serde_json::Value, key: &str) -> Result<Vec<u64>, String> {
    v[key]
        .as_array()
        .ok_or_else(|| format!("problem json: '{key}' must be an array"))?
        .iter()
        .map(|x| {
            x.as_u64()
                .ok_or_else(|| format!("problem json: '{key}' entries must be integers"))
        })
        .collect()
}

// ── witness artifact (ASSY) ──────────────────────────────────────

fn encode_witness(
    json: &str,
    point: &[Goldilocks],
    value: Goldilocks,
    opening: &Opening,
) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    put_len_bytes(&mut out, json.as_bytes());
    // Reuse the Tensor opening codec via the generic artifact over Goldilocks.
    let inner = crate::artifact::encode::<Goldilocks>(
        crate::artifact::TAG_BRAKEDOWN,
        point.len(),
        false,
        &[(point.to_vec(), value)],
        opening,
    )?;
    put_len_bytes(&mut out, &inner);
    Ok(out)
}

fn decode_witness(bytes: &[u8]) -> Result<(String, Vec<Goldilocks>, Goldilocks, Opening), String> {
    if bytes.len() < 5 || &bytes[0..4] != MAGIC {
        return Err("not an assayer witness artifact".into());
    }
    if bytes[4] != VERSION {
        return Err("unsupported witness version".into());
    }
    let mut pos = 5;
    let json = String::from_utf8(take_len(bytes, &mut pos)?.to_vec())
        .map_err(|_| "witness json not utf-8".to_string())?;
    let inner = take_len(bytes, &mut pos)?;
    let art = crate::artifact::decode::<Goldilocks>(inner)?;
    let (point, value) = art
        .points
        .into_iter()
        .next()
        .ok_or("witness artifact has no opening point")?;
    Ok((json, point, value, art.opening))
}

fn put_len_bytes(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}

fn take_len<'a>(bytes: &'a [u8], pos: &mut usize) -> Result<&'a [u8], String> {
    let end_len = pos.checked_add(4).ok_or("overflow")?;
    if end_len > bytes.len() {
        return Err("witness artifact truncated".into());
    }
    let n = u32::from_le_bytes(bytes[*pos..end_len].try_into().unwrap()) as usize;
    let start = end_len;
    let end = start.checked_add(n).ok_or("overflow")?;
    if end > bytes.len() {
        return Err("witness artifact truncated".into());
    }
    *pos = end;
    Ok(&bytes[start..end])
}

fn parse_commitment(s: &str) -> Result<Commitment, String> {
    let arr: [u8; 32] = hex_to_bytes(s)?
        .try_into()
        .map_err(|_| "commitment must be 32 bytes (64 hex chars)".to_string())?;
    Ok(Commitment(cyber_hemera::Hash::from_bytes(arr)))
}
