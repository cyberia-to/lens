// ---
// tags: lens, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! lens CLI — commit to a file as a polynomial, open evaluations, verify
//! openings without the file. Wraps every construction of the commitment
//! layer behind one binary. See specs/cli.md.
//!
//! Constructions: Brakedown (default) and Ikat over Goldilocks, Binius over
//! F₂¹²⁸, Porphyry over F_q — all via the `Lens` trait and a field-generic
//! command path. Assayer (tropical witness-verify) has its own two commands.

mod algo;
mod artifact;
mod assayer;
mod commands;
mod field;

#[cfg(test)]
mod tests;

use std::process::exit;

use algo::{Algo, parse_algo};
use genies::Fq;
use kuro::F2_128;
use lens::binius::Binius;
use lens::brakedown::Brakedown;
use lens::ikat::Ikat;
use lens::porphyry::Porphyry;
use nebu::Goldilocks;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        exit(1);
    }
    let cmd = args[0].as_str();
    let rest = &args[1..];

    let result = match cmd {
        "commit" => run_commit(rest),
        "open" => run_open(rest),
        "verify" | "verify-batch" => run_verify(rest),
        "open-batch" => run_open_batch(rest),
        "eval" => run_eval(rest),
        "check" => run_check(rest),
        "params" => run_params(rest),
        "vectors" => run_vectors(),
        "assayer" => run_assayer(rest),
        "assayer-verify" => run_assayer_verify(rest),
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

// ── dispatch: pick (field, construction) from --algo, monomorphize ──

macro_rules! dispatch {
    ($algo:expr, $f:ident $(, $arg:expr)*) => {
        match $algo {
            Algo::Brakedown => commands::$f::<Goldilocks, Brakedown>($algo $(, $arg)*),
            Algo::Ikat => commands::$f::<Goldilocks, Ikat>($algo $(, $arg)*),
            Algo::Binius => commands::$f::<F2_128, Binius>($algo $(, $arg)*),
            Algo::Porphyry => commands::$f::<Fq, Porphyry>($algo $(, $arg)*),
        }
    };
}

fn run_commit(args: &[String]) -> Result<(), String> {
    let Flags { algo, pos, .. } = parse_flags(args)?;
    let file = pos.first().ok_or("usage: lens commit <file> [--algo A]")?;
    dispatch!(algo, commit, file)
}

fn run_open(args: &[String]) -> Result<(), String> {
    let Flags { algo, out, pos, .. } = parse_flags(args)?;
    let (file, point) = pos
        .split_first()
        .ok_or("usage: lens open <file> <point...> [--algo A] [-o opening.lens]")?;
    dispatch!(algo, open, file, point, out.as_deref())
}

fn run_open_batch(args: &[String]) -> Result<(), String> {
    let Flags { algo, out, pos, .. } = parse_flags(args)?;
    if pos.len() != 2 {
        return Err(
            "usage: lens open-batch <file> <points-file> [--algo A] [-o opening.lens]".into(),
        );
    }
    dispatch!(algo, open_batch, &pos[0], &pos[1], out.as_deref())
}

fn run_verify(args: &[String]) -> Result<(), String> {
    if args.len() != 2 {
        return Err("usage: lens verify <commitment> <opening.lens>".into());
    }
    let bytes = std::fs::read(&args[1]).map_err(|e| format!("{}: {e}", args[1]))?;
    let algo = Algo::from_tag(artifact::peek_algo(&bytes)?)?;
    let c = args[0].as_str();
    match algo {
        Algo::Brakedown => commands::verify::<Goldilocks, Brakedown>(algo, c, &bytes),
        Algo::Ikat => commands::verify::<Goldilocks, Ikat>(algo, c, &bytes),
        Algo::Binius => commands::verify::<F2_128, Binius>(algo, c, &bytes),
        Algo::Porphyry => commands::verify::<Fq, Porphyry>(algo, c, &bytes),
    }
}

fn run_eval(args: &[String]) -> Result<(), String> {
    let Flags { algo, pos, .. } = parse_flags(args)?;
    let (file, point) = pos
        .split_first()
        .ok_or("usage: lens eval <file> <point...> [--algo A]")?;
    match algo {
        Algo::Brakedown | Algo::Ikat => commands::eval::<Goldilocks>(file, point),
        Algo::Binius => commands::eval::<F2_128>(file, point),
        Algo::Porphyry => commands::eval::<Fq>(file, point),
    }
}

fn run_check(args: &[String]) -> Result<(), String> {
    let Flags {
        algo, vars, pos, ..
    } = parse_flags(args)?;
    let file = pos.first().map(String::as_str);
    dispatch!(algo, check, file, vars)
}

fn run_params(args: &[String]) -> Result<(), String> {
    let Flags { algo, .. } = parse_flags(args)?;
    let (field, encoding) = match algo {
        Algo::Brakedown => (
            "Goldilocks (p = 2^64 - 2^32 + 1)",
            "Margulis expander + tensor",
        ),
        Algo::Ikat => ("Goldilocks NTT slots (R_q)", "NTT-batched Brakedown"),
        Algo::Binius => ("F₂¹²⁸ binary tower", "binary folding + hemera Merkle"),
        Algo::Porphyry => ("F_q (CSIDH-512 prime)", "expander codes over deep field"),
    };
    println!("construction : {}", algo.name());
    println!("field        : {field}");
    println!("encoding     : {encoding}");
    println!("commitment   : 32 bytes (hemera)");
    println!("security λ   : 128");
    Ok(())
}

fn run_vectors() -> Result<(), String> {
    let inputs: [(&str, &[u8]); 3] = [("empty", b""), ("hello", b"hello"), ("lens", b"lens")];
    println!("{{");
    let algos = [Algo::Brakedown, Algo::Ikat, Algo::Binius, Algo::Porphyry];
    for (ai, algo) in algos.iter().enumerate() {
        println!("  \"{}\": {{", algo.name());
        for (ii, (name, data)) in inputs.iter().enumerate() {
            let comma = if ii + 1 < inputs.len() { "," } else { "" };
            let c = commit_bytes(*algo, data);
            println!("    \"commit_{name}\": \"{c}\"{comma}");
        }
        let comma = if ai + 1 < algos.len() { "," } else { "" };
        println!("  }}{comma}");
    }
    println!("}}");
    Ok(())
}

fn commit_bytes(algo: Algo, data: &[u8]) -> String {
    use lens::Lens;
    match algo {
        Algo::Brakedown => Brakedown::commit(&commands::poly_from_bytes::<Goldilocks>(data))
            .0
            .to_hex(),
        Algo::Ikat => Ikat::commit(&commands::poly_from_bytes::<Goldilocks>(data))
            .0
            .to_hex(),
        Algo::Binius => Binius::commit(&commands::poly_from_bytes::<F2_128>(data))
            .0
            .to_hex(),
        Algo::Porphyry => Porphyry::commit(&commands::poly_from_bytes::<Fq>(data))
            .0
            .to_hex(),
    }
}

fn run_assayer(args: &[String]) -> Result<(), String> {
    let Flags { out, pos, .. } = parse_flags(args)?;
    let problem = pos
        .first()
        .ok_or("usage: lens assayer <problem.json> [-o witness.lens]")?;
    assayer::solve(problem, out.as_deref())
}

fn run_assayer_verify(args: &[String]) -> Result<(), String> {
    if args.len() != 2 {
        return Err("usage: lens assayer-verify <commitment> <witness.lens>".into());
    }
    assayer::verify(&args[0], &args[1])
}

// ── flags ────────────────────────────────────────────────────────

struct Flags {
    algo: Algo,
    out: Option<String>,
    vars: Option<usize>,
    pos: Vec<String>,
}

fn parse_flags(args: &[String]) -> Result<Flags, String> {
    let mut algo = Algo::Brakedown;
    let mut out = None;
    let mut vars = None;
    let mut pos = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--algo" => algo = parse_algo(next(args, &mut i, "--algo")?)?,
            "-o" => out = Some(next(args, &mut i, "-o")?.to_string()),
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

fn print_usage() {
    eprintln!(
        "\x1b[36m  lens\x1b[0m — polynomial commitment tool

  lens commit <file>                    commit a file, print 32-byte root
  lens open   <file> <point...> [-o f]  open at a point, write proof
  lens verify <commitment> <proof>      verify an opening (no file)
  lens open-batch <file> <points-file>  amortize many openings into one proof
  lens eval   <file> <point...>         evaluate, no proof (ground truth)
  lens check  <file>                    commit + open + verify, print PASS/FAIL
  lens params                           construction parameters
  lens vectors                          print test vectors (JSON)
  lens assayer <problem.json> [-o f]    solve tropical problem, emit witness
  lens assayer-verify <commit> <proof>  verify tropical witness

\x1b[90m  flags:  --algo brakedown|ikat|binius|porphyry   (default: brakedown)
          --vars N                                 synthetic 2^N polynomial (check)
          -o <file>                                write proof to file\x1b[0m"
    );
}
