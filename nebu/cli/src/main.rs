// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! nebu CLI — Goldilocks field calculator, encoder, NTT, and benchmarks.
//!
//! Supports GPU (wgpu) and CPU backends. GPU is default when available.
//! Use --gpu or --cpu flags to force a backend.

use nebu::field::{Goldilocks, P};
use nebu::{Fp2, batch, encoding, ntt, sqrt};
use nebu_wgsl::GpuContext;
use std::env;
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }
    let cmd = args[1].clone();
    let (forced, rest) = parse_backend_flag(&args[2..]);

    match cmd.as_str() {
        "calc" => cmd_calc(forced, &rest),
        "encode" => cmd_encode(&rest),
        "decode" => cmd_decode(&rest),
        "ntt" => cmd_ntt(forced, &rest),
        "intt" => cmd_intt(forced, &rest),
        "bench" => cmd_bench(forced, &rest),
        "help" | "--help" | "-h" => print_usage(),
        other => {
            eprintln!("unknown command: {other}");
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!(
        "\
\x1b[33m
     ███╗   ██╗███████╗██████╗ ██╗   ██╗
\x1b[33m    ████╗  ██║██╔════╝██╔══██╗██║   ██║
\x1b[32m    ██╔██╗ ██║█████╗  ██████╔╝██║   ██║
\x1b[36m    ██║╚██╗██║██╔══╝  ██╔══██╗██║   ██║
\x1b[34m    ██║ ╚████║███████╗██████╔╝╚██████╔╝
\x1b[35m    ╚═╝  ╚═══╝╚══════╝╚═════╝  ╚═════╝
\x1b[0m\x1b[37m    the golden field\x1b[0m
\x1b[90m
    Goldilocks · p = 2^64 - 2^32 + 1 · g=7 · two-adicity=32
    F_p2 = F_p[u]/(u^2-7) · NTT up to 2^32 · sqrt via Tonelli-Shanks
\x1b[0m
  nebu calc <op> <args...>      field arithmetic
  nebu encode <bytes>           bytes to field elements
  nebu encode --hex <hex>       hex to field elements
  nebu decode <elem...>         field elements to bytes
  nebu ntt <elem...>            forward NTT
  nebu intt <elem...>           inverse NTT
  nebu bench [op] [iterations]  benchmark operations
\x1b[90m
  calc ops: add sub mul inv neg sqrt pow7 exp square legendre
            batch-inv fp2-add fp2-sub fp2-mul fp2-inv fp2-conj fp2-norm
  values:   decimal or hex (0x prefix)
  flags:    --gpu  force GPU backend
            --cpu  force CPU backend
            (default: GPU if available, else CPU)
\x1b[0m
  -h, --help  Print this help"
    );
}

// ── argument parsing ───────────────────────────────────────────────

fn parse_u64(s: &str) -> u64 {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).unwrap_or_else(|e| {
            eprintln!("invalid hex '{s}': {e}");
            std::process::exit(1);
        })
    } else {
        s.parse::<u64>().unwrap_or_else(|e| {
            eprintln!("invalid number '{s}': {e}");
            std::process::exit(1);
        })
    }
}

fn parse_field(s: &str) -> Goldilocks {
    Goldilocks::new(parse_u64(s))
}

fn fmt_field(g: Goldilocks) -> String {
    format!("0x{:016X}", g.as_u64())
}

fn need_args(args: &[String], n: usize, usage: &str) {
    if args.len() < n {
        eprintln!("usage: nebu calc {usage}");
        std::process::exit(1);
    }
}

// ── GPU field helpers ──────────────────────────────────────────────

fn to_lohi(g: Goldilocks) -> (u32, u32) {
    let v = g.as_u64();
    (v as u32, (v >> 32) as u32)
}

fn from_lohi(lo: u32, hi: u32) -> Goldilocks {
    Goldilocks::new((hi as u64) << 32 | lo as u64)
}

fn gpu_field_op(ctx: &Ctx, op: &str) -> Goldilocks {
    let (lo, hi) = ctx.gpu().eval_field_op(op);
    from_lohi(lo, hi)
}

fn wgsl_args1(a: Goldilocks) -> String {
    let (lo, hi) = to_lohi(a);
    format!("{}u, {}u", lo, hi)
}

fn wgsl_args2(a: Goldilocks, b: Goldilocks) -> String {
    let (alo, ahi) = to_lohi(a);
    let (blo, bhi) = to_lohi(b);
    format!("{}u, {}u, {}u, {}u", alo, ahi, blo, bhi)
}

/// Print result with backend and timing info.
fn print_result(result: &str, backend: Backend, elapsed: std::time::Duration) {
    let us = elapsed.as_nanos() as f64 / 1000.0;
    if us < 1000.0 {
        eprintln!("\x1b[90m[{backend} {us:.0}us]\x1b[0m");
    } else {
        eprintln!("\x1b[90m[{backend} {:.2}ms]\x1b[0m", us / 1000.0);
    }
    println!("{result}");
}

fn print_results(results: &[String], backend: Backend, elapsed: std::time::Duration) {
    let us = elapsed.as_nanos() as f64 / 1000.0;
    if us < 1000.0 {
        eprintln!("\x1b[90m[{backend} {us:.0}us]\x1b[0m");
    } else {
        eprintln!("\x1b[90m[{backend} {:.2}ms]\x1b[0m", us / 1000.0);
    }
    for r in results {
        println!("{r}");
    }
}

// ── calc ───────────────────────────────────────────────────────────

fn cmd_calc(forced: Option<Backend>, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: nebu calc <op> <args...>");
        std::process::exit(1);
    }
    let ctx = Ctx::new(forced);
    let backend = ctx.backend();

    match args[0].as_str() {
        "add" => {
            need_args(args, 3, "add <a> <b>");
            let a = parse_field(&args[1]);
            let b = parse_field(&args[2]);
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_field_op(&ctx, &format!("gl_add({})", wgsl_args2(a, b)))
            } else {
                a + b
            };
            print_result(&fmt_field(r), backend, t.elapsed());
        }
        "sub" => {
            need_args(args, 3, "sub <a> <b>");
            let a = parse_field(&args[1]);
            let b = parse_field(&args[2]);
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_field_op(&ctx, &format!("gl_sub({})", wgsl_args2(a, b)))
            } else {
                a - b
            };
            print_result(&fmt_field(r), backend, t.elapsed());
        }
        "mul" => {
            need_args(args, 3, "mul <a> <b>");
            let a = parse_field(&args[1]);
            let b = parse_field(&args[2]);
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_field_op(&ctx, &format!("gl_mul({})", wgsl_args2(a, b)))
            } else {
                a * b
            };
            print_result(&fmt_field(r), backend, t.elapsed());
        }
        "inv" => {
            need_args(args, 2, "inv <a>");
            let a = parse_field(&args[1]);
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_field_op(&ctx, &format!("gl_inv({})", wgsl_args1(a)))
            } else {
                a.inv()
            };
            print_result(&fmt_field(r), backend, t.elapsed());
        }
        "neg" => {
            need_args(args, 2, "neg <a>");
            let a = parse_field(&args[1]);
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_field_op(&ctx, &format!("gl_neg({})", wgsl_args1(a)))
            } else {
                -a
            };
            print_result(&fmt_field(r), backend, t.elapsed());
        }
        "sqrt" => {
            // Tonelli-Shanks is too expensive for single GPU thread — always CPU
            need_args(args, 2, "sqrt <a>");
            let a = parse_field(&args[1]);
            let t = Instant::now();
            let r = sqrt::sqrt(a);
            let elapsed = t.elapsed();
            match r {
                Some(v) => print_result(&fmt_field(v), Backend::Cpu, elapsed),
                None => print_result("none", Backend::Cpu, elapsed),
            }
        }
        "pow7" => {
            need_args(args, 2, "pow7 <a>");
            let a = parse_field(&args[1]);
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_field_op(&ctx, &format!("gl_pow7({})", wgsl_args1(a)))
            } else {
                a.pow7()
            };
            print_result(&fmt_field(r), backend, t.elapsed());
        }
        "exp" => {
            need_args(args, 3, "exp <a> <e>");
            let a = parse_field(&args[1]);
            let e = parse_u64(&args[2]);
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_field_op(&ctx, &format!(
                    "gl_exp({}, {}u, {}u)", wgsl_args1(a), e as u32, (e >> 32) as u32
                ))
            } else {
                a.exp(e)
            };
            print_result(&fmt_field(r), backend, t.elapsed());
        }
        "square" => {
            need_args(args, 2, "square <a>");
            let a = parse_field(&args[1]);
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_field_op(&ctx, &format!("gl_square({})", wgsl_args1(a)))
            } else {
                a.square()
            };
            print_result(&fmt_field(r), backend, t.elapsed());
        }
        "legendre" => {
            // gl_legendre uses gl_exp with 64-bit exponent — always CPU
            need_args(args, 2, "legendre <a>");
            let a = parse_field(&args[1]);
            let t = Instant::now();
            let r = sqrt::legendre(a);
            print_result(&fmt_field(r), Backend::Cpu, t.elapsed());
        }
        "batch-inv" => {
            need_args(args, 2, "batch-inv <a1> <a2> ...");
            let inputs: Vec<Goldilocks> = args[1..].iter().map(|s| parse_field(s)).collect();
            // batch-inv is CPU-only (sequential algorithm)
            let t = Instant::now();
            let mut results = vec![Goldilocks::default(); inputs.len()];
            batch::batch_inv(&inputs, &mut results);
            let elapsed = t.elapsed();
            let lines: Vec<String> = results.iter().map(|r| fmt_field(*r)).collect();
            print_results(&lines, Backend::Cpu, elapsed);
        }
        "fp2-add" => {
            need_args(args, 5, "fp2-add <re1> <im1> <re2> <im2>");
            let x = Fp2::new(parse_field(&args[1]), parse_field(&args[2]));
            let y = Fp2::new(parse_field(&args[3]), parse_field(&args[4]));
            // fp2 add is just two field adds — CPU is fast enough, GPU supported too
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                let res = ctx.gpu().run_custom(
                    &format!(
                        "let z = fp2_add(fp2_new({}, {}), fp2_new({}, {}));\n\
                         let re = gl_canon(z.x, z.y);\n\
                         let im = gl_canon(z.z, z.w);\n\
                         out[0] = re.x; out[1] = re.y; out[2] = im.x; out[3] = im.y;",
                        wgsl_args1(x.re), wgsl_args1(x.im),
                        wgsl_args1(y.re), wgsl_args1(y.im)
                    ),
                    4,
                );
                Fp2::new(from_lohi(res[0], res[1]), from_lohi(res[2], res[3]))
            } else {
                x + y
            };
            print_result(&format!("{} {}", fmt_field(r.re), fmt_field(r.im)), backend, t.elapsed());
        }
        "fp2-sub" => {
            need_args(args, 5, "fp2-sub <re1> <im1> <re2> <im2>");
            let x = Fp2::new(parse_field(&args[1]), parse_field(&args[2]));
            let y = Fp2::new(parse_field(&args[3]), parse_field(&args[4]));
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                let res = ctx.gpu().run_custom(
                    &format!(
                        "let z = fp2_sub(fp2_new({}, {}), fp2_new({}, {}));\n\
                         let re = gl_canon(z.x, z.y);\n\
                         let im = gl_canon(z.z, z.w);\n\
                         out[0] = re.x; out[1] = re.y; out[2] = im.x; out[3] = im.y;",
                        wgsl_args1(x.re), wgsl_args1(x.im),
                        wgsl_args1(y.re), wgsl_args1(y.im)
                    ),
                    4,
                );
                Fp2::new(from_lohi(res[0], res[1]), from_lohi(res[2], res[3]))
            } else {
                x - y
            };
            print_result(&format!("{} {}", fmt_field(r.re), fmt_field(r.im)), backend, t.elapsed());
        }
        "fp2-mul" => {
            need_args(args, 5, "fp2-mul <re1> <im1> <re2> <im2>");
            let x = Fp2::new(parse_field(&args[1]), parse_field(&args[2]));
            let y = Fp2::new(parse_field(&args[3]), parse_field(&args[4]));
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                let res = ctx.gpu().run_custom(
                    &format!(
                        "let z = fp2_mul(fp2_new({}, {}), fp2_new({}, {}));\n\
                         let re = gl_canon(z.x, z.y);\n\
                         let im = gl_canon(z.z, z.w);\n\
                         out[0] = re.x; out[1] = re.y; out[2] = im.x; out[3] = im.y;",
                        wgsl_args1(x.re), wgsl_args1(x.im),
                        wgsl_args1(y.re), wgsl_args1(y.im)
                    ),
                    4,
                );
                Fp2::new(from_lohi(res[0], res[1]), from_lohi(res[2], res[3]))
            } else {
                x * y
            };
            print_result(&format!("{} {}", fmt_field(r.re), fmt_field(r.im)), backend, t.elapsed());
        }
        "fp2-inv" => {
            need_args(args, 3, "fp2-inv <re> <im>");
            let x = Fp2::new(parse_field(&args[1]), parse_field(&args[2]));
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                let res = ctx.gpu().run_custom(
                    &format!(
                        "let z = fp2_inv(fp2_new({}, {}));\n\
                         let re = gl_canon(z.x, z.y);\n\
                         let im = gl_canon(z.z, z.w);\n\
                         out[0] = re.x; out[1] = re.y; out[2] = im.x; out[3] = im.y;",
                        wgsl_args1(x.re), wgsl_args1(x.im)
                    ),
                    4,
                );
                Fp2::new(from_lohi(res[0], res[1]), from_lohi(res[2], res[3]))
            } else {
                x.inv()
            };
            print_result(&format!("{} {}", fmt_field(r.re), fmt_field(r.im)), backend, t.elapsed());
        }
        "fp2-conj" => {
            need_args(args, 3, "fp2-conj <re> <im>");
            let x = Fp2::new(parse_field(&args[1]), parse_field(&args[2]));
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                let res = ctx.gpu().run_custom(
                    &format!(
                        "let z = fp2_conj(fp2_new({}, {}));\n\
                         let re = gl_canon(z.x, z.y);\n\
                         let im = gl_canon(z.z, z.w);\n\
                         out[0] = re.x; out[1] = re.y; out[2] = im.x; out[3] = im.y;",
                        wgsl_args1(x.re), wgsl_args1(x.im)
                    ),
                    4,
                );
                Fp2::new(from_lohi(res[0], res[1]), from_lohi(res[2], res[3]))
            } else {
                x.conj()
            };
            print_result(&format!("{} {}", fmt_field(r.re), fmt_field(r.im)), backend, t.elapsed());
        }
        "fp2-norm" => {
            need_args(args, 3, "fp2-norm <re> <im>");
            let x = Fp2::new(parse_field(&args[1]), parse_field(&args[2]));
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                let res = ctx.gpu().run_custom(
                    &format!(
                        "let z = fp2_norm(fp2_new({}, {}));\n\
                         let c = gl_canon(z.x, z.y);\n\
                         out[0] = c.x; out[1] = c.y;",
                        wgsl_args1(x.re), wgsl_args1(x.im)
                    ),
                    2,
                );
                from_lohi(res[0], res[1])
            } else {
                x.norm()
            };
            print_result(&fmt_field(r), backend, t.elapsed());
        }
        other => {
            eprintln!("unknown calc op: {other}");
            std::process::exit(1);
        }
    }
}

// ── encode ─────────────────────────────────────────────────────────

fn cmd_encode(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: nebu encode [--hex] <data>");
        std::process::exit(1);
    }

    let bytes = if args[0] == "--hex" {
        need_args(args, 2, "");
        decode_hex(&args[1])
    } else {
        args[0].as_bytes().to_vec()
    };

    let max_elems = (bytes.len() + 6) / 7;
    let mut out = vec![Goldilocks::default(); max_elems];
    let n = encoding::bytes_to_field_elements(&bytes, &mut out);
    for i in 0..n {
        println!("{}", fmt_field(out[i]));
    }
}

// ── decode ─────────────────────────────────────────────────────────

fn cmd_decode(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: nebu decode <element1> [element2] ...");
        std::process::exit(1);
    }

    let elems: Vec<Goldilocks> = args.iter().map(|s| parse_field(s)).collect();
    let mut out = vec![0u8; elems.len() * 8];
    let n = encoding::field_elements_to_bytes(&elems, &mut out);
    let mut hex = String::with_capacity(n * 2);
    for b in &out[..n] {
        hex.push_str(&format!("{:02x}", b));
    }
    println!("{hex}");
}

// ── ntt / intt ─────────────────────────────────────────────────────

fn cmd_ntt(forced: Option<Backend>, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: nebu ntt <e1> <e2> ... (length must be power of 2)");
        std::process::exit(1);
    }
    let data: Vec<Goldilocks> = args.iter().map(|s| parse_field(s)).collect();
    if !data.len().is_power_of_two() {
        eprintln!("error: length {} is not a power of 2", data.len());
        std::process::exit(1);
    }

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();
    let t = Instant::now();

    let results = if backend == Backend::Gpu {
        let mut gpu_data: Vec<(u32, u32)> = data.iter().map(|g| to_lohi(*g)).collect();
        ctx.gpu().run_ntt(&mut gpu_data);
        gpu_data.iter().map(|&(lo, hi)| from_lohi(lo, hi)).collect::<Vec<_>>()
    } else {
        let mut cpu_data = data;
        ntt::ntt(&mut cpu_data);
        cpu_data
    };

    let elapsed = t.elapsed();
    let lines: Vec<String> = results.iter().map(|x| fmt_field(*x)).collect();
    print_results(&lines, backend, elapsed);
}

fn cmd_intt(forced: Option<Backend>, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: nebu intt <e1> <e2> ... (length must be power of 2)");
        std::process::exit(1);
    }
    let data: Vec<Goldilocks> = args.iter().map(|s| parse_field(s)).collect();
    if !data.len().is_power_of_two() {
        eprintln!("error: length {} is not a power of 2", data.len());
        std::process::exit(1);
    }

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();
    let t = Instant::now();

    let results = if backend == Backend::Gpu {
        let mut gpu_data: Vec<(u32, u32)> = data.iter().map(|g| to_lohi(*g)).collect();
        ctx.gpu().run_intt(&mut gpu_data);
        gpu_data.iter().map(|&(lo, hi)| from_lohi(lo, hi)).collect::<Vec<_>>()
    } else {
        let mut cpu_data = data;
        ntt::intt(&mut cpu_data);
        cpu_data
    };

    let elapsed = t.elapsed();
    let lines: Vec<String> = results.iter().map(|x| fmt_field(*x)).collect();
    print_results(&lines, backend, elapsed);
}

// ── bench ──────────────────────────────────────────────────────────

fn cmd_bench(forced: Option<Backend>, args: &[String]) {
    let (op, iters) = match args.len() {
        0 => ("all", 1_000_000u64),
        1 => (args[0].as_str(), 1_000_000u64),
        _ => (args[0].as_str(), parse_u64(&args[1])),
    };

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();
    eprintln!("\x1b[90m[backend: {backend}]\x1b[0m");

    let a = Goldilocks::new(0x123456789ABCDEF0);
    let b = Goldilocks::new(0xFEDCBA9876543210);

    if op == "all" || op == "add" {
        if backend == Backend::Gpu {
            bench_op("add", iters, || {
                black_box(ctx.gpu().eval_field_op(
                    &format!("gl_add({})", wgsl_args2(a, b))
                ));
            });
        } else {
            bench_op("add", iters, || { black_box(black_box(a) + black_box(b)); });
        }
    }
    if op == "all" || op == "sub" {
        if backend == Backend::Gpu {
            bench_op("sub", iters, || {
                black_box(ctx.gpu().eval_field_op(
                    &format!("gl_sub({})", wgsl_args2(a, b))
                ));
            });
        } else {
            bench_op("sub", iters, || { black_box(black_box(a) - black_box(b)); });
        }
    }
    if op == "all" || op == "mul" {
        if backend == Backend::Gpu {
            bench_op("mul", iters, || {
                black_box(ctx.gpu().eval_field_op(
                    &format!("gl_mul({})", wgsl_args2(a, b))
                ));
            });
        } else {
            bench_op("mul", iters, || { black_box(black_box(a) * black_box(b)); });
        }
    }
    if op == "all" || op == "square" {
        if backend == Backend::Gpu {
            bench_op("square", iters, || {
                black_box(ctx.gpu().eval_field_op(
                    &format!("gl_square({})", wgsl_args1(a))
                ));
            });
        } else {
            bench_op("square", iters, || { black_box(black_box(a).square()); });
        }
    }
    if op == "all" || op == "pow7" {
        if backend == Backend::Gpu {
            bench_op("pow7", iters, || {
                black_box(ctx.gpu().eval_field_op(
                    &format!("gl_pow7({})", wgsl_args1(a))
                ));
            });
        } else {
            bench_op("pow7", iters, || { black_box(black_box(a).pow7()); });
        }
    }
    if op == "all" || op == "inv" {
        let inv_iters = iters.min(100_000);
        if backend == Backend::Gpu {
            bench_op("inv", inv_iters.min(1000), || {
                black_box(ctx.gpu().eval_field_op(
                    &format!("gl_inv({})", wgsl_args1(a))
                ));
            });
        } else {
            bench_op("inv", inv_iters, || { black_box(black_box(a).inv()); });
        }
    }
    if op == "all" || op == "sqrt" {
        let sqrt_iters = iters.min(10_000);
        // sqrt on GPU can timeout — use CPU for bench
        bench_op("sqrt", sqrt_iters, || { black_box(sqrt::sqrt(black_box(a))); });
    }
    if op == "all" || op == "exp" {
        let exp_iters = iters.min(100_000);
        if backend == Backend::Gpu {
            bench_op("exp", exp_iters.min(1000), || {
                black_box(ctx.gpu().eval_field_op(
                    &format!("gl_exp({}, {}u, {}u)", wgsl_args1(a), (P - 2) as u32, ((P - 2) >> 32) as u32)
                ));
            });
        } else {
            bench_op("exp", exp_iters, || { black_box(black_box(a).exp(P - 2)); });
        }
    }
    if op == "all" || op == "batch-inv" {
        let n = 256;
        let inputs: Vec<Goldilocks> = (1..=n).map(|i| Goldilocks::new(i as u64)).collect();
        let mut results = vec![Goldilocks::default(); n];
        let bi_iters = iters.min(10_000);
        bench_op(&format!("batch-inv({n})"), bi_iters, || {
            batch::batch_inv(black_box(&inputs), black_box(&mut results));
        });
    }
    if op == "all" || op == "ntt" {
        let n = 1024;
        if backend == Backend::Gpu {
            let mut gpu_data: Vec<(u32, u32)> = (0..n).map(|i| (i as u32, 0u32)).collect();
            let ntt_iters = iters.min(1000);
            bench_op(&format!("ntt({n})"), ntt_iters, || {
                ctx.gpu().run_ntt(black_box(&mut gpu_data));
            });
        } else {
            let mut data: Vec<Goldilocks> = (0..n).map(|i| Goldilocks::new(i as u64)).collect();
            let ntt_iters = iters.min(10_000);
            bench_op(&format!("ntt({n})"), ntt_iters, || {
                ntt::ntt(black_box(&mut data));
            });
        }
    }
    if op == "all" || op == "fp2-mul" {
        let x = Fp2::new(a, b);
        let y = Fp2::new(b, a);
        if backend == Backend::Gpu {
            bench_op("fp2-mul", iters.min(100_000), || {
                black_box(ctx.gpu().run_custom(
                    &format!(
                        "let z = fp2_mul(fp2_new({}, {}), fp2_new({}, {}));\n\
                         out[0] = z.x; out[1] = z.y; out[2] = z.z; out[3] = z.w;",
                        wgsl_args1(x.re), wgsl_args1(x.im),
                        wgsl_args1(y.re), wgsl_args1(y.im)
                    ),
                    4,
                ));
            });
        } else {
            bench_op("fp2-mul", iters, || { black_box(black_box(x) * black_box(y)); });
        }
    }
    if op == "all" || op == "fp2-inv" {
        let x = Fp2::new(a, b);
        if backend == Backend::Gpu {
            bench_op("fp2-inv", iters.min(1000), || {
                black_box(ctx.gpu().run_custom(
                    &format!(
                        "let z = fp2_inv(fp2_new({}, {}));\n\
                         out[0] = z.x; out[1] = z.y; out[2] = z.z; out[3] = z.w;",
                        wgsl_args1(x.re), wgsl_args1(x.im)
                    ),
                    4,
                ));
            });
        } else {
            let inv_iters = iters.min(100_000);
            bench_op("fp2-inv", inv_iters, || { black_box(black_box(x).inv()); });
        }
    }
}

fn bench_op<F: FnMut()>(name: &str, iters: u64, mut f: F) {
    for _ in 0..iters.min(1000) {
        f();
    }

    let start = Instant::now();
    for _ in 0..iters {
        f();
    }
    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() as f64 / iters as f64;

    if ns_per_op < 1000.0 {
        println!("{name:>20}  {ns_per_op:8.1} ns/op  ({iters} iters)");
    } else if ns_per_op < 1_000_000.0 {
        println!("{name:>20}  {:8.1} us/op  ({iters} iters)", ns_per_op / 1000.0);
    } else {
        println!("{name:>20}  {:8.1} ms/op  ({iters} iters)", ns_per_op / 1_000_000.0);
    }
}

// ── hex helpers ────────────────────────────────────────────────────

fn decode_hex(s: &str) -> Vec<u8> {
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if s.len() % 2 != 0 {
        eprintln!("hex string must have even length");
        std::process::exit(1);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).unwrap_or_else(|e| {
                eprintln!("invalid hex at position {i}: {e}");
                std::process::exit(1);
            })
        })
        .collect()
}
