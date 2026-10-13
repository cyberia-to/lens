//! Line sessions under concurrency: several threads run sessions and
//! permutations on one backend at once; every result against the CPU.
use cyber_lens_rspcs::backend::{Backend, Cpu, CpuSession, LineSession};
use cyber_lens_rspcs::program::{OP_MULB, Program};
const P: u64 = 0xFFFF_FFFF_0000_0001;
fn rng(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}
fn main() {
    let name = std::env::args().nth(1).unwrap_or("webgpu".into());
    let b = if name == "metal" {
        cyber_lens_gpu::metal()
    } else {
        cyber_lens_gpu::webgpu()
    }
    .expect("backend")
    .backend;
    let b = &*b;
    std::thread::scope(|sc| {
        for t in 0..6u64 {
            sc.spawn(move || {
                let mut r = 1000 + t;
                if t % 2 == 1 {
                    for round in 0..20 {
                        let mut st: Vec<[u64; 16]> = (0..8192)
                            .map(|_| core::array::from_fn(|_| rng(&mut r) % P))
                            .collect();
                        let mut want = st.clone();
                        Cpu.permute(&mut want);
                        b.permute(&mut st);
                        assert!(st == want, "permute thread {t} round {round}");
                    }
                    return;
                }
                let inputs = 40usize;
                let consts: Vec<[u64; 3]> = (0..3)
                    .map(|_| [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P])
                    .chain([[5, 0, 0]])
                    .collect();
                let base = (inputs + consts.len()) as u32;
                let ops: Vec<(u8, u32, u32)> = (0..300u32)
                    .map(|k| {
                        let code = (rng(&mut r) % 5) as u8;
                        let a = (rng(&mut r) % (base + k) as u64) as u32;
                        let bb = if code == OP_MULB {
                            (consts.len() - 1) as u32
                        } else {
                            (rng(&mut r) % (base + k) as u64) as u32
                        };
                        (code, a, bb)
                    })
                    .collect();
                let prog = Program::from_slots(&ops, consts, inputs, base + 299);
                let len = 1usize << 15;
                let cols: Vec<[u64; 3]> = (0..inputs * len)
                    .map(|_| [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P])
                    .collect();
                let mut dev = b.line_session(&prog, &cols, len).expect("session");
                let mut cpu = CpuSession::new(&prog, &cols, len);
                let mut rows = len;
                let mut round = 0;
                while rows >= 2 {
                    let w: Vec<[u64; 3]> = (0..rows / 2)
                        .map(|_| [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P])
                        .collect();
                    let xs = [0u64, 2, 3, 4, 5, 6, 7, 8];
                    let (d, c) = (dev.sums(&w, &xs), cpu.sums(&w, &xs));
                    if d != c {
                        println!("MISMATCH thread {t} round {round} rows {rows}");
                    }
                    let a = [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P];
                    dev.fold(a);
                    cpu.fold(a);
                    rows /= 2;
                    round += 1;
                }
                if dev.firsts() != cpu.firsts() {
                    println!("MISMATCH firsts thread {t}");
                }
                println!("thread {t} done");
            });
        }
    });
}
