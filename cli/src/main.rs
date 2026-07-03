// ---
// tags: lens, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! lens CLI — commit to a file as a polynomial, open evaluations, verify
//! openings without the file. Wraps the commitment layer behind one binary.
//!
//! Core commands (specs/cli.md §1–5): commit, open, verify, eval, check.
//! Plus params and vectors. v0.1 wires the two Goldilocks constructions —
//! Brakedown (default) and Ikat. Wide-field constructions (Binius over
//! F₂¹²⁸, Porphyry over F_q) and Assayer are reported honestly as not yet
//! wired, rather than half-working.

mod artifact;

use std::process::exit;
use std::time::{Duration, Instant};

use lens::brakedown::Brakedown;
use lens::ikat::Ikat;
use lens::{Commitment, Lens, MultilinearPoly, Opening, Transcript};
use nebu::Goldilocks;
use nebu::encoding::encode_7;

// ── construction selection ───────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Algo {
    Brakedown,
    Ikat,
}

impl Algo {
    fn name(self) -> &'static str {
        match self {
            Algo::Brakedown => "brakedown",
            Algo::Ikat => "ikat",
        }
    }

    fn tag(self) -> u8 {
        match self {
            Algo::Brakedown => artifact::TAG_BRAKEDOWN,
            Algo::Ikat => artifact::TAG_IKAT,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, String> {
        match tag {
            artifact::TAG_BRAKEDOWN => Ok(Algo::Brakedown),
            artifact::TAG_IKAT => Ok(Algo::Ikat),
            other => Err(format!("unknown construction tag {other} in artifact")),
        }
    }

    /// Fiat-Shamir domain — identical between open and verify (specs/cli.md §4).
    fn domain(self) -> &'static [u8] {
        match self {
            Algo::Brakedown => b"lens/brakedown/v0.1.0",
            Algo::Ikat => b"lens/ikat/v0.1.0",
        }
    }
}

/// Parse `--algo`. Goldilocks constructions are wired; the rest report
/// honestly instead of pretending.
fn parse_algo(s: &str) -> Result<Algo, String> {
    match s {
        "brakedown" => Ok(Algo::Brakedown),
        "ikat" => Ok(Algo::Ikat),
        "binius" | "porphyry" => Err(format!(
            "construction '{s}' is not yet wired into the cli \
             (v0.1 core is Goldilocks: brakedown, ikat)"
        )),
        other => Err(format!("unknown construction '{other}'")),
    }
}

// generic dispatch over the two Goldilocks lenses ------------------

fn commit_with(algo: Algo, poly: &MultilinearPoly<Goldilocks>) -> Commitment {
    match algo {
        Algo::Brakedown => Brakedown::commit(poly),
        Algo::Ikat => Ikat::commit(poly),
    }
}

fn open_with(
    algo: Algo,
    poly: &MultilinearPoly<Goldilocks>,
    point: &[Goldilocks],
    t: &mut Transcript,
) -> Opening {
    match algo {
        Algo::Brakedown => Brakedown::open(poly, point, t),
        Algo::Ikat => Ikat::open(poly, point, t),
    }
}

fn verify_with(
    algo: Algo,
    c: &Commitment,
    point: &[Goldilocks],
    value: Goldilocks,
    proof: &Opening,
    t: &mut Transcript,
) -> bool {
    match algo {
        Algo::Brakedown => Brakedown::verify(c, point, value, proof, t),
        Algo::Ikat => Ikat::verify(c, point, value, proof, t),
    }
}

// ── main ─────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        exit(1);
    }
    let cmd = args[0].as_str();
    let rest = &args[1..];

    let result = match cmd {
        "commit" => cmd_commit(rest),
        "open" => cmd_open(rest),
        "verify" => cmd_verify(rest),
        "eval" => cmd_eval(rest),
        "check" => cmd_check(rest),
        "params" => cmd_params(rest),
        "vectors" => cmd_vectors(),
        "assayer" | "assayer-verify" | "open-batch" | "verify-batch" => Err(format!(
            "'{cmd}' is specified but not yet implemented (specs/cli.md §8, §10)"
        )),
        "help" | "-h" | "--help" => {
            print_usage();
            return;
        }
        other => Err(format!("unknown command '{other}' — try `lens help`")),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        exit(1);
    }
}

// ── commands ─────────────────────────────────────────────────────

fn cmd_commit(args: &[String]) -> Result<(), String> {
    let Flags { algo, pos, .. } = parse_flags(args)?;
    let file = pos.first().ok_or("usage: lens commit <file> [--algo A]")?;
    let poly = poly_from_file(file)?;

    let t0 = Instant::now();
    let commitment = commit_with(algo, &poly);
    timing(algo, "commit", poly.num_vars, t0.elapsed());

    println!("{}", commitment.0.to_hex());
    Ok(())
}

fn cmd_open(args: &[String]) -> Result<(), String> {
    let Flags { algo, out, pos, .. } = parse_flags(args)?;
    let file = pos
        .first()
        .ok_or("usage: lens open <file> <point...> [--algo A] [-o opening.lens]")?;
    let poly = poly_from_file(file)?;
    let point = parse_point(&pos[1..], poly.num_vars)?;

    let value = poly.evaluate(&point);
    let t0 = Instant::now();
    let mut t = Transcript::new(algo.domain());
    let opening = open_with(algo, &poly, &point, &mut t);
    timing(algo, "open", poly.num_vars, t0.elapsed());

    let bytes = artifact::encode(algo.tag(), &point, value, &opening)?;
    println!("value: {}", hex8(value));
    match out {
        Some(path) => std::fs::write(&path, &bytes).map_err(|e| format!("{path}: {e}"))?,
        None => println!("{}", hex(&bytes)),
    }
    Ok(())
}

fn cmd_verify(args: &[String]) -> Result<(), String> {
    if args.len() != 2 {
        return Err("usage: lens verify <commitment> <opening.lens>".into());
    }
    let commitment = parse_commitment(&args[0])?;
    let bytes = std::fs::read(&args[1]).map_err(|e| format!("{}: {e}", args[1]))?;
    let art = artifact::decode(&bytes)?;
    let algo = Algo::from_tag(art.algo_tag)?;

    let t0 = Instant::now();
    let mut t = Transcript::new(algo.domain());
    let ok = verify_with(
        algo,
        &commitment,
        &art.point,
        art.value,
        &art.opening,
        &mut t,
    );
    timing(algo, "verify", art.num_vars, t0.elapsed());

    println!("{}", if ok { "valid" } else { "invalid" });
    exit(if ok { 0 } else { 1 });
}

fn cmd_eval(args: &[String]) -> Result<(), String> {
    // eval is construction-independent; --algo is accepted for symmetry and ignored.
    let Flags { pos, .. } = parse_flags(args)?;
    let file = pos
        .first()
        .ok_or("usage: lens eval <file> <point...> [--algo A]")?;
    let poly = poly_from_file(file)?;
    let point = parse_point(&pos[1..], poly.num_vars)?;
    println!("{}", hex8(poly.evaluate(&point)));
    Ok(())
}

fn cmd_check(args: &[String]) -> Result<(), String> {
    let Flags {
        algo, vars, pos, ..
    } = parse_flags(args)?;
    let poly = match (pos.first(), vars) {
        (Some(file), _) => poly_from_file(file)?,
        (None, Some(n)) => synthetic_poly(n),
        (None, None) => {
            return Err("usage: lens check <file> [--algo A]  |  lens check --vars N".into());
        }
    };

    let t0 = Instant::now();
    let commitment = commit_with(algo, &poly);
    let dt_commit = t0.elapsed();
    println!("commit  {}   {}", commitment.0.to_hex(), fmt_dur(dt_commit));

    let point = derive_point(&commitment, poly.num_vars);
    let value = poly.evaluate(&point);

    let t1 = Instant::now();
    let mut ot = Transcript::new(algo.domain());
    let opening = open_with(algo, &poly, &point, &mut ot);
    println!("open    value {}   {}", hex8(value), fmt_dur(t1.elapsed()));

    let t2 = Instant::now();
    let mut vt = Transcript::new(algo.domain());
    let ok = verify_with(algo, &commitment, &point, value, &opening, &mut vt);
    println!(
        "verify  {}   {}",
        if ok { "valid" } else { "invalid" },
        fmt_dur(t2.elapsed())
    );

    println!("{}", if ok { "PASS" } else { "FAIL" });
    exit(if ok { 0 } else { 1 });
}

fn cmd_params(args: &[String]) -> Result<(), String> {
    let Flags { algo, .. } = parse_flags(args)?;
    match algo {
        Algo::Brakedown => {
            println!("construction : Brakedown");
            println!("field        : Goldilocks (p = 2^64 - 2^32 + 1)");
            println!("encoding     : Margulis expander + tensor decomposition");
            println!("queries/round: 20");
            println!("commitment   : 32 bytes (hemera)");
            println!("security λ   : 128");
        }
        Algo::Ikat => {
            println!("construction : Ikat");
            println!("field        : Goldilocks NTT slots (R_q via jali)");
            println!("encoding     : NTT-batched Brakedown");
            println!("commitment   : 32 bytes (hemera)");
            println!("security λ   : 128");
        }
    }
    Ok(())
}

fn cmd_vectors() -> Result<(), String> {
    // Deterministic anchors for cross-implementation verification.
    // Commitment of fixed inputs under each Goldilocks construction.
    let inputs: [(&str, &[u8]); 3] = [("empty", b""), ("hello", b"hello"), ("lens", b"lens")];
    println!("{{");
    for (ai, algo) in [Algo::Brakedown, Algo::Ikat].iter().enumerate() {
        println!("  \"{}\": {{", algo.name());
        for (ii, (name, data)) in inputs.iter().enumerate() {
            let poly = poly_from_bytes(data);
            let c = commit_with(*algo, &poly);
            let comma = if ii + 1 < inputs.len() { "," } else { "" };
            println!("    \"commit_{name}\": \"{}\"{comma}", c.0.to_hex());
        }
        let comma = if ai + 1 < 2 { "," } else { "" };
        println!("  }}{comma}");
    }
    println!("}}");
    Ok(())
}

// ── input → polynomial ───────────────────────────────────────────

fn poly_from_file(path: &str) -> Result<MultilinearPoly<Goldilocks>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(poly_from_bytes(&bytes))
}

/// Encode bytes as field elements (canonical 7-byte chunks) and pad the
/// evaluation table to the next power of two.
fn poly_from_bytes(bytes: &[u8]) -> MultilinearPoly<Goldilocks> {
    let mut evals: Vec<Goldilocks> = bytes.chunks(7).map(encode_7).collect();
    if evals.is_empty() {
        evals.push(Goldilocks::ZERO);
    }
    let target = evals.len().next_power_of_two();
    evals.resize(target, Goldilocks::ZERO);
    MultilinearPoly::new(evals)
}

/// Deterministic synthetic polynomial of 2^n evaluations, for `check --vars`.
fn synthetic_poly(n: usize) -> MultilinearPoly<Goldilocks> {
    let len = 1usize << n;
    let evals = (0..len as u64)
        .map(|i| {
            Goldilocks::new(i.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1)).canonicalize()
        })
        .collect();
    MultilinearPoly::new(evals)
}

/// A deterministic evaluation point derived from the commitment, so `check`
/// exercises open/verify at a non-trivial location.
fn derive_point(commitment: &Commitment, num_vars: usize) -> Vec<Goldilocks> {
    let mut t = Transcript::new(b"lens/check/point");
    t.absorb(commitment.as_bytes());
    let mut point = Vec::with_capacity(num_vars);
    for _ in 0..num_vars {
        point.push(t.squeeze_field());
    }
    point
}

// ── parsing ──────────────────────────────────────────────────────

/// Parsed command-line flags plus positional arguments.
struct Flags {
    algo: Algo,
    out: Option<String>,
    vars: Option<usize>,
    pos: Vec<String>,
}

/// Extract `--algo A`, `-o F`, `--vars N`, and positional args in one pass.
fn parse_flags(args: &[String]) -> Result<Flags, String> {
    let mut algo = Algo::Brakedown;
    let mut out = None;
    let mut vars = None;
    let mut pos = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--algo" => {
                algo = parse_algo(next(args, &mut i, "--algo")?)?;
            }
            "-o" => {
                out = Some(next(args, &mut i, "-o")?.to_string());
            }
            "--vars" => {
                let n = next(args, &mut i, "--vars")?
                    .parse::<usize>()
                    .map_err(|_| "--vars expects a number".to_string())?;
                if n > 24 {
                    return Err("--vars capped at 24 (2^24 evaluations)".into());
                }
                vars = Some(n);
            }
            other => pos.push(other.to_string()),
        }
        i += 1;
    }
    Ok(Flags {
        algo,
        out,
        vars,
        pos,
    })
}

fn next<'a>(args: &'a [String], i: &mut usize, flag: &str) -> Result<&'a str, String> {
    *i += 1;
    args.get(*i)
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} expects a value"))
}

/// Parse `ν` field elements; error if the count mismatches the polynomial.
fn parse_point(strs: &[String], num_vars: usize) -> Result<Vec<Goldilocks>, String> {
    if strs.len() != num_vars {
        return Err(format!(
            "point needs {num_vars} coordinates for this file, got {}",
            strs.len()
        ));
    }
    strs.iter().map(|s| parse_goldilocks(s)).collect()
}

/// Parse one Goldilocks element from decimal or `0x` hex; reject ≥ p.
fn parse_goldilocks(s: &str) -> Result<Goldilocks, String> {
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

fn parse_commitment(s: &str) -> Result<Commitment, String> {
    let bytes = hex_to_bytes(s)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "commitment must be 32 bytes (64 hex chars)".to_string())?;
    Ok(Commitment(cyber_hemera::Hash::from_bytes(arr)))
}

// ── hex + formatting ─────────────────────────────────────────────

fn hex_to_bytes(s: &str) -> Result<Vec<u8>, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if !s.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "invalid hex".to_string()))
        .collect()
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn hex8(g: Goldilocks) -> String {
    hex(&g.as_u64().to_le_bytes())
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

fn print_usage() {
    eprintln!(
        "\x1b[36m  lens\x1b[0m — polynomial commitment tool

  lens commit <file>                    commit a file, print 32-byte root
  lens open   <file> <point...> [-o f]  open at a point, write proof
  lens verify <commitment> <proof>      verify an opening (no file)
  lens eval   <file> <point...>         evaluate, no proof (ground truth)
  lens check  <file>                    commit + open + verify, print PASS/FAIL
  lens params                           construction parameters
  lens vectors                          print test vectors (JSON)

\x1b[90m  flags:  --algo brakedown|ikat   (default: brakedown)
          --vars N                 synthetic 2^N polynomial (check)
          -o <file>                write proof to file

  not yet wired: binius, porphyry, assayer, batch\x1b[0m"
    );
}

#[cfg(test)]
mod tests;
