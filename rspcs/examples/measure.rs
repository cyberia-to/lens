//! Proof size, prove time, verify time and proven security for TensorRs
//! and WHIR. Release build only:
//!
//! ```text
//! cargo run --release -p cyber-lens-rspcs --example measure [-- runs]
//! ```
//!
//! Times are medians over `runs` (default 5) independent runs with fixed
//! seeds; prove = commit + open, reported with both parts.

use std::time::Instant;

use cyber_lens_core::{MultilinearPcs, Transcript};
use cyber_lens_rspcs::{TensorConfig, TensorRs, TensorRsParams, Whir, WhirConfig, WhirParams};
use nebu::field::P;
use nebu::{Fp3, Goldilocks};

fn rng(seed: u64) -> impl FnMut() -> u64 {
    let mut s = seed ^ 0x9E37_79B9_7F4A_7C15;
    move || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

struct Row {
    commit_ms: f64,
    open_ms: f64,
    verify_ms: f64,
    bytes: usize,
}

fn measure<S: MultilinearPcs>(
    p: &S::Params,
    m: usize,
    runs: usize,
    size: impl Fn(&S::Proof) -> usize,
) -> Row {
    let (mut c, mut o, mut v, mut b) = (vec![], vec![], vec![], vec![]);
    for run in 0..runs {
        let mut r = rng(1000 * m as u64 + run as u64);
        let evals: Vec<Goldilocks> = (0..1usize << m).map(|_| Goldilocks::new(r() % P)).collect();
        let point: Vec<Fp3> = (0..m)
            .map(|_| {
                Fp3::new(
                    Goldilocks::new(r() % P),
                    Goldilocks::new(r() % P),
                    Goldilocks::new(r() % P),
                )
            })
            .collect();
        let t0 = Instant::now();
        let (com, data) = S::commit(p, &evals);
        let t1 = Instant::now();
        let (value, proof) = S::open(p, &data, &point, &mut Transcript::new(b"measure"));
        let t2 = Instant::now();
        S::verify(
            p,
            &com,
            m,
            &point,
            value,
            &proof,
            &mut Transcript::new(b"measure"),
        )
        .expect("verifies");
        let t3 = Instant::now();
        c.push((t1 - t0).as_secs_f64() * 1e3);
        o.push((t2 - t1).as_secs_f64() * 1e3);
        v.push((t3 - t2).as_secs_f64() * 1e3);
        b.push(size(&proof) as f64);
    }
    Row {
        commit_ms: median(c),
        open_ms: median(o),
        verify_ms: median(v),
        bytes: median(b) as usize,
    }
}

fn main() {
    let runs: usize = std::env::args()
        .nth(1)
        .map_or(5, |s| s.parse().expect("runs"));
    let (a, b) = (cyber_hemera::hash(b"a"), cyber_hemera::hash(b"b"));
    let perm: Vec<f64> = (0..runs)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..100_000 {
                std::hint::black_box(cyber_hemera::tree::hash_node(&a, &b, false));
            }
            t.elapsed().as_secs_f64() * 1e6 / 1e5
        })
        .collect();
    println!(
        "hemera hash_node (one permutation): {:.3} us (median of {runs} × 100k)\n",
        median(perm)
    );
    println!(
        "| scheme | num_vars | rate | shape / schedule | proof bytes | prove ms (commit + open) | verify ms | security bits | 128-bit proven |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for m in [10usize, 12, 16, 20] {
        for r in [1u8, 2, 4] {
            let tp = TensorRsParams {
                log_inv_rate: r,
                ..TensorRsParams::default()
            };
            let tc = TensorConfig::derive(&tp, m).expect("config");
            let row = measure::<TensorRs>(&tp, m, runs, |p| p.to_bytes().len());
            let sec = TensorRs::security_bits(&tp, m);
            println!(
                "| TensorRs | {m} | 1/{} | k1=2^{} k2=2^{} N=2^{} t={}{} | {} | {:.2} ({:.2} + {:.2}) | {:.3} | {:.2} | {} |",
                1 << r,
                tc.row_vars,
                tc.col_vars,
                tc.log_n,
                tc.queries,
                if tc.opens_all() { " (all)" } else { "" },
                row.bytes,
                row.commit_ms + row.open_ms,
                row.commit_ms,
                row.open_ms,
                row.verify_ms,
                sec,
                if sec >= 128.0 { "yes" } else { "no" }
            );
            let wp = WhirParams {
                log_inv_rate: r,
                ..WhirParams::default()
            };
            let wc = WhirConfig::derive(&wp, m).expect("config");
            let sched: Vec<String> = wc
                .rounds
                .iter()
                .map(|rd| {
                    format!(
                        "t={}{}",
                        rd.queries,
                        if rd.opens_all() { "(all)" } else { "" }
                    )
                })
                .collect();
            let row = measure::<Whir>(&wp, m, runs, |p| p.to_bytes().len());
            let sec = Whir::security_bits(&wp, m);
            println!(
                "| WHIR | {m} | 1/{} | k=4, {} rounds [{}], final 2^{} | {} | {:.2} ({:.2} + {:.2}) | {:.3} | {:.2} | {} |",
                1 << r,
                wc.rounds.len(),
                sched.join(" "),
                wc.final_vars,
                row.bytes,
                row.commit_ms + row.open_ms,
                row.commit_ms,
                row.open_ms,
                row.verify_ms,
                sec,
                if sec >= 128.0 { "yes" } else { "no" }
            );
        }
    }
}
