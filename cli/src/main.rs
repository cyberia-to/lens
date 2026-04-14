// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! jali CLI — polynomial ring arithmetic tool.
//!
//! Supports GPU (wgpu) and CPU backends. GPU is default when available.
//! Use --gpu or --cpu flags to force a backend.

use nebu::Goldilocks;
use jali::ring::RingElement;
use jali::ntt;
use jali::sample;
use jali::encoding;
use jali_wgsl::GpuContext;
use std::hint::black_box;
use std::time::Instant;

// ── backend selection ─────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Backend {
    Cpu,
    Gpu,
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Backend::Cpu => write!(f, "cpu"),
            Backend::Gpu => write!(f, "gpu"),
        }
    }
}

struct Ctx {
    gpu: Option<GpuContext>,
    forced: Option<Backend>,
}

impl Ctx {
    fn new(forced: Option<Backend>) -> Self {
        let gpu = if forced == Some(Backend::Cpu) {
            None
        } else {
            GpuContext::new()
        };
        if forced == Some(Backend::Gpu) && gpu.is_none() {
            eprintln!("error: --gpu requested but no GPU adapter available");
            std::process::exit(1);
        }
        Self { gpu, forced }
    }

    fn backend(&self) -> Backend {
        if self.forced == Some(Backend::Cpu) {
            return Backend::Cpu;
        }
        if self.gpu.is_some() {
            Backend::Gpu
        } else {
            Backend::Cpu
        }
    }

    fn gpu(&self) -> &GpuContext {
        self.gpu.as_ref().unwrap()
    }
}

/// Strip --gpu / --cpu from args, return (forced backend, remaining args).
fn parse_backend_flag(args: &[String]) -> (Option<Backend>, Vec<String>) {
    let mut forced = None;
    let mut rest = Vec::new();
    for a in args {
        match a.as_str() {
            "--gpu" => forced = Some(Backend::Gpu),
            "--cpu" => forced = Some(Backend::Cpu),
            _ => rest.push(a.clone()),
        }
    }
    (forced, rest)
}

// ── conversion helpers ────────────────────────────────────────────

fn to_lohi(g: Goldilocks) -> (u32, u32) {
    let v = g.as_u64();
    (v as u32, (v >> 32) as u32)
}

fn from_lohi(lo: u32, hi: u32) -> Goldilocks {
    Goldilocks::new((hi as u64) << 32 | lo as u64)
}

fn ring_to_gpu(elem: &RingElement) -> Vec<(u32, u32)> {
    elem.active_coeffs().iter().map(|g| to_lohi(*g)).collect()
}

fn gpu_to_ring(data: &[(u32, u32)], n: usize) -> RingElement {
    let coeffs: Vec<Goldilocks> = data.iter().map(|&(lo, hi)| from_lohi(lo, hi)).collect();
    RingElement::from_coeffs(&coeffs, n)
}

fn print_usage() {
    eprintln!("jali — polynomial ring arithmetic R_q = F_p[x]/(x^n+1)");
    eprintln!();
    eprintln!("usage:");
    eprintln!("  jali calc add <n> <a0,a1,...> <b0,b1,...>  — add two polynomials");
    eprintln!("  jali calc mul <n> <a0,a1,...> <b0,b1,...>  — multiply two polynomials");
    eprintln!("  jali ntt forward <n> <a0,a1,...>           — forward NTT");
    eprintln!("  jali ntt inverse <n> <a0,a1,...>           — inverse NTT");
    eprintln!("  jali sample uniform <seed> <n>             — sample uniform polynomial");
    eprintln!("  jali sample ternary <seed> <n>             — sample ternary polynomial");
    eprintln!("  jali sample cbd <seed> <n> <eta>           — sample CBD polynomial");
    eprintln!("  jali bench [n] [iters]                     — benchmark ring ops");
    eprintln!("  jali help                                  — show this message");
    eprintln!();
    eprintln!("flags:");
    eprintln!("  --gpu   force GPU backend");
    eprintln!("  --cpu   force CPU backend");
    eprintln!("  (default: GPU if available, else CPU)");
}

fn parse_poly(s: &str, n: usize) -> RingElement {
    let mut elem = RingElement::new(n);
    for (i, tok) in s.split(',').enumerate() {
        if i >= n { break; }
        let v: u64 = tok.trim().parse().unwrap_or(0);
        elem.coeffs[i] = Goldilocks::new(v);
    }
    elem
}

fn print_poly(elem: &RingElement) {
    let n = elem.n;
    let mut first = true;
    for i in 0..n {
        let v = elem.coeffs[i].as_u64();
        if !first { print!(","); }
        print!("{}", v);
        first = false;
    }
    println!();
}

/// Print result with backend and timing info.
fn print_timed(backend: Backend, elapsed: std::time::Duration) {
    let us = elapsed.as_nanos() as f64 / 1000.0;
    if us < 1000.0 {
        eprintln!("\x1b[90m[{backend} {us:.0}us]\x1b[0m");
    } else {
        eprintln!("\x1b[90m[{backend} {:.2}ms]\x1b[0m", us / 1000.0);
    }
}

// ── calc ─────────────────────────────────────────────────────────

fn cmd_calc(forced: Option<Backend>, args: &[String]) {
    if args.len() < 4 {
        eprintln!("error: calc requires: <add|mul> <n> <poly_a> <poly_b>");
        return;
    }
    let op = args[0].as_str();
    let n: usize = args[1].parse().unwrap_or(0);
    if !n.is_power_of_two() || n > 4096 {
        eprintln!("error: n must be a power of 2 <= 4096");
        return;
    }
    let a = parse_poly(&args[2], n);
    let b = parse_poly(&args[3], n);

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();
    let t = Instant::now();

    let result = match op {
        "add" => {
            if backend == Backend::Gpu {
                let ga = ring_to_gpu(&a);
                let gb = ring_to_gpu(&b);
                let gr = ctx.gpu().run_ring_add(&ga, &gb, n);
                gpu_to_ring(&gr, n)
            } else {
                a.add(&b)
            }
        }
        "mul" => {
            if backend == Backend::Gpu {
                let ga = ring_to_gpu(&a);
                let gb = ring_to_gpu(&b);
                let gr = ctx.gpu().run_ring_mul(&ga, &gb, n);
                gpu_to_ring(&gr, n)
            } else {
                a.mul(&b)
            }
        }
        _ => { eprintln!("error: unknown calc op '{}'", op); return; }
    };

    print_timed(backend, t.elapsed());
    print_poly(&result);
}

// ── ntt ──────────────────────────────────────────────────────────

fn cmd_ntt(forced: Option<Backend>, args: &[String]) {
    if args.len() < 3 {
        eprintln!("error: ntt requires: <forward|inverse> <n> <coeffs>");
        return;
    }
    let direction = args[0].as_str();
    let n: usize = args[1].parse().unwrap_or(0);
    if !n.is_power_of_two() || n > 4096 {
        eprintln!("error: n must be a power of 2 <= 4096");
        return;
    }
    let mut elem = parse_poly(&args[2], n);

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();
    let t = Instant::now();

    match direction {
        "forward" => {
            // NTT command uses jali's negacyclic NTT (CPU path) or raw NTT dispatch
            // For GPU: we do not have a standalone negacyclic NTT dispatch (that's part of ring_mul).
            // Fall back to CPU for standalone NTT.
            if backend == Backend::Gpu {
                eprintln!("\x1b[90m[ntt: using CPU — standalone NTT not yet on GPU]\x1b[0m");
            }
            ntt::to_ntt(&mut elem);
            print_timed(Backend::Cpu, t.elapsed());
            print_poly(&elem);
        }
        "inverse" => {
            if backend == Backend::Gpu {
                eprintln!("\x1b[90m[intt: using CPU — standalone INTT not yet on GPU]\x1b[0m");
            }
            elem.is_ntt = true;
            ntt::from_ntt(&mut elem);
            print_timed(Backend::Cpu, t.elapsed());
            print_poly(&elem);
        }
        _ => { eprintln!("error: unknown ntt direction '{}'", direction); }
    }
}

// ── sample ───────────────────────────────────────────────────────

fn cmd_sample(args: &[String]) {
    if args.len() < 3 {
        eprintln!("error: sample requires: <uniform|ternary|cbd> <seed> <n> [eta]");
        return;
    }
    let kind = args[0].as_str();
    let seed: u64 = args[1].parse().unwrap_or(0);
    let n: usize = args[2].parse().unwrap_or(0);
    if !n.is_power_of_two() || n > 4096 {
        eprintln!("error: n must be a power of 2 <= 4096");
        return;
    }
    let elem = match kind {
        "uniform" => sample::sample_uniform(seed, n),
        "ternary" => sample::sample_ternary(seed, n),
        "cbd" => {
            if args.len() < 4 {
                eprintln!("error: cbd requires eta parameter");
                return;
            }
            let eta: usize = args[3].parse().unwrap_or(2);
            sample::sample_cbd(seed, n, eta)
        }
        _ => { eprintln!("error: unknown sample kind '{}'", kind); return; }
    };
    print_poly(&elem);
}

// ── bench ────────────────────────────────────────────────────────

fn cmd_bench(forced: Option<Backend>, args: &[String]) {
    let n: usize = if args.is_empty() { 1024 } else { args[0].parse().unwrap_or(1024) };
    let iters: u64 = if args.len() < 2 { 1000 } else { args[1].parse().unwrap_or(1000) };

    if !n.is_power_of_two() || n > 4096 {
        eprintln!("error: n must be a power of 2 <= 4096");
        return;
    }

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();
    eprintln!("\x1b[90m[backend: {backend}, n={n}, iters={iters}]\x1b[0m");

    let a = sample::sample_uniform(1, n);
    let b = sample::sample_uniform(2, n);

    // ── CPU benchmarks ─────────────────────────────────────────
    eprintln!();
    eprintln!("\x1b[36m--- CPU ---\x1b[0m");

    // Benchmark add (CPU)
    let start = Instant::now();
    for _ in 0..iters {
        black_box(black_box(&a).add(black_box(&b)));
    }
    let elapsed = start.elapsed();
    eprintln!("ring_add  n={}: {:.1} us/op ({} iters)",
        n, elapsed.as_micros() as f64 / iters as f64, iters);

    // Benchmark mul (CPU)
    let mul_iters = iters.min(100);
    let start = Instant::now();
    for _ in 0..mul_iters {
        black_box(black_box(&a).mul(black_box(&b)));
    }
    let elapsed = start.elapsed();
    eprintln!("ring_mul  n={}: {:.1} us/op ({} iters)",
        n, elapsed.as_micros() as f64 / mul_iters as f64, mul_iters);

    // Benchmark NTT forward (CPU)
    let start = Instant::now();
    for _ in 0..iters {
        let mut c = a.clone();
        ntt::to_ntt(black_box(&mut c));
    }
    let elapsed = start.elapsed();
    eprintln!("ntt_fwd   n={}: {:.1} us/op ({} iters)",
        n, elapsed.as_micros() as f64 / iters as f64, iters);

    // Benchmark encoding roundtrip (CPU)
    let mut buf = vec![0u8; n * 8];
    let start = Instant::now();
    for _ in 0..iters {
        encoding::encode_ring(black_box(&a), black_box(&mut buf));
        black_box(encoding::decode_ring(black_box(&buf), n));
    }
    let elapsed = start.elapsed();
    eprintln!("enc_rt    n={}: {:.1} us/op ({} iters)",
        n, elapsed.as_micros() as f64 / iters as f64, iters);

    // ── GPU benchmarks ─────────────────────────────────────────
    if backend == Backend::Gpu {
        eprintln!();
        eprintln!("\x1b[33m--- GPU ---\x1b[0m");

        let ga = ring_to_gpu(&a);
        let gb = ring_to_gpu(&b);

        // Benchmark ring_add (GPU)
        let start = Instant::now();
        for _ in 0..iters {
            black_box(ctx.gpu().run_ring_add(black_box(&ga), black_box(&gb), n));
        }
        let elapsed = start.elapsed();
        eprintln!("ring_add  n={}: {:.1} us/op ({} iters)",
            n, elapsed.as_micros() as f64 / iters as f64, iters);

        // Benchmark ring_mul (GPU)
        let gpu_mul_iters = iters.min(100);
        let start = Instant::now();
        for _ in 0..gpu_mul_iters {
            black_box(ctx.gpu().run_ring_mul(black_box(&ga), black_box(&gb), n));
        }
        let elapsed = start.elapsed();
        eprintln!("ring_mul  n={}: {:.1} us/op ({} iters)",
            n, elapsed.as_micros() as f64 / gpu_mul_iters as f64, gpu_mul_iters);

        // Benchmark pointwise_mul (GPU)
        let start = Instant::now();
        for _ in 0..iters {
            black_box(ctx.gpu().run_ring_pointwise_mul(black_box(&ga), black_box(&gb), n));
        }
        let elapsed = start.elapsed();
        eprintln!("pointwise n={}: {:.1} us/op ({} iters)",
            n, elapsed.as_micros() as f64 / iters as f64, iters);
    }
}

// ── main ─────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    let cmd = args[1].clone();
    let (forced, rest) = parse_backend_flag(&args[2..]);

    match cmd.as_str() {
        "calc" => cmd_calc(forced, &rest),
        "ntt" => cmd_ntt(forced, &rest),
        "sample" => cmd_sample(&rest),
        "bench" => cmd_bench(forced, &rest),
        "help" | "--help" | "-h" => print_usage(),
        _ => {
            eprintln!("error: unknown command '{}'\n", cmd);
            print_usage();
        }
    }
}
