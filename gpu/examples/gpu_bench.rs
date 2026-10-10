//! Throughput of each backend: hemera permutations per second (batch of
//! 2^20 states) and grinding to `bits` (default 24) on the hash40 and lane
//! injections; each GPU result checked against the CPU backend.
//!
//! `cargo run --release -p cyber-lens-gpu --features metal,webgpu --example gpu_bench [-- bits]`

use std::time::Instant;

use cyber_lens_rspcs::backend::{Backend, Cpu, Inject, State, hash40_template};

fn main() {
    let bits: u32 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(24);
    let mut all: Vec<cyber_lens_gpu::Selected> =
        vec![cyber_lens_gpu::select(cyber_lens_gpu::Choice::Cpu).expect("cpu")];
    for b in [cyber_lens_gpu::metal(), cyber_lens_gpu::webgpu()] {
        match b {
            Ok(s) => all.push(s),
            Err(e) => eprintln!("{e}"),
        }
    }
    let n = 1usize << 20;
    let input: Vec<State> = (0..n as u64)
        .map(|k| core::array::from_fn(|i| k * 16 + i as u64))
        .collect();
    let mut want = input.clone();
    Cpu.permute(&mut want);
    let seed = [42u8; 32];
    let t = hash40_template(&seed);
    let want_nonce = Cpu.grind(&t, Inject::hash40(), bits);
    println!(
        "| backend | device | permute 2^20 (ms) | Mperm/s | grind {bits} bits (ms) | nonce | equal to cpu |"
    );
    println!("|---|---|---|---|---|---|---|");
    for s in &all {
        let b = &s.backend;
        let mut st = input.clone();
        b.permute(&mut st); // warm-up (pipeline, buffers)
        let mut st = input.clone();
        let t0 = Instant::now();
        b.permute(&mut st);
        let pm = t0.elapsed().as_secs_f64();
        let t1 = Instant::now();
        let nonce = b.grind(&t, Inject::hash40(), bits);
        let gm = t1.elapsed().as_secs_f64();
        println!(
            "| {} | {} | {:.1} | {:.1} | {:.0} | {} | {} |",
            b.name(),
            s.device,
            pm * 1e3,
            n as f64 / pm / 1e6,
            gm * 1e3,
            nonce,
            st == want && nonce == want_nonce
        );
    }
}
