//! trop CLI — tropical semiring calculator, matrix operations, and benchmarks.
//!
//! (min, +) algebra: addition = min, multiplication = saturating add.
//! Supports GPU (wgpu) and CPU backends. GPU is default when available.
//! Use --gpu or --cpu flags to force a backend.

use std::env;
use std::hint::black_box;
use std::time::Instant;
use trop::{TropMatrix, Tropical, kleene_star};
use trop_wgsl::GpuContext;

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
        "matmul" => cmd_matmul(forced, &rest),
        "kleene" => cmd_kleene(forced, &rest),
        "bench" => cmd_bench(forced),
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
\x1b[90m
    ████████╗██████╗  ██████╗ ██████╗
    ╚══██╔══╝██╔══██╗██╔═══██╗██╔══██╗
       ██║   ██████╔╝██║   ██║██████╔╝
       ██║   ██╔══██╗██║   ██║██╔═══╝
       ██║   ██║  ██║╚██████╔╝██║
       ╚═╝   ╚═╝  ╚═╝ ╚═════╝ ╚═╝
\x1b[0m\x1b[37m    the tropical semiring\x1b[0m
\x1b[90m
    (min, +) algebra over u64
    Tropical add: a + b = min(a, b)       identity: +inf (u64::MAX)
    Tropical mul: a * b = a + b (sat)     identity: 0
    Kleene star = all-pairs shortest paths = Floyd-Warshall
\x1b[0m
  trop calc add <a> <b>                tropical addition: min(a, b)
  trop calc mul <a> <b>                tropical multiplication: a + b (saturating)
  trop matmul <n> <entries...>         multiply two n x n matrices
  trop kleene <n> <entries...>         Kleene star (all-pairs shortest paths)
  trop bench                           benchmark matmul and Kleene at various sizes
\x1b[90m
  values:  unsigned integers, INF for +infinity (u64::MAX)
  matmul:  2*n*n entries (matrix A then matrix B), row-major
  kleene:  n*n entries (one matrix), row-major

  flags:   --gpu  force GPU backend
           --cpu  force CPU backend
           (default: GPU if available, else CPU)
\x1b[0m
  -h, --help  Print this help"
    );
}

// -- argument helpers ---------------------------------------------------------

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

fn parse_tropical(s: &str) -> Tropical {
    if s.eq_ignore_ascii_case("INF") || s == "+inf" || s == "inf" {
        return Tropical::INF;
    }
    let v: u64 = s
        .parse()
        .unwrap_or_else(|e| die(&format!("invalid number '{s}': {e}")));
    Tropical::from_u64(v)
}

fn parse_usize(s: &str) -> usize {
    s.parse()
        .unwrap_or_else(|e| die(&format!("invalid dimension '{s}': {e}")))
}

fn fmt_trop(t: Tropical) -> String {
    if t.is_inf() {
        "INF".to_string()
    } else {
        t.as_u64().to_string()
    }
}

fn print_matrix(m: &TropMatrix) {
    for i in 0..m.n {
        let row: Vec<String> = (0..m.n)
            .map(|j| {
                let v = m.get(i, j);
                if v.is_inf() {
                    "INF".to_string()
                } else {
                    v.as_u64().to_string()
                }
            })
            .collect();
        println!("  [{}]", row.join(", "));
    }
}

fn parse_matrix(n: usize, entries: &[String]) -> TropMatrix {
    if entries.len() < n * n {
        die(&format!(
            "expected {} entries for {n}x{n} matrix, got {}",
            n * n,
            entries.len()
        ));
    }
    let mut m = TropMatrix::new(n);
    for i in 0..n {
        for j in 0..n {
            m.set(i, j, parse_tropical(&entries[i * n + j]));
        }
    }
    m
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

// -- GPU helpers --------------------------------------------------------------

/// Convert a TropMatrix to a flat u32 array (n x n, row-major, packed).
/// Tropical values are truncated to u32 for GPU. Values > u32::MAX become INF.
fn matrix_to_gpu(m: &TropMatrix) -> Vec<u32> {
    let n = m.n;
    let mut flat = vec![u32::MAX; n * n];
    for i in 0..n {
        for j in 0..n {
            let v = m.get(i, j).as_u64();
            flat[i * n + j] = if v >= u32::MAX as u64 {
                u32::MAX
            } else {
                v as u32
            };
        }
    }
    flat
}

/// Convert a flat u32 array (n x n, row-major) back to a TropMatrix.
fn gpu_to_matrix(data: &[u32], n: usize) -> TropMatrix {
    let mut m = TropMatrix::new(n);
    for i in 0..n {
        for j in 0..n {
            let v = data[i * n + j];
            let trop = if v == u32::MAX {
                Tropical::INF
            } else {
                Tropical::from_u64(v as u64)
            };
            m.set(i, j, trop);
        }
    }
    m
}

// -- commands ----------------------------------------------------------------

fn cmd_calc(forced: Option<Backend>, args: &[String]) {
    if args.len() < 3 {
        die("usage: trop calc <add|mul> <a> <b>");
    }
    let ctx = Ctx::new(forced);
    let backend = ctx.backend();
    let op = args[0].as_str();
    let a = parse_tropical(&args[1]);
    let b = parse_tropical(&args[2]);

    let a_u32 = if a.as_u64() >= u32::MAX as u64 {
        u32::MAX
    } else {
        a.as_u64() as u32
    };
    let b_u32 = if b.as_u64() >= u32::MAX as u64 {
        u32::MAX
    } else {
        b.as_u64() as u32
    };

    let t = Instant::now();
    let result = match (backend, op) {
        (Backend::Gpu, "add") => {
            let r = ctx
                .gpu()
                .eval_tropical_op(&format!("trop_add({}u, {}u)", a_u32, b_u32));
            if r == u32::MAX {
                Tropical::INF
            } else {
                Tropical::from_u64(r as u64)
            }
        }
        (Backend::Gpu, "mul") => {
            let r = ctx
                .gpu()
                .eval_tropical_op(&format!("trop_mul({}u, {}u)", a_u32, b_u32));
            if r == u32::MAX {
                Tropical::INF
            } else {
                Tropical::from_u64(r as u64)
            }
        }
        (Backend::Cpu, "add") => a.add(b),
        (Backend::Cpu, "mul") => a.mul(b),
        (_, other) => die(&format!("unknown calc op: {other}\nops: add, mul")),
    };
    let elapsed = t.elapsed();

    print_result(
        &format!(
            "{op}({}, {}) = {}",
            fmt_trop(a),
            fmt_trop(b),
            fmt_trop(result)
        ),
        backend,
        elapsed,
    );
}

fn cmd_matmul(forced: Option<Backend>, args: &[String]) {
    if args.is_empty() {
        die("usage: trop matmul <n> <entries...>");
    }
    let n = parse_usize(&args[0]);
    if n == 0 {
        die("dimension must be > 0");
    }
    let entries = &args[1..];
    let needed = 2 * n * n;
    if entries.len() < needed {
        die(&format!(
            "matmul: need {needed} entries (two {n}x{n} matrices), got {}",
            entries.len()
        ));
    }

    let a = parse_matrix(n, &entries[..n * n]);
    let b = parse_matrix(n, &entries[n * n..]);

    println!("A ({n}x{n}):");
    print_matrix(&a);
    println!("B ({n}x{n}):");
    print_matrix(&b);

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();

    let t = Instant::now();
    let c = if backend == Backend::Gpu {
        let ga = matrix_to_gpu(&a);
        let gb = matrix_to_gpu(&b);
        let gc = ctx.gpu().run_matmul(&ga, &gb, n);
        gpu_to_matrix(&gc, n)
    } else {
        a.mul(&b)
    };
    let elapsed = t.elapsed();

    println!("A * B ({n}x{n}):");
    print_matrix(&c);

    let us = elapsed.as_nanos() as f64 / 1000.0;
    if us < 1000.0 {
        eprintln!("\x1b[90m[{backend} {us:.0}us]\x1b[0m");
    } else {
        eprintln!("\x1b[90m[{backend} {:.2}ms]\x1b[0m", us / 1000.0);
    }
}

fn cmd_kleene(forced: Option<Backend>, args: &[String]) {
    if args.is_empty() {
        die("usage: trop kleene <n> <entries...>");
    }
    let n = parse_usize(&args[0]);
    if n == 0 {
        die("dimension must be > 0");
    }
    let entries = &args[1..];
    if entries.len() < n * n {
        die(&format!(
            "kleene: need {} entries ({n}x{n} matrix), got {}",
            n * n,
            entries.len()
        ));
    }

    let a = parse_matrix(n, entries);
    println!("A ({n}x{n}):");
    print_matrix(&a);

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();

    let t = Instant::now();
    let star = if backend == Backend::Gpu {
        // GPU Kleene star: iterative tropical matrix multiply.
        // A* = I + A + A^2 + ... + A^(n-1)
        // We compute via repeated squaring of (I + A), accumulating.
        // Simpler approach: Floyd-Warshall style using matmul iterations.
        //
        // Initialize D = I min A (identity merged with input).
        // Then iterate n-1 times: D = D * D (tropical matmul).
        // Actually, we use the standard approach:
        //   D = I min A, then for k in 0..ceil(log2(n)): D = D * D
        // This computes A* in O(log n) matrix multiplies.
        let mut d = TropMatrix::new(n);
        for i in 0..n {
            for j in 0..n {
                d.set(i, j, a.get(i, j));
            }
            // Merge with identity: diagonal = min(A[i][i], 0)
            let cur = d.get(i, i);
            d.set(i, i, cur.add(Tropical::ONE));
        }

        // Repeated squaring: ceil(log2(n)) iterations
        let iters = if n <= 1 {
            0
        } else {
            (n as f64).log2().ceil() as usize
        };
        let mut gd = matrix_to_gpu(&d);
        for _ in 0..iters {
            gd = ctx.gpu().run_matmul(&gd, &gd, n);
        }
        gpu_to_matrix(&gd, n)
    } else {
        kleene_star(&a)
    };
    let elapsed = t.elapsed();

    println!("A* ({n}x{n}):");
    print_matrix(&star);

    let us = elapsed.as_nanos() as f64 / 1000.0;
    if us < 1000.0 {
        eprintln!("\x1b[90m[{backend} {us:.0}us]\x1b[0m");
    } else {
        eprintln!("\x1b[90m[{backend} {:.2}ms]\x1b[0m", us / 1000.0);
    }
}

fn cmd_bench(forced: Option<Backend>) {
    println!("trop bench — tropical matmul and Kleene star\n");

    // If no backend forced, run both CPU and GPU side-by-side
    let run_cpu = forced != Some(Backend::Gpu);
    let run_gpu = forced != Some(Backend::Cpu);

    let gpu_ctx = if run_gpu { GpuContext::new() } else { None };
    let has_gpu = gpu_ctx.is_some();

    if forced.is_none() {
        if has_gpu {
            eprintln!("\x1b[90m[comparing CPU vs GPU]\x1b[0m");
        } else {
            eprintln!("\x1b[90m[CPU only — no GPU adapter]\x1b[0m");
        }
    } else {
        eprintln!("\x1b[90m[backend: {}]\x1b[0m", forced.unwrap());
    }
    println!();

    for &n in &[4usize, 8, 16, 32] {
        // Build test matrices with deterministic weights.
        let mut a = TropMatrix::new(n);
        for i in 0..n {
            for j in 0..n {
                let w = ((i * n + j) % 97) as u64;
                a.set(i, j, Tropical::from_u64(w));
            }
        }
        let mut b = TropMatrix::new(n);
        for i in 0..n {
            for j in 0..n {
                let w = ((j * n + i + 13) % 89) as u64;
                b.set(i, j, Tropical::from_u64(w));
            }
        }

        let iters = match n {
            4 => 100_000u64,
            8 => 10_000,
            16 => 1_000,
            32 => 100,
            _ => 10,
        };

        let gpu_iters = match n {
            4 => 10_000u64,
            8 => 1_000,
            16 => 500,
            32 => 100,
            _ => 10,
        };

        let fmt_time = |ns: f64| -> String {
            if ns < 1_000.0 {
                format!("{ns:>8.1} ns")
            } else if ns < 1_000_000.0 {
                format!("{:>8.1} us", ns / 1_000.0)
            } else {
                format!("{:>8.2} ms", ns / 1_000_000.0)
            }
        };

        // CPU matmul
        let cpu_matmul = if run_cpu {
            let t = Instant::now();
            for _ in 0..iters {
                black_box(black_box(&a).mul(black_box(&b)));
            }
            Some(t.elapsed().as_nanos() as f64 / iters as f64)
        } else {
            None
        };

        // GPU matmul
        let gpu_matmul = if run_gpu && has_gpu {
            let gpu = gpu_ctx.as_ref().unwrap();
            let ga = matrix_to_gpu(&a);
            let gb = matrix_to_gpu(&b);
            // warmup
            for _ in 0..gpu_iters.min(10) {
                black_box(gpu.run_matmul(black_box(&ga), black_box(&gb), n));
            }
            let t = Instant::now();
            for _ in 0..gpu_iters {
                black_box(gpu.run_matmul(black_box(&ga), black_box(&gb), n));
            }
            Some(t.elapsed().as_nanos() as f64 / gpu_iters as f64)
        } else {
            None
        };

        // CPU Kleene
        let cpu_kleene = if run_cpu {
            let t = Instant::now();
            for _ in 0..iters {
                black_box(kleene_star(black_box(&a)));
            }
            Some(t.elapsed().as_nanos() as f64 / iters as f64)
        } else {
            None
        };

        // GPU Kleene (iterative matmul)
        let gpu_kleene = if run_gpu && has_gpu {
            let gpu = gpu_ctx.as_ref().unwrap();
            let kleene_iters_log = if n <= 1 {
                0
            } else {
                (n as f64).log2().ceil() as usize
            };
            let mut d = TropMatrix::new(n);
            for i in 0..n {
                for j in 0..n {
                    d.set(i, j, a.get(i, j));
                }
                let cur = d.get(i, i);
                d.set(i, i, cur.add(Tropical::ONE));
            }
            let gd_init = matrix_to_gpu(&d);
            // warmup
            for _ in 0..gpu_iters.min(10) {
                let mut gd = gd_init.clone();
                for _ in 0..kleene_iters_log {
                    gd = gpu.run_matmul(&gd, &gd, n);
                }
                black_box(gd);
            }
            let t = Instant::now();
            for _ in 0..gpu_iters {
                let mut gd = gd_init.clone();
                for _ in 0..kleene_iters_log {
                    gd = gpu.run_matmul(&gd, &gd, n);
                }
                black_box(gd);
            }
            Some(t.elapsed().as_nanos() as f64 / gpu_iters as f64)
        } else {
            None
        };

        // Print results
        match (cpu_matmul, gpu_matmul) {
            (Some(cpu), Some(gpu)) => {
                let speedup = cpu / gpu;
                println!(
                    "  {n:>2}x{n:<2}  matmul  cpu {}  gpu {}  ({speedup:.1}x)",
                    fmt_time(cpu),
                    fmt_time(gpu)
                );
            }
            (Some(cpu), None) => {
                println!(
                    "  {n:>2}x{n:<2}  matmul  cpu {}  ({iters} iters)",
                    fmt_time(cpu)
                );
            }
            (None, Some(gpu)) => {
                println!(
                    "  {n:>2}x{n:<2}  matmul  gpu {}  ({gpu_iters} iters)",
                    fmt_time(gpu)
                );
            }
            _ => {}
        }

        match (cpu_kleene, gpu_kleene) {
            (Some(cpu), Some(gpu)) => {
                let speedup = cpu / gpu;
                println!(
                    "  {n:>2}x{n:<2}  kleene  cpu {}  gpu {}  ({speedup:.1}x)",
                    fmt_time(cpu),
                    fmt_time(gpu)
                );
            }
            (Some(cpu), None) => {
                println!(
                    "  {n:>2}x{n:<2}  kleene  cpu {}  ({iters} iters)",
                    fmt_time(cpu)
                );
            }
            (None, Some(gpu)) => {
                println!(
                    "  {n:>2}x{n:<2}  kleene  gpu {}  ({gpu_iters} iters)",
                    fmt_time(gpu)
                );
            }
            _ => {}
        }
    }
}
