//! kuro CLI — F2 tower field calculator, encoder, decoder, and benchmarks.
//!
//! Binary field tower: F2 -> F2^2 -> F2^4 -> ... -> F2^128
//! Supports GPU (wgpu) and CPU backends. GPU is default when available.
//! Use --gpu or --cpu flags to force a backend.

use kuro::{F2_2, F2_4, F2_8, F2_16, F2_32, F2_64, F2_128, Packed128};
use kuro_wgsl::GpuContext;
use std::env;
use std::hint::black_box;
use std::time::Instant;

// -- backend selection -------------------------------------------------------

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
    if args.len() < 2 { print_usage(); std::process::exit(1); }
    let cmd = args[1].clone();
    let (forced, rest) = parse_backend_flag(&args[2..]);

    match cmd.as_str() {
        "calc"                    => cmd_calc(forced, &rest),
        "encode"                  => cmd_encode(&rest),
        "decode"                  => cmd_decode(&rest),
        "bench"                   => cmd_bench(forced, &rest),
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
    ██╗  ██╗██╗   ██╗██████╗  ██████╗
    ██║ ██╔╝██║   ██║██╔══██╗██╔═══██╗
    █████╔╝ ██║   ██║██████╔╝██║   ██║
    ██╔═██╗ ██║   ██║██╔══██╗██║   ██║
    ██║  ██╗╚██████╔╝██║  ██║╚██████╔╝
    ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝ ╚═════╝
\x1b[0m\x1b[37m    the black field\x1b[0m
\x1b[90m
    F2 tower: F2 -> F2^2 -> F2^4 -> F2^8 -> F2^16 -> F2^32 -> F2^64 -> F2^128
    Addition = XOR · Multiplication = Karatsuba over tower levels
    Inversion = tower-recursive via Fermat · Frobenius = iterated squaring
    Each extension: x^2 + x + alpha, alpha from previous level
\x1b[0m
  kuro calc <op> <args...>              field arithmetic
  kuro calc add <a> <b> [--level N]     XOR two elements
  kuro calc mul <a> <b> [--level N]     Karatsuba multiply
  kuro calc square <a> [--level N]      a * a (optimized)
  kuro calc inv <a> [--level N]         tower-recursive inverse (Fermat)
  kuro calc frobenius <a> [--level N]   Frobenius endomorphism (x -> x^2)
  kuro encode <hex_bytes>               bytes to F2^128 tower elements
  kuro decode <hex_element>             F2^128 tower element to bytes
  kuro bench [iterations]               benchmark all operations
\x1b[90m
  values:  hex with 0x prefix (e.g. 0xff, 0xdeadbeef)
  --level  tower level in bits: 2, 4, 8, 16, 32, 64, 128 (default: 128)
  flags:   --gpu  force GPU backend
           --cpu  force CPU backend
           (default: GPU if available, else CPU)
\x1b[0m
  -h, --help  Print this help");
}

// -- argument helpers ---------------------------------------------------------

fn die(msg: &str) -> ! { eprintln!("{msg}"); std::process::exit(1); }

fn parse_u128(s: &str) -> u128 {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u128::from_str_radix(hex, 16).unwrap_or_else(|e| die(&format!("invalid hex '{s}': {e}")))
    } else {
        s.parse::<u128>().unwrap_or_else(|e| die(&format!("invalid number '{s}': {e}")))
    }
}

fn parse_level(args: &[String]) -> u32 {
    for i in 0..args.len() {
        if args[i] == "--level" {
            if i + 1 >= args.len() { die("--level requires a value (2, 4, 8, 16, 32, 64, 128)"); }
            let lvl: u32 = args[i + 1].parse()
                .unwrap_or_else(|e| die(&format!("invalid level '{}': {e}", args[i + 1])));
            return match lvl {
                2 | 4 | 8 | 16 | 32 | 64 | 128 => lvl,
                _ => die("level must be one of: 2, 4, 8, 16, 32, 64, 128"),
            };
        }
    }
    128
}

/// Collect positional args, skipping --level and its value.
fn positional(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if skip { skip = false; continue; }
        if a == "--level" { skip = true; continue; }
        out.push(a.clone());
    }
    out
}

fn need(args: &[String], n: usize, usage: &str) {
    if args.len() < n { die(&format!("usage: kuro calc {usage}")); }
}

fn fmt_hex(v: u128, bits: u32) -> String {
    format!("0x{:0>width$X}", v, width = (bits as usize + 3) / 4)
}

fn print_timed(label: &str, val: &str, backend: Backend, elapsed: std::time::Duration) {
    let us = elapsed.as_nanos() as f64 / 1000.0;
    if us < 1000.0 { eprintln!("\x1b[90m[{backend} {us:.0}us]\x1b[0m"); }
    else { eprintln!("\x1b[90m[{backend} {:.2}ms]\x1b[0m", us / 1000.0); }
    println!("{label}{val}");
}

// -- GPU field helpers -------------------------------------------------------

/// Convert a u128 tower element to WGSL F2_128 constructor literal.
fn wgsl_f2_128(v: u128) -> String {
    let [w0, w1, w2, w3] = kuro_wgsl::u128_to_u32s(v);
    format!("F2_128({w0}u, {w1}u, {w2}u, {w3}u)")
}

/// WGSL function name for a binary op at the given level.
fn wgsl_binop_name(op: &str, level: u32) -> String {
    match level {
        2   => format!("f2_2_{op}"),
        4   => format!("f2_4_{op}"),
        8   => format!("f2_8_{op}"),
        16  => format!("f2_16_{op}"),
        32  => format!("f2_32_{op}"),
        64  => format!("f2_64_{op}"),
        128 => format!("f2_128_{op}"),
        _   => unreachable!(),
    }
}

/// Build a WGSL literal for a value at the given tower level.
fn wgsl_val(v: u128, level: u32) -> String {
    match level {
        2 | 4 | 8 | 16 | 32 => format!("{}u", v as u32),
        64 => {
            let lo = v as u32;
            let hi = (v >> 32) as u32;
            format!("F2_64({lo}u, {hi}u)")
        }
        128 => wgsl_f2_128(v),
        _ => unreachable!(),
    }
}

/// Number of u32 outputs for a given tower level.
fn level_n_u32s(level: u32) -> usize {
    match level {
        2 | 4 | 8 | 16 | 32 => 1,
        64 => 2,
        128 => 4,
        _ => unreachable!(),
    }
}

/// WGSL code to store a result of the given level into `out[]`.
fn wgsl_store(level: u32) -> &'static str {
    match level {
        2 | 4 | 8 | 16 | 32 => "out[0] = r;",
        64 => "out[0] = r[0]; out[1] = r[1];",
        128 => "out[0] = r[0]; out[1] = r[1]; out[2] = r[2]; out[3] = r[3];",
        _ => unreachable!(),
    }
}

/// Convert u32 results back to u128 based on level.
fn u32s_to_val(results: &[u32], level: u32) -> u128 {
    match level {
        2 | 4 | 8 | 16 | 32 => results[0] as u128,
        64 => (results[0] as u128) | ((results[1] as u128) << 32),
        128 => {
            (results[0] as u128)
                | ((results[1] as u128) << 32)
                | ((results[2] as u128) << 64)
                | ((results[3] as u128) << 96)
        }
        _ => unreachable!(),
    }
}

/// Run a GPU binary op at any tower level.
fn gpu_binop(ctx: &Ctx, op: &str, a: u128, b: u128, level: u32) -> u128 {
    let fname = wgsl_binop_name(op, level);
    let va = wgsl_val(a, level);
    let vb = wgsl_val(b, level);
    let store = wgsl_store(level);
    let body = format!("let r = {fname}({va}, {vb});\n{store}");
    let results = ctx.gpu().run_custom(&body, level_n_u32s(level));
    u32s_to_val(&results, level)
}

/// Run a GPU unary op (square = mul(a,a), frobenius via iterated squaring).
fn gpu_unary(ctx: &Ctx, op: &str, a: u128, level: u32) -> u128 {
    match op {
        "square" => gpu_binop(ctx, "mul", a, a, level),
        _ => {
            // For inv and frobenius, fall back to CPU — these are
            // complex multi-step algorithms not easily expressed as
            // a single WGSL expression at arbitrary levels.
            tower_unary(op, a, level)
        }
    }
}

// -- CPU tower ops -----------------------------------------------------------

fn tower_unary(op: &str, a: u128, level: u32) -> u128 {
    macro_rules! run {
        ($ty:ident, $val:expr) => {{
            let x = $ty($val as _);
            match op {
                "square"    => x.square().0 as u128,
                "inv"       => x.inv().0 as u128,
                "frobenius" => x.frobenius(1).0 as u128,
                _ => unreachable!(),
            }
        }};
    }
    match level {
        2   => run!(F2_2,   a),  4   => run!(F2_4,   a),
        8   => run!(F2_8,   a),  16  => run!(F2_16,  a),
        32  => run!(F2_32,  a),  64  => run!(F2_64,  a),
        128 => run!(F2_128, a),  _   => unreachable!(),
    }
}

fn tower_binop(op: &str, a: u128, b: u128, level: u32) -> u128 {
    macro_rules! run {
        ($ty:ident, $a:expr, $b:expr) => {{
            let (x, y) = ($ty($a as _), $ty($b as _));
            match op { "add" => x.add(y).0 as u128, "mul" => x.mul(y).0 as u128, _ => unreachable!() }
        }};
    }
    match level {
        2   => run!(F2_2,   a, b),  4   => run!(F2_4,   a, b),
        8   => run!(F2_8,   a, b),  16  => run!(F2_16,  a, b),
        32  => run!(F2_32,  a, b),  64  => run!(F2_64,  a, b),
        128 => run!(F2_128, a, b),  _   => unreachable!(),
    }
}

// -- commands ----------------------------------------------------------------

fn cmd_calc(forced: Option<Backend>, args: &[String]) {
    if args.is_empty() { die("usage: kuro calc <op> <args...> [--level N]"); }
    let op = args[0].as_str();
    let rest = &args[1..];
    let level = parse_level(rest);
    let pos = positional(rest);

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();

    match op {
        "add" | "mul" => {
            need(&pos, 2, &format!("{op} <a> <b> [--level N]"));
            let (a, b) = (parse_u128(&pos[0]), parse_u128(&pos[1]));
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_binop(&ctx, op, a, b, level)
            } else {
                tower_binop(op, a, b, level)
            };
            print_timed(&format!("F2^{level} {op}: "), &fmt_hex(r, level), backend, t.elapsed());
        }
        "square" | "inv" | "frobenius" => {
            need(&pos, 1, &format!("{op} <a> [--level N]"));
            let a = parse_u128(&pos[0]);
            if op == "inv" && a == 0 { die("error: inverse of zero is undefined"); }
            let t = Instant::now();
            let r = if backend == Backend::Gpu {
                gpu_unary(&ctx, op, a, level)
            } else {
                tower_unary(op, a, level)
            };
            let actual_backend = if backend == Backend::Gpu && (op == "inv" || op == "frobenius") {
                Backend::Cpu // these fall back to CPU
            } else {
                backend
            };
            print_timed(&format!("F2^{level} {op}: "), &fmt_hex(r, level), actual_backend, t.elapsed());
        }
        other => { die(&format!("unknown calc op: {other}\nops: add mul square inv frobenius")); }
    }
}

fn cmd_encode(args: &[String]) {
    if args.is_empty() { die("usage: kuro encode <hex_bytes>\n  hex_bytes: even-length hex string"); }
    let input = args[0].strip_prefix("0x")
        .or_else(|| args[0].strip_prefix("0X"))
        .unwrap_or(&args[0]);
    if input.len() % 2 != 0 { die("error: hex string must have even length"); }
    let bytes: Vec<u8> = (0..input.len()).step_by(2).map(|i| {
        u8::from_str_radix(&input[i..i + 2], 16)
            .unwrap_or_else(|e| die(&format!("invalid hex byte '{}': {e}", &input[i..i + 2])))
    }).collect();

    println!("input: {} ({} bytes)", input.to_lowercase(), bytes.len());
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let mut val: u128 = 0;
        for (j, &b) in chunk.iter().enumerate() { val |= (b as u128) << (j * 8); }
        println!("  tower[{i}]: 0x{:032X}  ({} bytes packed)", val, chunk.len());
    }
}

fn cmd_decode(args: &[String]) {
    if args.is_empty() { die("usage: kuro decode <hex_element>"); }
    let val = parse_u128(&args[0]);
    let mut bytes = [0u8; 16];
    let mut v = val;
    for b in bytes.iter_mut() { *b = (v & 0xFF) as u8; v >>= 8; }
    let len = bytes.iter().rposition(|&b| b != 0).map_or(1, |i| i + 1);
    let hex: String = bytes[..len].iter().map(|b| format!("{b:02x}")).collect();
    println!("element: 0x{:032X}", val);
    println!("  bytes: {hex} ({len} bytes)");
}

fn cmd_bench(forced: Option<Backend>, args: &[String]) {
    let iters: u64 = if args.is_empty() { 100_000 } else {
        args[0].parse().unwrap_or_else(|e| die(&format!("invalid iteration count '{}': {e}", args[0])))
    };

    let ctx = Ctx::new(forced);
    let backend = ctx.backend();
    eprintln!("\x1b[90m[backend: {backend}]\x1b[0m");

    println!("kuro bench -- {iters} iterations per operation\n");

    let a128 = F2_128(0xDEAD_BEEF_CAFE_BABE_1234_5678_9ABC_DEF0);
    let b128 = F2_128(0x0123_4567_89AB_CDEF_FEDC_BA98_7654_3210);
    let a128v = a128.0;
    let b128v = b128.0;

    if backend == Backend::Gpu {
        bench_op("F2^128 add   (gpu)", iters, || {
            black_box(gpu_binop(&ctx, "add", black_box(a128v), black_box(b128v), 128));
        });
        bench_op("F2^128 mul   (gpu)", iters, || {
            black_box(gpu_binop(&ctx, "mul", black_box(a128v), black_box(b128v), 128));
        });
        bench_op("F2^128 square(gpu)", iters, || {
            black_box(gpu_binop(&ctx, "mul", black_box(a128v), black_box(a128v), 128));
        });
    }

    bench_op("F2^128 add   (cpu)", iters, || black_box(black_box(a128).add(black_box(b128))));
    bench_op("F2^128 mul   (cpu)", iters, || black_box(black_box(a128).mul(black_box(b128))));
    bench_op("F2^128 square(cpu)", iters, || black_box(black_box(a128).square()));
    bench_op("F2^128 inv   (cpu)", iters, || black_box(black_box(a128).inv()));
    bench_op("F2^128 frob  (cpu)", iters, || black_box(black_box(a128).frobenius(1)));
    println!();

    let a64 = F2_64(0xDEAD_BEEF_CAFE_BABE);
    let b64 = F2_64(0x0123_4567_89AB_CDEF);
    let a64v = a64.0;
    let b64v = b64.0;

    if backend == Backend::Gpu {
        bench_op("F2^64  add   (gpu)", iters, || {
            black_box(gpu_binop(&ctx, "add", black_box(a64v as u128), black_box(b64v as u128), 64));
        });
        bench_op("F2^64  mul   (gpu)", iters, || {
            black_box(gpu_binop(&ctx, "mul", black_box(a64v as u128), black_box(b64v as u128), 64));
        });
    }

    bench_op("F2^64  add   (cpu)", iters, || black_box(black_box(a64).add(black_box(b64))));
    bench_op("F2^64  mul   (cpu)", iters, || black_box(black_box(a64).mul(black_box(b64))));
    bench_op("F2^64  inv   (cpu)", iters, || black_box(black_box(a64).inv()));
    println!();

    let a32 = F2_32(0xDEAD_BEEF);
    let b32 = F2_32(0x0123_4567);
    bench_op("F2^32  add   (cpu)", iters, || black_box(black_box(a32).add(black_box(b32))));
    bench_op("F2^32  mul   (cpu)", iters, || black_box(black_box(a32).mul(black_box(b32))));
    bench_op("F2^32  inv   (cpu)", iters, || black_box(black_box(a32).inv()));
    println!();

    let pa = Packed128(0xFFFF_FFFF_0000_0000_AAAA_AAAA_5555_5555);
    let pb = Packed128(0x0F0F_0F0F_F0F0_F0F0_3333_3333_CCCC_CCCC);
    let pav = pa.0;
    let pbv = pb.0;

    if backend == Backend::Gpu {
        bench_op("Packed ip    (gpu)", iters, || {
            black_box(ctx.gpu().run_packed_inner_product(black_box(pav), black_box(pbv)));
        });
    }

    bench_op("Packed add   (cpu)", iters, || black_box(black_box(pa).add(black_box(pb))));
    bench_op("Packed mul   (cpu)", iters, || black_box(black_box(pa).mul(black_box(pb))));
    bench_op("Packed pop   (cpu)", iters, || black_box(black_box(pa).popcount()));
}

fn bench_op<F: Fn() -> T, T>(label: &str, iters: u64, f: F) {
    let t = Instant::now();
    for _ in 0..iters { f(); }
    let ns = t.elapsed().as_nanos() as f64 / iters as f64;
    println!("  {label}  {ns:>8.1} ns/op");
}
