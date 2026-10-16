//! Hemera throughput per backend: permutations per second over 2^20
//! independent states and header mining (four permutations per nonce,
//! target zero so every nonce of the dispatch is tried), Metal at several
//! states per thread. Every row is checked against the CPU backend; the
//! table gives the median and the best of `reps` runs.
//!
//! `cargo run --release -p cyber-lens-gpu --features metal,webgpu --example hemera_bench [-- reps [metal_ks]]`
//! (`metal_ks` comma-separated, default `1,2,4,8`)

use std::time::Instant;

use cyber_lens_gpu::mine::{self, Job};
use cyber_lens_rspcs::backend::{Backend, Cpu, State};

const P: u64 = 0xFFFF_FFFF_0000_0001;

fn stats(mut v: Vec<f64>) -> (f64, f64) {
    v.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    (v[v.len() / 2], v[v.len() - 1])
}

fn timed(reps: usize, mut f: impl FnMut()) -> Vec<f64> {
    f(); // warm-up (pipelines, buffers)
    (0..reps)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64()
        })
        .collect()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let reps: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(7);
    let ks: Vec<usize> = args
        .next()
        .unwrap_or_else(|| "1,2,4,8".into())
        .split(',')
        .map(|k| k.parse().expect("k"))
        .collect();

    let n = 1usize << 20;
    let mut r = 0x9e37_79b9_7f4a_7c15u64;
    let input: Vec<State> = (0..n)
        .map(|k| {
            core::array::from_fn(|i| {
                r ^= r << 13;
                r ^= r >> 7;
                r ^= r << 17;
                if (k + i) % 11 == 0 { 0 } else { r % P }
            })
        })
        .collect();
    let mut want = input.clone();
    Cpu.permute(&mut want);

    let mut blocks = [[0u64; 8]; 4];
    for (b, block) in blocks.iter_mut().enumerate() {
        for (i, l) in block.iter_mut().enumerate() {
            *l = (0x0123_4567_89ab_cdefu64.wrapping_mul((8 * b + i + 1) as u64)) >> 8;
        }
    }
    blocks[0][3] &= 0x00ff_ffff;
    blocks[0][4] = 0;
    blocks[0][5] &= !0xff_ffff_ffff;
    let job = Job {
        init: [0; 16],
        blocks,
        tail_lane: 10,
        tail: 168,
        target: [0; 4],
    };
    let mine_n: u32 = 1 << 22;
    let cpu_mine_n: u32 = 1 << 18;

    println!(
        "| backend | permute 2^20: Mperm/s median | best | mine: MH/s median | best | Mperm/s (mine) | equal to cpu |"
    );
    println!("|---|---|---|---|---|---|---|");

    let row = |name: String,
               permute: &dyn Fn(&mut [State]),
               mine_fn: &dyn Fn(u128, u32) -> Option<u32>,
               count: u32| {
        let mut ok = true;
        let mut st = input.clone();
        permute(&mut st);
        ok &= st == want;
        let pt = timed(reps, || {
            let mut st = input.clone();
            let t = Instant::now();
            permute(&mut st);
            std::hint::black_box((t, &st));
        });
        // `timed` includes the clone; time the clone alone and subtract it
        let ct = timed(reps, || {
            std::hint::black_box(input.clone());
        });
        let (cm, _) = stats(ct);
        let pt: Vec<f64> = pt.into_iter().map(|t| n as f64 / (t - cm).max(1e-9) / 1e6).collect();
        let (pm, pb) = stats(pt);
        let mt = timed(reps, || {
            std::hint::black_box(mine_fn(1 << 40, count));
        });
        let mt: Vec<f64> = mt.into_iter().map(|t| f64::from(count) / t / 1e6).collect();
        let (mm, mb) = stats(mt);
        println!(
            "| {name} | {pm:.1} | {pb:.1} | {mm:.2} | {mb:.2} | {:.1} | {ok} |",
            4.0 * mm
        );
    };

    row(
        "cpu (all cores)".into(),
        &|s| Cpu.permute(s),
        &|b, c| mine::cpu(&job, b, c),
        cpu_mine_n,
    );

    #[cfg(target_os = "macos")]
    for &k in &ks {
        let m = cyber_lens_gpu::metal::Metal::with_states_per_thread(k).expect("metal");
        // agreement on a reachable target first
        let mut easy = job;
        easy.target = [0, 0, 0, 1 << 52];
        let got = m.mine(&easy, 7, 1 << 16).expect("mine");
        assert_eq!(got, mine::cpu(&easy, 7, 1 << 16), "metal k={k} mine");
        row(
            format!("metal k={k}"),
            &|s| m.permute(s),
            &|b, c| m.mine(&job, b, c).expect("mine"),
            mine_n,
        );
    }
    if let Ok(w) = cyber_lens_gpu::webgpu::WebGpu::new() {
        let mut easy = job;
        easy.target = [0, 0, 0, 1 << 52];
        let got = w.mine(&easy, 7, 1 << 16).expect("mine");
        assert_eq!(got, mine::cpu(&easy, 7, 1 << 16), "webgpu mine");
        row(
            "webgpu".into(),
            &|s| w.permute(s),
            &|b, c| w.mine(&job, b, c).expect("mine"),
            mine_n,
        );
    }
}
