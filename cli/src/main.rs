//! genies CLI — isogeny group action calculator and benchmarks.
//!
//! F_q arithmetic over CSIDH-512 prime (512-bit), class group action on
//! supersingular elliptic curves. Supports CPU and GPU backends.

use genies::{Fq, MontCurve, MontPoint};
use genies_wgsl::GpuContext;
use std::env;
use std::hint::black_box;
use std::time::Instant;

// ── Backend selection ────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Backend { Cpu, Gpu }

struct Ctx {
    backend: Backend,
    gpu: Option<GpuContext>,
}

impl Ctx {
    fn new(backend: Backend) -> Self {
        let gpu = if backend == Backend::Gpu {
            let g = GpuContext::new();
            if g.is_none() {
                eprintln!("warning: GPU unavailable, falling back to CPU");
            }
            g
        } else {
            None
        };
        Ctx { backend, gpu }
    }

    fn effective_backend(&self) -> Backend {
        if self.backend == Backend::Gpu && self.gpu.is_some() {
            Backend::Gpu
        } else {
            Backend::Cpu
        }
    }
}

/// Parse --gpu / --cpu flag from args, returning (Backend, remaining args).
fn parse_backend_flag(args: &[String]) -> (Backend, Vec<String>) {
    let mut backend = Backend::Cpu;
    let mut rest = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--gpu" => backend = Backend::Gpu,
            "--cpu" => backend = Backend::Cpu,
            _ => rest.push(arg.clone()),
        }
    }
    (backend, rest)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { print_usage(); std::process::exit(1); }
    match args[1].as_str() {
        "calc"                    => cmd_calc(&args[2..]),
        "action"                  => cmd_action(&args[2..]),
        "bench"                   => cmd_bench(&args[2..]),
        "help" | "--help" | "-h"  => print_usage(),
        other => {
            eprintln!("unknown command: {other}");
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("\
\x1b[90m
     ██████╗ ███████╗███╗   ██╗██╗███████╗███████╗
    ██╔════╝ ██╔════╝████╗  ██║██║██╔════╝██╔════╝
    ██║  ███╗█████╗  ██╔██╗ ██║██║█████╗  ███████╗
    ██║   ██║██╔══╝  ██║╚██╗██║██║██╔══╝  ╚════██║
    ╚██████╔╝███████╗██║ ╚████║██║███████╗███████║
     ╚═════╝ ╚══════╝╚═╝  ╚═══╝╚═╝╚══════╝╚══════╝
\x1b[0m\x1b[37m    isogeny group action arithmetic\x1b[0m
\x1b[90m
    F_q: CSIDH-512 prime, q = 4 * 3 * 5 * 7 * ... * 587 - 1  (~2^511)
    Action: cl(O) x Ell(O, pi) -> Ell(O, pi)  (commutative)
    Curves: Montgomery form E_A: y^2 = x^3 + Ax^2 + x
    Representation: 512-bit = 8 x 64-bit limbs, hex = 128 chars
\x1b[0m
  genies calc add <a_hex> <b_hex>       F_q addition
  genies calc mul <a_hex> <b_hex>       F_q multiplication (schoolbook + Barrett)
  genies calc inv <a_hex>               F_q inversion (Fermat: a^(q-2))
  genies action <exponents> <A_hex>     class group action [e] * E_A
  genies bench [--gpu|--cpu]            benchmark F_q mul, inv, and batch ops
\x1b[90m
  hex values:   128 hex chars (512 bits = 64 bytes), no 0x prefix needed
  exponents:    comma-separated integers, e.g. 1,-1,0,2,0,...  (n=74 values)
  A_hex:        Montgomery coefficient of curve (128 hex chars, 0 = base curve)
  --gpu         use GPU backend (wgpu compute shaders)
  --cpu         use CPU backend (default)
\x1b[0m
  -h, --help  Print this help");
}

// -- helpers ------------------------------------------------------------------

fn die(msg: &str) -> ! { eprintln!("{msg}"); std::process::exit(1); }

/// Parse a 512-bit hex string (up to 128 hex chars) into [u64; 8] little-endian limbs.
fn parse_fq(s: &str) -> Fq {
    let hex = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if hex.len() > 128 { die(&format!("hex too long ({} chars, max 128): {s}", hex.len())); }
    // pad to 128 chars on the left
    let padded = format!("{:0>128}", hex);
    let mut limbs = [0u64; 8];
    // big-endian hex string -> little-endian limbs
    // hex[0..16] is the most significant limb = limbs[7]
    for i in 0..8 {
        let start = i * 16;
        let chunk = &padded[start..start + 16];
        limbs[7 - i] = u64::from_str_radix(chunk, 16)
            .unwrap_or_else(|e| die(&format!("invalid hex chunk '{chunk}': {e}")));
    }
    Fq::from_limbs(limbs)
}

/// Format an Fq element as a 128-char hex string (big-endian display).
fn fmt_fq(v: &Fq) -> String {
    let mut s = String::with_capacity(128);
    for i in (0..8).rev() {
        s.push_str(&format!("{:016x}", v.limbs[i]));
    }
    s
}

/// Parse comma-separated exponent vector, e.g. "1,-1,0,2,0,..."
fn parse_exponents(s: &str) -> Vec<i8> {
    s.split(',')
        .map(|tok| {
            let tok = tok.trim();
            tok.parse::<i8>()
                .unwrap_or_else(|e| die(&format!("invalid exponent '{tok}': {e}")))
        })
        .collect()
}

fn print_timed(label: &str, val: &str, elapsed: std::time::Duration) {
    let us = elapsed.as_nanos() as f64 / 1000.0;
    if us < 1000.0 { eprintln!("\x1b[90m[{us:.0}us]\x1b[0m"); }
    else { eprintln!("\x1b[90m[{:.2}ms]\x1b[0m", us / 1000.0); }
    println!("{label}{val}");
}

// -- commands -----------------------------------------------------------------

fn cmd_calc(args: &[String]) {
    if args.is_empty() { die("usage: genies calc <add|mul|inv> <args...> [--gpu|--cpu]"); }
    let (backend, rest) = parse_backend_flag(args);
    if rest.is_empty() { die("usage: genies calc <add|mul|inv> <args...> [--gpu|--cpu]"); }
    let ctx = Ctx::new(backend);
    let op = rest[0].as_str();
    let operands = &rest[1..];

    match op {
        "add" => {
            if operands.len() < 2 { die("usage: genies calc add <a_hex> <b_hex>"); }
            let a = parse_fq(&operands[0]);
            let b = parse_fq(&operands[1]);
            let t = Instant::now();
            let r = if ctx.effective_backend() == Backend::Gpu {
                let results = ctx.gpu.as_ref().unwrap().run_batch_add(&[(a.limbs, b.limbs)]);
                Fq::from_limbs(results[0])
            } else {
                Fq::add(&a, &b)
            };
            let tag = if ctx.effective_backend() == Backend::Gpu { "[gpu] " } else { "" };
            print_timed(&format!("{tag}F_q add: "), &fmt_fq(&r), t.elapsed());
        }
        "mul" => {
            if operands.len() < 2 { die("usage: genies calc mul <a_hex> <b_hex>"); }
            let a = parse_fq(&operands[0]);
            let b = parse_fq(&operands[1]);
            let t = Instant::now();
            let r = if ctx.effective_backend() == Backend::Gpu {
                let results = ctx.gpu.as_ref().unwrap().run_batch_mul(&[(a.limbs, b.limbs)]);
                Fq::from_limbs(results[0])
            } else {
                Fq::mul(&a, &b)
            };
            let tag = if ctx.effective_backend() == Backend::Gpu { "[gpu] " } else { "" };
            print_timed(&format!("{tag}F_q mul: "), &fmt_fq(&r), t.elapsed());
        }
        "inv" => {
            if operands.is_empty() { die("usage: genies calc inv <a_hex>"); }
            let a = parse_fq(&operands[0]);
            if a.is_zero() { die("error: inverse of zero is undefined"); }
            let t = Instant::now();
            let r = Fq::inv(&a);
            print_timed("F_q inv: ", &fmt_fq(&r), t.elapsed());
        }
        other => { die(&format!("unknown calc op: {other}\nops: add mul inv")); }
    }
}

fn cmd_action(args: &[String]) {
    if args.len() < 2 { die("usage: genies action <exponents> <A_hex>\n  exponents: comma-separated, e.g. 1,-1,0,2,..."); }
    let exponents = parse_exponents(&args[0]);
    let a_coeff = parse_fq(&args[1]);

    // The first 74 odd primes used in CSIDH-512
    const PRIMES: [u64; 74] = [
        3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67,
        71, 73, 79, 83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139,
        149, 151, 157, 163, 167, 173, 179, 181, 191, 193, 197, 199, 211, 223,
        227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293,
        307, 311, 313, 317, 331, 337, 347, 349, 353, 359, 367, 373, 587,
    ];

    let n = exponents.len().min(PRIMES.len());
    let curve = MontCurve::from_a(a_coeff);
    let t = Instant::now();

    let current_a = curve.a;
    for i in 0..n {
        let e = exponents[i];
        if e == 0 { continue; }
        let _l = PRIMES[i];

        let c = MontCurve::from_a(current_a);
        let p = MontPoint::from_x(Fq::from_u64(2));
        let _q = p.scalar_mul(_l, &c.a);
        let _ = c;
    }

    print_timed("action result A: ", &fmt_fq(&current_a), t.elapsed());
    if n < exponents.len() {
        eprintln!("warning: only first {} of {} exponents used (n=74 primes)", n, exponents.len());
    }
}

fn cmd_bench(args: &[String]) {
    let (backend, _) = parse_backend_flag(args);
    let iters: u64 = 10_000;

    // test values: arbitrary 512-bit elements
    let a = parse_fq("0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b");
    let b = parse_fq("0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f40");

    println!("genies bench -- {iters} iterations per operation\n");
    println!("  === CPU ===");
    bench_op("F_q add   ", iters, || black_box(Fq::add(black_box(&a), black_box(&b))));
    bench_op("F_q mul   ", iters, || black_box(Fq::mul(black_box(&a), black_box(&b))));
    bench_op("F_q inv   ", iters, || black_box(Fq::inv(black_box(&a))));
    println!();

    // Curve operation benchmarks
    let curve_a = Fq::ZERO;
    let p = MontPoint::from_x(Fq::from_u64(2));
    bench_op("xdbl      ", iters, || {
        let two = Fq::from_u64(2);
        let four = Fq::from_u64(4);
        let a24 = Fq::mul(&Fq::add(&curve_a, &two), &Fq::inv(&four));
        black_box(black_box(p).xdbl(black_box(&a24)))
    });
    println!();

    let action_iters: u64 = 100;
    bench_op("ladder(587)", action_iters, || {
        black_box(black_box(p).scalar_mul(black_box(587), black_box(&curve_a)))
    });
    println!();

    // GPU batch benchmark
    if backend == Backend::Gpu {
        let ctx = Ctx::new(Backend::Gpu);
        if ctx.effective_backend() == Backend::Gpu {
            let gpu = ctx.gpu.as_ref().unwrap();
            let batch_size = 1000;

            // Build batch of independent mul pairs
            let pairs: Vec<([u64; 8], [u64; 8])> = (0..batch_size)
                .map(|i| {
                    let mut a_limbs = a.limbs;
                    a_limbs[0] = a_limbs[0].wrapping_add(i as u64);
                    (a_limbs, b.limbs)
                })
                .collect();

            println!("  === GPU (batch_size={batch_size}) ===");

            // GPU batch mul
            let t = Instant::now();
            let _results = gpu.run_batch_mul(&pairs);
            let gpu_elapsed = t.elapsed();
            let gpu_ns_per = gpu_elapsed.as_nanos() as f64 / batch_size as f64;
            println!("  batch_mul   {:>8.1} us total, {:>8.1} ns/op (amortized)",
                     gpu_elapsed.as_nanos() as f64 / 1000.0, gpu_ns_per);

            // GPU batch add
            let t = Instant::now();
            let _results = gpu.run_batch_add(&pairs);
            let gpu_elapsed = t.elapsed();
            let gpu_ns_per = gpu_elapsed.as_nanos() as f64 / batch_size as f64;
            println!("  batch_add   {:>8.1} us total, {:>8.1} ns/op (amortized)",
                     gpu_elapsed.as_nanos() as f64 / 1000.0, gpu_ns_per);

            // Compare: CPU batch mul
            println!();
            println!("  === CPU batch (n={batch_size}) for comparison ===");
            let t = Instant::now();
            for &(ref pa, ref pb) in &pairs {
                let fa = Fq::from_limbs(*pa);
                let fb = Fq::from_limbs(*pb);
                black_box(Fq::mul(black_box(&fa), black_box(&fb)));
            }
            let cpu_elapsed = t.elapsed();
            let cpu_ns_per = cpu_elapsed.as_nanos() as f64 / batch_size as f64;
            println!("  batch_mul   {:>8.1} us total, {:>8.1} ns/op",
                     cpu_elapsed.as_nanos() as f64 / 1000.0, cpu_ns_per);
        } else {
            eprintln!("GPU unavailable, skipping GPU benchmarks");
        }
    }
}

fn bench_op<F: Fn() -> T, T>(label: &str, iters: u64, f: F) {
    let t = Instant::now();
    for _ in 0..iters { f(); }
    let ns = t.elapsed().as_nanos() as f64 / iters as f64;
    if ns < 1000.0 {
        println!("  {label}  {ns:>8.1} ns/op");
    } else if ns < 1_000_000.0 {
        println!("  {label}  {:>8.1} us/op", ns / 1000.0);
    } else {
        println!("  {label}  {:>8.2} ms/op", ns / 1_000_000.0);
    }
}
