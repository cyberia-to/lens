//! Generic command implementations, one code path over every algebra.
//!
//! Each function is monomorphized per `(F, L)` by the dispatch in `main`.
//! A single opening verifies via `L::verify`; a batch via `L::batch_verify`.
//! The artifact records which mode it is, since the two paths differ per
//! construction (Binius's `batch_verify` re-derives a random point).

use std::process::exit;
use std::time::{Duration, Instant};

use lens::{Commitment, Lens, MultilinearPoly, Transcript};

use crate::algo::Algo;
use crate::artifact;
use crate::field::CliField;

pub fn commit<F: CliField, L: Lens<F>>(algo: Algo, file: &str) -> Result<(), String> {
    let poly = poly_from_file::<F>(file)?;
    let t0 = Instant::now();
    let commitment = L::commit(&poly);
    timing(algo, "commit", poly.num_vars, t0.elapsed());
    println!("{}", commitment.0.to_hex());
    Ok(())
}

pub fn open<F: CliField, L: Lens<F>>(
    algo: Algo,
    file: &str,
    point_strs: &[String],
    out: Option<&str>,
) -> Result<(), String> {
    let poly = poly_from_file::<F>(file)?;
    let point = parse_point::<F>(point_strs, poly.num_vars)?;
    let value = poly.evaluate(&point);

    let t0 = Instant::now();
    let mut t = Transcript::new(algo.domain());
    let opening = L::open(&poly, &point, &mut t);
    timing(algo, "open", poly.num_vars, t0.elapsed());

    let points = vec![(point, value)];
    let bytes = artifact::encode::<F>(algo.tag(), poly.num_vars, false, &points, &opening)?;
    println!("value: {}", hex(&value.to_le()));
    write_artifact(&bytes, out)
}

pub fn open_batch<F: CliField, L: Lens<F>>(
    algo: Algo,
    file: &str,
    points_file: &str,
    out: Option<&str>,
) -> Result<(), String> {
    let poly = poly_from_file::<F>(file)?;
    let raw = std::fs::read_to_string(points_file).map_err(|e| format!("{points_file}: {e}"))?;
    let mut points = Vec::new();
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let coords: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        let point = parse_point::<F>(&coords, poly.num_vars)?;
        let value = poly.evaluate(&point);
        points.push((point, value));
    }
    if points.is_empty() {
        return Err("no points in points-file".into());
    }

    let t0 = Instant::now();
    let mut t = Transcript::new(algo.domain());
    let opening = L::batch_open(&poly, &points, &mut t);
    timing(algo, "open-batch", poly.num_vars, t0.elapsed());

    let bytes = artifact::encode::<F>(algo.tag(), poly.num_vars, true, &points, &opening)?;
    println!("points: {}", points.len());
    write_artifact(&bytes, out)
}

pub fn verify<F: CliField, L: Lens<F>>(
    algo: Algo,
    commitment_hex: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let commitment = parse_commitment(commitment_hex)?;
    let art = artifact::decode::<F>(bytes)?;
    if art.points.is_empty() {
        return Err("artifact has no opening points".into());
    }

    let t0 = Instant::now();
    let mut t = Transcript::new(algo.domain());
    // Single and batch verify are distinct paths per construction — Binius's
    // batch_verify re-derives a random point, so a single open must use verify.
    let ok = if art.batch {
        L::batch_verify(&commitment, &art.points, &art.opening, &mut t)
    } else {
        let (point, value) = &art.points[0];
        L::verify(&commitment, point, *value, &art.opening, &mut t)
    };
    timing(algo, "verify", art.num_vars, t0.elapsed());

    println!("{}", if ok { "valid" } else { "invalid" });
    exit(if ok { 0 } else { 1 });
}

pub fn eval<F: CliField>(file: &str, point_strs: &[String]) -> Result<(), String> {
    let poly = poly_from_file::<F>(file)?;
    let point = parse_point::<F>(point_strs, poly.num_vars)?;
    println!("{}", hex(&poly.evaluate(&point).to_le()));
    Ok(())
}

pub fn check<F: CliField, L: Lens<F>>(
    algo: Algo,
    file: Option<&str>,
    vars: Option<usize>,
) -> Result<(), String> {
    let poly = match (file, vars) {
        (Some(f), _) => poly_from_file::<F>(f)?,
        (None, Some(n)) => synthetic_poly::<F>(n),
        (None, None) => {
            return Err("usage: lens check <file> [--algo A]  |  lens check --vars N".into());
        }
    };

    let t0 = Instant::now();
    let commitment = L::commit(&poly);
    println!(
        "commit  {}   {}",
        commitment.0.to_hex(),
        fmt_dur(t0.elapsed())
    );

    let point = derive_point::<F>(&commitment, poly.num_vars);
    let value = poly.evaluate(&point);

    let t1 = Instant::now();
    let mut ot = Transcript::new(algo.domain());
    let opening = L::open(&poly, &point, &mut ot);
    println!(
        "open    value {}   {}",
        hex(&value.to_le()),
        fmt_dur(t1.elapsed())
    );

    let t2 = Instant::now();
    let mut vt = Transcript::new(algo.domain());
    let ok = L::verify(&commitment, &point, value, &opening, &mut vt);
    println!(
        "verify  {}   {}",
        if ok { "valid" } else { "invalid" },
        fmt_dur(t2.elapsed())
    );

    println!("{}", if ok { "PASS" } else { "FAIL" });
    exit(if ok { 0 } else { 1 });
}

// ── shared helpers ───────────────────────────────────────────────

pub fn poly_from_file<F: CliField>(path: &str) -> Result<MultilinearPoly<F>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(poly_from_bytes::<F>(&bytes))
}

/// Encode bytes as field elements and pad the table to the next power of two.
pub fn poly_from_bytes<F: CliField>(bytes: &[u8]) -> MultilinearPoly<F> {
    let mut evals: Vec<F> = bytes.chunks(F::INPUT_CHUNK).map(F::encode_chunk).collect();
    if evals.is_empty() {
        evals.push(F::ZERO);
    }
    let target = evals.len().next_power_of_two();
    evals.resize(target, F::ZERO);
    MultilinearPoly::new(evals)
}

fn synthetic_poly<F: CliField>(n: usize) -> MultilinearPoly<F> {
    let len = 1usize << n;
    let evals = (0..len)
        .map(|i| {
            let mut seed = [0u8; 8];
            seed.copy_from_slice(&(i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes());
            F::encode_chunk(&seed[..F::INPUT_CHUNK.min(8)])
        })
        .collect();
    MultilinearPoly::new(evals)
}

/// A deterministic point derived from the commitment, so `check` exercises
/// open/verify at a non-trivial location.
pub fn derive_point<F: CliField>(commitment: &Commitment, num_vars: usize) -> Vec<F> {
    let mut t = Transcript::new(b"lens/check/point");
    t.absorb(commitment.as_bytes());
    let mut point = Vec::with_capacity(num_vars);
    for _ in 0..num_vars {
        point.push(t.squeeze_field());
    }
    point
}

fn parse_point<F: CliField>(strs: &[String], num_vars: usize) -> Result<Vec<F>, String> {
    if strs.len() != num_vars {
        return Err(format!(
            "point needs {num_vars} coordinates for this file, got {}",
            strs.len()
        ));
    }
    strs.iter().map(|s| F::parse(s)).collect()
}

fn parse_commitment(s: &str) -> Result<Commitment, String> {
    let bytes = hex_to_bytes(s)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "commitment must be 32 bytes (64 hex chars)".to_string())?;
    Ok(Commitment(cyber_hemera::Hash::from_bytes(arr)))
}

fn write_artifact(bytes: &[u8], out: Option<&str>) -> Result<(), String> {
    match out {
        Some(path) => std::fs::write(path, bytes).map_err(|e| format!("{path}: {e}")),
        None => {
            println!("{}", hex(bytes));
            Ok(())
        }
    }
}

// ── hex + formatting ─────────────────────────────────────────────

pub fn hex_to_bytes(s: &str) -> Result<Vec<u8>, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if !s.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "invalid hex".to_string()))
        .collect()
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn timing(algo: Algo, stage: &str, num_vars: usize, dur: Duration) {
    eprintln!(
        "\x1b[90m[{} {} 2^{}  {}]\x1b[0m",
        algo.name(),
        stage,
        num_vars,
        fmt_dur(dur)
    );
}

fn fmt_dur(d: Duration) -> String {
    let us = d.as_secs_f64() * 1e6;
    if us < 1000.0 {
        format!("{us:.0}us")
    } else {
        format!("{:.1}ms", us / 1000.0)
    }
}
