//! Every GPU backend against the CPU backend on random inputs: the same
//! permutation outputs (canonical limbs), the same least grinding nonce for
//! both nonce injections. A backend that is not built or has no device is
//! reported and skipped.

use cyber_lens_rspcs::backend::{Backend, Cpu, Inject, State, hash40_template};

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// hemera 0.4's permutation vectors (`vectors/hemera.json` of cyber-hemera
/// 0.4.0, file sha-256 5b88d88a…dda5f): name, input, output (canonical limbs).
#[rustfmt::skip]
const HEMERA_PERMUTATION: [(&str, [u64; 16], [u64; 16]); 4] = [
    ("zeros",
     [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000],
     [0x895cbbb878ed7b95, 0x4d9447e9b10d0a16, 0xeda4b52a1341bdfa, 0xecb0dde11b61b8f7, 0x882a552953fff61a, 0x301b343adb1d0f0d, 0xd3ca66a578b75857, 0x3be5ae56adc25b69, 0x3546e64a96c3aae1, 0xd68bd20423ae7095, 0x5eb69c0eb8b3397e, 0x40f3c6e8e6485294, 0xc90ce42c6d96dadf, 0x6ce65e1d2915ab36, 0xc6f8b4499590fb1e, 0xbe3974adef258033]),
    ("iota",
     [0x0000000000000000, 0x0000000000000001, 0x0000000000000002, 0x0000000000000003, 0x0000000000000004, 0x0000000000000005, 0x0000000000000006, 0x0000000000000007, 0x0000000000000008, 0x0000000000000009, 0x000000000000000a, 0x000000000000000b, 0x000000000000000c, 0x000000000000000d, 0x000000000000000e, 0x000000000000000f],
     [0x566c0cbb8e4baf40, 0x70bff2279d137b8d, 0xa8302c88173a9a23, 0x0dcfb7edffeaaf54, 0x3f623d7a8a5542ec, 0xf8d28758d3844017, 0x76c90e979516637d, 0xb0cce7d290e07756, 0xf000a79b1b3140d8, 0xa65c85e8d16bf499, 0x4ada4e8695b9af8d, 0x27bbbb5bf6f0777a, 0xac65b622281938a7, 0x54a28ec8783cd3ef, 0x88603f130241a3c3, 0x94dd7cec2bfb78b0]),
    ("p_minus_1",
     [0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000, 0xffffffff00000000],
     [0xb1e504b21678577a, 0x2cd37d8529eb67b9, 0xa949d6a8f4f2cf6c, 0x8b8d75c1d4a7a5a6, 0x64df9fb5f668aaa2, 0x4bbf169465a16dd9, 0xbfe3788762e3b834, 0x067638f8712f4d81, 0x83b2e259cdfcb4c3, 0xb120313f3125503b, 0x909811c4b91ada15, 0x22db33308c8de273, 0xf4541036a8280da0, 0x9659da63e426bdf7, 0xc4deaaa64131060a, 0x832c00eaf0bac9a7]),
    ("random",
     [0x856d9c28a9741a02, 0x86ad14d4430c32b6, 0xc366ae2c20764753, 0xce6336ec93771f1d, 0x1a79312058b39663, 0x3ca10c145535098f, 0x9e66e2665154e05c, 0xba96d597e9fdde9c, 0xa11665ed06c80221, 0x480c4fbc1a4b3a65, 0xb6a11497bd5f9551, 0x9cefea2e8b1c5e3b, 0xbf1e5a8e8a548847, 0x5d0032063244d097, 0xb0ba1fab0cea9cf6, 0x333d875f5b54b44f],
     [0xa6341eaea32413a5, 0x6011492b4a4849f5, 0xb7e83d614f05d91e, 0xb7806f1febb839e0, 0x7df4f5b00e17bfe6, 0xc25e2842099867bc, 0xb3fec268c09f4e06, 0xb5cf509e1511e4ba, 0xae2ede919fa51078, 0xda1c4befc75481ea, 0x524a2068acca0aba, 0xca9d616605b9b291, 0xf344c679905d27c9, 0xd7dd17a93f54a115, 0x2bbe8d9a5d07326f, 0x748c9cc68978afd2]),
];

fn rng(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn backends() -> Vec<cyber_lens_gpu::Selected> {
    let mut v = Vec::new();
    for (name, b) in [
        ("metal", cyber_lens_gpu::metal()),
        ("webgpu", cyber_lens_gpu::webgpu()),
    ] {
        match b {
            Ok(s) => v.push(s),
            Err(e) => {
                // LENS_GPU_REQUIRE=metal,webgpu turns a missing backend into a failure
                let required = std::env::var("LENS_GPU_REQUIRE").unwrap_or_default();
                assert!(
                    !required.split(',').any(|r| r == name),
                    "{name} required: {e}"
                );
                eprintln!("{name}: skipped ({e})")
            }
        }
    }
    v
}

/// A required backend must run the operation (no silent CPU fallback).
fn required(name: &str) -> bool {
    std::env::var("LENS_GPU_REQUIRE")
        .unwrap_or_default()
        .split(',')
        .any(|r| r == name)
}

fn random_states(n: usize, seed: u64) -> Vec<State> {
    let mut r = seed;
    (0..n)
        .map(|k| {
            core::array::from_fn(|i| match (k + i) % 7 {
                0 => 0,
                1 => P - 1,
                2 => rng(&mut r) % 4,
                _ => rng(&mut r) % P,
            })
        })
        .collect()
}

#[test]
fn permutations_equal_cpu() {
    for s in backends() {
        for n in [4096usize, 5000, 70_001] {
            let input = random_states(n, n as u64 * 31 + 1);
            let mut want = input.clone();
            Cpu.permute(&mut want);
            let mut got = input.clone();
            s.backend.permute(&mut got);
            let bad = got.iter().zip(&want).position(|(a, b)| a != b);
            assert_eq!(
                bad,
                None,
                "{} ({}) n={n}: first differing state",
                s.backend.name(),
                s.device
            );
        }
    }
}

/// Edge states against hemera's own single permutation (not the batched
/// CPU backend): zeros, p − 1 everywhere, unit vectors, alternating 0 and
/// p − 1, small values; each repeated so every lane of a GPU thread sees it.
#[test]
fn edge_states_equal_hemera() {
    let mut edges: Vec<State> = vec![[0; 16], [P - 1; 16], [1; 16], [2; 16]];
    for j in 0..16 {
        let mut e = [0u64; 16];
        e[j] = 1;
        edges.push(e);
        let mut e = [0u64; 16];
        e[j] = P - 1;
        edges.push(e);
    }
    edges.push(core::array::from_fn(|i| if i % 2 == 0 { 0 } else { P - 1 }));
    edges.push(core::array::from_fn(|i| i as u64));
    // the published vectors pin the CPU permutation this suite compares to
    for (name, input, output) in HEMERA_PERMUTATION {
        let mut h = input.map(cyber_hemera::field::Goldilocks::new);
        cyber_hemera::permutation::permute(&mut h);
        assert_eq!(
            h.map(|x| x.as_canonical_u64()),
            output,
            "hemera 0.4 vector {name}"
        );
        edges.push(input);
    }
    let want: Vec<State> = edges
        .iter()
        .map(|s| {
            let mut h = s.map(cyber_hemera::field::Goldilocks::new);
            cyber_hemera::permutation::permute(&mut h);
            h.map(|x| x.as_canonical_u64())
        })
        .collect();
    for s in backends() {
        for reps in [1usize, 4099] {
            let input: Vec<State> = (0..reps * edges.len())
                .map(|k| edges[k % edges.len()])
                .collect();
            let mut got = input.clone();
            s.backend.permute(&mut got);
            for (k, g) in got.iter().enumerate() {
                assert_eq!(
                    g,
                    &want[k % edges.len()],
                    "{} edge {} (copy {})",
                    s.backend.name(),
                    k % edges.len(),
                    k / edges.len()
                );
            }
        }
    }
}

#[test]
fn grinding_finds_the_cpu_nonce() {
    let mut r = 0x5eed_u64;
    for s in backends() {
        for round in 0..12 {
            let bits = 1 + (round % 18) as u32;
            let template: State = core::array::from_fn(|_| rng(&mut r) % P);
            let lane = (rng(&mut r) % 9) as usize;
            let mut t = template;
            t[lane] = 0;
            let want = Cpu.grind(&t, Inject::lane(lane), bits);
            let got = s.backend.grind(&t, Inject::lane(lane), bits);
            assert_eq!(got, want, "{} lane {lane} bits {bits}", s.backend.name());

            let mut seed = [0u8; 32];
            for b in seed.iter_mut() {
                *b = rng(&mut r) as u8;
            }
            let t = hash40_template(&seed);
            let want = Cpu.grind(&t, Inject::hash40(), bits);
            let got = s.backend.grind(&t, Inject::hash40(), bits);
            assert_eq!(got, want, "{} hash40 bits {bits}", s.backend.name());
        }
    }
}

#[test]
fn grinding_past_the_first_dispatch() {
    // dispatches start at 2^bits and double: over 32 seeds at 12 and 20
    // bits the least nonce lies beyond the first dispatch in about a third
    // of the cases (several dispatches); every backend returns the least
    for bits in [12u32, 20] {
        let seeds = if bits == 12 { 32u8 } else { 4 };
        for seed in 0..seeds {
            let t = hash40_template(&[seed.wrapping_mul(29).wrapping_add(7); 32]);
            let want = Cpu.grind(&t, Inject::hash40(), bits);
            for s in backends() {
                assert_eq!(
                    s.backend.grind(&t, Inject::hash40(), bits),
                    want,
                    "{} bits {bits} seed {seed} (nonce {want})",
                    s.backend.name()
                );
            }
        }
    }
}

#[cfg(all(feature = "metal", target_os = "macos", target_arch = "aarch64"))]
#[test]
fn metal_states_per_thread_equal_cpu() {
    // several independent states per GPU thread, interleaved round by
    // round; a ragged tail (n not a multiple of k) and zero lanes included
    for k in [1usize, 2, 3, 4, 8] {
        let Ok(m) = cyber_lens_gpu::metal::Metal::with_states_per_thread(k) else {
            assert!(
                std::env::var("LENS_GPU_REQUIRE")
                    .unwrap_or_default()
                    .split(',')
                    .all(|r| r != "metal"),
                "metal required"
            );
            eprintln!("metal: skipped");
            return;
        };
        let mut input = random_states(4099, k as u64);
        input[5] = [0; 16];
        let mut want = input.clone();
        Cpu.permute(&mut want);
        let mut got = input;
        m.permute(&mut got);
        assert_eq!(got, want, "k={k}");
        let t = hash40_template(&[k as u8; 32]);
        assert_eq!(
            m.grind(&t, Inject::hash40(), 12),
            Cpu.grind(&t, Inject::hash40(), 12),
            "k={k}"
        );
        // header mining at k states a thread: the CPU's nonce, and it
        // verifies with hemera's hash of the header
        let mut msg = [0u8; 168];
        for (i, b) in msg.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(13).wrapping_add(k as u8);
        }
        let target = [0, 0, 0, 1u64 << 53];
        let job = mine_job(&msg, target);
        let base = 3 + ((k as u128) << 70);
        let got = m.mine(&job, base, 1 << 15).expect("mine");
        assert_eq!(
            got,
            cyber_lens_gpu::mine::cpu(&job, base, 1 << 15),
            "k={k} mine"
        );
        let n = base + u128::from(got.expect("a nonce in 2^15 at 2^245"));
        msg[24..40].copy_from_slice(&n.to_le_bytes());
        let h = cyber_hemera::hash(&msg);
        let d: [u64; 4] = core::array::from_fn(|i| {
            u64::from_le_bytes(h.as_bytes()[8 * i..8 * i + 8].try_into().expect("8 bytes"))
        });
        assert!(
            cyber_lens_gpu::mine::below(&d, &target),
            "k={k}: hemera::hash of the mined header"
        );
    }
}

#[test]
fn coset_ntts_equal_cpu() {
    let mut r = 99u64;
    for s in backends() {
        for log_n in [12u32, 13, 16] {
            let coeffs: Vec<u64> = (0..1u64 << log_n).map(|_| rng(&mut r) % P).collect();
            let shifts: Vec<u64> = (0..4).map(|_| rng(&mut r) % P).chain([1]).collect();
            assert_eq!(
                s.backend.coset_ntt(&coeffs, &shifts),
                Cpu.coset_ntt(&coeffs, &shifts),
                "{} log_n {log_n}",
                s.backend.name()
            );
        }
    }
}

#[test]
fn sponges_equal_cpu() {
    let mut r = 1234u64;
    for s in backends() {
        for (rate, blocks, items) in [(9usize, 1usize, 4096usize), (9, 6, 5000), (8, 3, 4097)] {
            let init: State = core::array::from_fn(|i| if i == rate { 2 } else { rng(&mut r) % 3 });
            let lanes: Vec<u64> = (0..items * rate * blocks)
                .map(|_| rng(&mut r) % P)
                .collect();
            assert_eq!(
                s.backend.sponge(&init, rate, blocks, &lanes),
                Cpu.sponge(&init, rate, blocks, &lanes),
                "{} rate {rate} blocks {blocks}",
                s.backend.name()
            );
        }
    }
}

#[test]
fn program_line_sums_equal_cpu() {
    use cyber_lens_rspcs::program::{OP_MULB, Program, from_fp3};
    let mut r = 4242u64;
    for s in backends() {
        let inputs = 23usize;
        let consts: Vec<[u64; 3]> = (0..6)
            .map(|_| [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P])
            .chain([[9, 0, 0]])
            .collect();
        let base = (inputs + consts.len()) as u32;
        let mut ops = Vec::new();
        for k in 0..400u32 {
            let avail = base + k;
            let code = (rng(&mut r) % 5) as u8;
            let a = (rng(&mut r) % avail as u64) as u32;
            let b = if code == OP_MULB {
                (consts.len() - 1) as u32
            } else {
                (rng(&mut r) % avail as u64) as u32
            };
            ops.push((code, a, b));
        }
        let prog = Program::from_slots(&ops, consts, inputs, base + 399);
        let pairs = 3000usize;
        // random limbs, and runs of edge limbs (p − 1, p − 2^32, 0, 1):
        // the Fp3 product's wide sums at their largest
        let edge = [P - 1, P - (1 << 32), 0, 1, P - 1, P - 1];
        let cols: Vec<[u64; 3]> = (0..inputs * 2 * pairs)
            .map(|j| {
                if j % 7 < 3 {
                    [edge[j % 6], edge[(j + 1) % 6], edge[(j + 4) % 6]]
                } else {
                    [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P]
                }
            })
            .collect();
        let mut weights: Vec<[u64; 3]> = (0..pairs)
            .map(|_| [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P])
            .collect();
        weights[7] = [0, 0, 0];
        let xs = [0u64, 2, 3, 4, 5, 6, 7];
        let want = Cpu.line_sums(&prog, &cols, &weights, &xs);
        assert_eq!(
            s.backend.line_sums(&prog, &cols, &weights, &xs),
            want,
            "{} regs {}",
            s.backend.name(),
            prog.regs
        );
        let _ = from_fp3;
    }
}

#[test]
fn coset_leaves_equal_the_reference() {
    use cyber_lens_rspcs::backend::{CosetLeaves, coset_leaves_reference, coset_values_reference};
    let mut r = 77u64;
    for s in backends() {
        for (members, limbs, log_k, log_n, log_w) in [
            (2usize, 1usize, 12u32, 16u32, 3u32),
            (2, 3, 12, 16, 4),
            (1, 3, 13, 16, 2),
        ] {
            let k = 1usize << log_k;
            let data: Vec<Vec<u64>> = (0..members * limbs)
                .map(|_| (0..k).map(|_| rng(&mut r) % P).collect())
                .collect();
            let mut init = [0u64; 16];
            init[9] = 2;
            let job = CosetLeaves {
                limbs: data.iter().map(|v| v.as_slice()).collect(),
                limbs_per_member: limbs,
                log_n,
                log_width: log_w,
                init,
                rate: 9,
            };
            let cosets = job.cosets();
            for range in [0..cosets, 3..5] {
                let mut vals = vec![0u64; job.limbs.len() * range.len() * k];
                match s.backend.coset_leaves_kept(&job, range.clone(), &mut vals) {
                    Some(got) => {
                        assert_eq!(
                            got,
                            coset_leaves_reference(&job, range.clone()),
                            "{} kept leaves {range:?}",
                            s.backend.name()
                        );
                        assert!(
                            vals == coset_values_reference(&job, range.clone()),
                            "{} kept values {range:?}",
                            s.backend.name()
                        );
                    }
                    None => assert!(
                        !required(s.backend.name()),
                        "{}: kept coset leaves declined",
                        s.backend.name()
                    ),
                }
                match s.backend.coset_leaves(&job, range.clone()) {
                    Some(got) => assert_eq!(
                        got,
                        coset_leaves_reference(&job, range.clone()),
                        "{} members {members} limbs {limbs} range {range:?}",
                        s.backend.name()
                    ),
                    None => assert!(
                        !required(s.backend.name()),
                        "{}: coset leaves declined",
                        s.backend.name()
                    ),
                }
            }
        }
    }
}

#[test]
fn line_sessions_equal_the_cpu_session() {
    use cyber_lens_rspcs::backend::{CpuSession, LineSession};
    use cyber_lens_rspcs::program::{OP_MULB, Program};
    let mut r = 999u64;
    for s in backends() {
        let inputs = 11usize;
        let consts: Vec<[u64; 3]> = (0..3)
            .map(|_| [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P])
            .chain([[5, 0, 0]])
            .collect();
        let base = (inputs + consts.len()) as u32;
        let ops: Vec<(u8, u32, u32)> = (0..150u32)
            .map(|k| {
                let code = (rng(&mut r) % 5) as u8;
                let a = (rng(&mut r) % (base + k) as u64) as u32;
                let b = if code == OP_MULB {
                    (consts.len() - 1) as u32
                } else {
                    (rng(&mut r) % (base + k) as u64) as u32
                };
                (code, a, b)
            })
            .collect();
        let prog = Program::from_slots(&ops, consts, inputs, base + 149);
        let len = 1usize << 13;
        let cols: Vec<[u64; 3]> = (0..inputs * len)
            .map(|_| [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P])
            .collect();
        let Some(mut dev) = s.backend.line_session(&prog, &cols, len) else {
            assert!(
                !required(s.backend.name()),
                "{}: no line session",
                s.backend.name()
            );
            continue;
        };
        let mut cpu = CpuSession::new(&prog, &cols, len);
        let mut rows = len;
        while rows >= 2 {
            let weights: Vec<[u64; 3]> = (0..rows / 2)
                .map(|_| [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P])
                .collect();
            let xs = [0u64, 2, 3, 5];
            assert_eq!(
                dev.sums(&weights, &xs),
                cpu.sums(&weights, &xs),
                "{} rows {rows}",
                s.backend.name()
            );
            let alpha = [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P];
            dev.fold(alpha);
            cpu.fold(alpha);
            rows /= 2;
            assert_eq!(
                dev.columns(),
                cpu.columns(),
                "{} columns at {rows} rows",
                s.backend.name()
            );
        }
        assert_eq!(dev.firsts(), cpu.firsts(), "{}", s.backend.name());
    }
}

#[test]
fn merkle_levels_equal_the_reference() {
    use cyber_lens_rspcs::backend::merkle_levels_reference;
    let mut r = 31u64;
    for s in backends() {
        for arities in [
            vec![2usize; 13],
            vec![4, 4, 4, 4, 4, 4, 2],
            vec![2usize; 18],
        ] {
            let n: usize = arities.iter().product();
            let leaves: Vec<[u64; 4]> = (0..n)
                .map(|_| core::array::from_fn(|_| rng(&mut r) % P))
                .collect();
            match s.backend.merkle_levels(&leaves, &arities, 0x6e6f6465) {
                Some(got) => assert_eq!(
                    got,
                    merkle_levels_reference(&leaves, &arities, 0x6e6f6465),
                    "{} {arities:?}",
                    s.backend.name()
                ),
                None => assert!(
                    !required(s.backend.name()),
                    "{}: merkle levels declined",
                    s.backend.name()
                ),
            }
        }
    }
}

/// The asynchronous WebGPU path (a browser's): the same results, awaited
/// through the map callback without a blocking device wait.
#[cfg(feature = "webgpu")]
#[test]
fn webgpu_async_ops_equal_cpu() {
    use cyber_lens_rspcs::backend::merkle_levels_reference;
    let w = match cyber_lens_gpu::webgpu::WebGpu::new() {
        Ok(w) => w,
        Err(e) => {
            assert!(!required("webgpu"), "{e}");
            return;
        }
    };
    let mut r = 5u64;
    let coeffs: Vec<u64> = (0..1u64 << 13).map(|_| rng(&mut r) % P).collect();
    let shifts = [3u64, 7, 1];
    let got = pollster::block_on(w.coset_ntt_async(&coeffs, &shifts)).expect("async ntt");
    assert_eq!(got, Cpu.coset_ntt(&coeffs, &shifts));
    let leaves: Vec<[u64; 4]> = (0..1usize << 12)
        .map(|_| core::array::from_fn(|_| rng(&mut r) % P))
        .collect();
    let arities = [4usize, 4, 4, 4, 4, 4];
    let got = pollster::block_on(w.merkle_levels_async(&leaves, &arities, 0x6e6f6465))
        .expect("async levels");
    assert_eq!(got, merkle_levels_reference(&leaves, &arities, 0x6e6f6465));
}

/// Coset counts that leave the 2-D block grid ragged (not a multiple of
/// its row): the regression of a run whose proof differed on WebGPU.
#[test]
fn ragged_coset_groups_equal_the_reference() {
    use cyber_lens_rspcs::backend::{CosetLeaves, coset_leaves_reference};
    let mut r = 4141u64;
    for s in backends() {
        // k = 2^21: 2^12 blocks a coset; 14 cosets = 57,344 blocks (rows of 32,768)
        let k = 1usize << 21;
        let data: Vec<Vec<u64>> = vec![(0..k).map(|_| rng(&mut r) % P).collect()];
        let mut init = [0u64; 16];
        init[9] = 2;
        let job = CosetLeaves {
            limbs: data.iter().map(|v| v.as_slice()).collect(),
            limbs_per_member: 1,
            log_n: 29,
            log_width: 4,
            init,
            rate: 9,
        };
        let range = 242..256;
        match s.backend.coset_leaves(&job, range.clone()) {
            Some(got) => {
                let want = coset_leaves_reference(&job, range.clone());
                assert!(
                    got == want,
                    "{}: ragged coset leaves differ at {:?}",
                    s.backend.name(),
                    got.iter().zip(&want).position(|(a, b)| a != b)
                );
            }
            None => assert!(
                !required(s.backend.name()),
                "{}: declined",
                s.backend.name()
            ),
        }
        let shifts: Vec<u64> = (0..14).map(|i| job.shift(242 + i)).collect();
        assert_eq!(
            s.backend.coset_ntt(&data[0], &shifts),
            Cpu.coset_ntt(&data[0], &shifts),
            "{}: ragged NTT",
            s.backend.name()
        );
    }
}

/// States whose first partial S-box input is zero (0⁷ = 0), built
/// backwards from the target through the first six full rounds and the
/// initial external layer; placed among ordinary states at every position
/// of a group of eight, so each lane of a multi-state GPU thread meets one
/// (the fast kernels carry non-canonical representatives: a zero may
/// arrive as p).
#[test]
fn a_zero_partial_sbox_input_in_every_batch_position() {
    use nebu::Goldilocks as G;
    let rc = cyber_lens_gpu::consts::round_constants();
    let ic = cyber_lens_gpu::consts::INTERNAL;
    // the Poseidon2 reference block, as a matrix (not the addition schedule)
    const M4: [[u64; 4]; 4] = [[5, 7, 1, 3], [4, 6, 1, 1], [1, 3, 5, 7], [1, 1, 4, 6]];
    let mat4 = |x: [G; 4]| -> [G; 4] {
        core::array::from_fn(|r| (0..4).fold(G::ZERO, |a, c| a + G::new(M4[r][c]) * x[c]))
    };
    let external = |s: [G; 16]| -> [G; 16] {
        let mut y = [G::ZERO; 16];
        for c in 0..4 {
            let m = mat4([s[4 * c], s[4 * c + 1], s[4 * c + 2], s[4 * c + 3]]);
            y[4 * c..4 * c + 4].copy_from_slice(&m);
        }
        let mut out = y;
        for k in 0..4 {
            let sum = y[k] + y[4 + k] + y[8 + k] + y[12 + k];
            for c in 0..4 {
                out[4 * c + k] = y[4 * c + k] + sum;
            }
        }
        out
    };
    // the external layer's inverse by Gauss–Jordan on its matrix
    let mut m = [[G::ZERO; 32]; 16];
    for j in 0..16 {
        let mut e = [G::ZERO; 16];
        e[j] = G::ONE;
        let col = external(e);
        for i in 0..16 {
            m[i][j] = col[i];
        }
        m[j][16 + j] = G::ONE;
    }
    for c in 0..16 {
        let piv = (c..16).find(|&r| m[r][c] != G::ZERO).expect("invertible");
        m.swap(c, piv);
        let inv = m[c][c].inv();
        for x in m[c].iter_mut() {
            *x *= inv;
        }
        for r in 0..16 {
            if r != c && m[r][c] != G::ZERO {
                let f = m[r][c];
                let row = m[c];
                for (x, y) in m[r].iter_mut().zip(row) {
                    *x -= f * y;
                }
            }
        }
    }
    let inv_external = |s: [G; 16]| -> [G; 16] {
        core::array::from_fn(|i| (0..16).fold(G::ZERO, |a, j| a + m[i][16 + j] * s[j]))
    };
    // 7⁻¹ mod p − 1
    let pm1 = (P - 1) as i128;
    let e7 = {
        let (mut a, mut b, mut x0, mut x1) = (7i128, pm1, 1i128, 0i128);
        while b != 0 {
            let q = a / b;
            (a, b) = (b, a - q * b);
            (x0, x1) = (x1, x0 - q * x1);
        }
        assert_eq!(a, 1);
        x0.rem_euclid(pm1)
    };
    let root7 = |x: G| x.exp(e7 as u64);
    let mut r = 0x2e70u64;
    let crafted = |r: &mut u64| -> State {
        let mut t: [G; 16] = core::array::from_fn(|_| G::new(rng(r) % P));
        t[0] = G::ZERO - G::new(rc[ic]);
        let mut s = t;
        for round in (0..6).rev() {
            s = inv_external(s);
            for i in 0..16 {
                s[i] = root7(s[i]) - G::new(rc[round * 16 + i]);
            }
        }
        s = inv_external(s);
        // forward check: lane 0 + c₀ is zero before the first partial S-box
        let mut f = external(s);
        for round in 0..6 {
            for i in 0..16 {
                f[i] = (f[i] + G::new(rc[round * 16 + i])).exp(7);
            }
            f = external(f);
        }
        assert_eq!(f[0] + G::new(rc[ic]), G::ZERO);
        core::array::from_fn(|i| s[i].as_u64())
    };
    for sb in backends() {
        for pos in 0..8 {
            let mut states = random_states(4099, 7 + pos as u64);
            for k in 0..64 {
                states[pos + 8 * k] = crafted(&mut r);
            }
            let mut want = states.clone();
            Cpu.permute(&mut want);
            let mut got = states;
            sb.backend.permute(&mut got);
            assert_eq!(
                got,
                want,
                "{} zero at batch position {pos}",
                sb.backend.name()
            );
        }
    }
}

#[test]
fn byte_trees_equal_the_cpu_tree() {
    use cyber_lens_rspcs::backend::ByteTree;
    use cyber_lens_rspcs::merkle::Tree;
    let mut r = 77u64;
    for s in backends() {
        for (limbs, width, leaves) in [
            (1usize, 16usize, 1usize << 12),
            (3, 16, 1 << 12),
            (1, 32, 1 << 13),
            (3, 32, 1 << 12),
        ] {
            // canonical limbs plus a few non-canonical representatives (v + p)
            let symbols: Vec<u64> = (0..limbs * width * leaves)
                .map(|i| {
                    let v = rng(&mut r) % P;
                    if i % 97 == 0 && v < u64::MAX - P {
                        v + P
                    } else {
                        v
                    }
                })
                .collect();
            let job = ByteTree {
                symbols: &symbols,
                limbs,
                width,
                leaves,
            };
            let Some(levels) = s.backend.byte_tree(&job) else {
                assert!(
                    s.backend.name() != "metal" || !required("metal"),
                    "metal: byte tree declined"
                );
                continue;
            };
            let want = Tree::build_on_cpu(leaves, |j| {
                let mut b = Vec::with_capacity(width * limbs * 8);
                for t in 0..width {
                    for l in 0..limbs {
                        b.extend_from_slice(
                            &(symbols[(j + t * leaves) * limbs + l] % P).to_le_bytes(),
                        );
                    }
                }
                b
            });
            let root = levels.last().expect("root")[0];
            let mut rb = [0u8; 32];
            for i in 0..4 {
                rb[8 * i..8 * i + 8].copy_from_slice(&root[i].to_le_bytes());
            }
            assert_eq!(
                rb,
                *want.root().as_bytes(),
                "{} limbs {limbs} width {width}",
                s.backend.name()
            );
        }
    }
}

/// A header-mining job of a 168-byte message as hemera hashes it (zero
/// state, 7-byte limbs, length in lane 10, pad block `0x01 ‖ 0…`), the
/// nonce bytes 24..40 zeroed in block 0.
fn mine_job(msg: &[u8; 168], target: [u64; 4]) -> cyber_lens_gpu::mine::Job {
    let limb = |b: &[u8]| {
        let mut w = [0u8; 8];
        w[..b.len()].copy_from_slice(b);
        u64::from_le_bytes(w)
    };
    let mut blocks = [[0u64; 8]; 4];
    for (b, block) in blocks.iter_mut().enumerate().take(3) {
        for (i, l) in block.iter_mut().enumerate() {
            *l = limb(&msg[b * 56 + i * 7..b * 56 + i * 7 + 7]);
        }
    }
    blocks[3][0] = 1;
    blocks[0][3] &= 0x00ff_ffff;
    blocks[0][4] = 0;
    blocks[0][5] &= !0xff_ffff_ffff;
    cyber_lens_gpu::mine::Job {
        init: [0; 16],
        blocks,
        tail_lane: 10,
        tail: 168,
        target,
    }
}

#[allow(unused_variables)] // every arm is feature-gated
fn mine_on(
    s: &cyber_lens_gpu::Selected,
    job: &cyber_lens_gpu::mine::Job,
    base: u128,
    count: u32,
) -> Option<u32> {
    match s.backend.name() {
        #[cfg(all(feature = "metal", target_os = "macos", target_arch = "aarch64"))]
        "metal" => cyber_lens_gpu::metal::Metal::new()
            .expect("metal")
            .mine(job, base, count)
            .expect("metal mine"),
        #[cfg(feature = "webgpu")]
        "webgpu" => cyber_lens_gpu::webgpu::WebGpu::new()
            .expect("webgpu")
            .mine(job, base, count)
            .expect("webgpu mine"),
        other => panic!("no mine on {other}"),
    }
}

#[test]
fn mining_finds_the_cpu_nonce_and_hemera_agrees() {
    use cyber_lens_gpu::mine::{below, cpu, digest};
    let mut r = 0x1234_5678_9abc_u64;
    let devices = backends();
    for round in 0..8 {
        let mut msg = [0u8; 168];
        for b in msg.iter_mut() {
            *b = rng(&mut r) as u8;
        }
        // targets 2^244 .. 2^251: one in 4096 .. one in 32 digests pass
        let target = [0, 0, 0, 1u64 << (52 + round % 8)];
        let job = mine_job(&msg, target);
        // bases across the low word, the high word, and a 2^64 boundary neighbourhood
        let base: u128 = match round % 3 {
            0 => rng(&mut r) as u128 % (1 << 40),
            1 => ((rng(&mut r) as u128) << 64) | (rng(&mut r) as u128 % (1 << 40)),
            _ => ((rng(&mut r) as u128) << 64) | (u64::MAX as u128 - 70_000),
        };
        let count = 1u32 << 16;
        let want = cpu(&job, base, count);
        let i = want.expect("a nonce in 2^16 at 2^244 or easier");
        // the digest is hemera's hash of the header with that nonce
        let n = base + i as u128;
        msg[24..40].copy_from_slice(&n.to_le_bytes());
        let d = digest(&job, n);
        let bytes: Vec<u8> = d.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(bytes, cyber_hemera::hash(&msg).as_bytes(), "round {round}");
        assert!(below(&d, &target));
        for s in &devices {
            let got = mine_on(s, &job, base, count);
            assert_eq!(
                got,
                want,
                "{} round {round} base {base:#x}",
                s.backend.name()
            );
        }
    }
}

#[test]
fn mining_reports_none_when_nothing_passes() {
    let job = mine_job(&[0x77; 168], [0, 0, 0, 0]);
    assert_eq!(cyber_lens_gpu::mine::cpu(&job, 5, 4096), None);
    for s in backends() {
        assert_eq!(mine_on(&s, &job, 5, 4096), None, "{}", s.backend.name());
    }
}
