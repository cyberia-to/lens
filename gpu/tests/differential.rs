//! Every GPU backend against the CPU backend on random inputs: the same
//! permutation outputs (canonical limbs), the same least grinding nonce for
//! both nonce injections. A backend that is not built or has no device is
//! reported and skipped.

use cyber_lens_rspcs::backend::{Backend, Cpu, Inject, State, hash40_template};

const P: u64 = 0xFFFF_FFFF_0000_0001;

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

#[test]
fn zero_lane_inverse_is_zero() {
    // a state whose lane 0 is zero before the first partial S-box exercises 0⁻¹ = 0
    // only by chance; check the CPU and the GPU agree on all-zero inputs too
    for s in backends() {
        let mut want = vec![[0u64; 16]; 4096];
        Cpu.permute(&mut want);
        let mut got = vec![[0u64; 16]; 4096];
        s.backend.permute(&mut got);
        assert_eq!(got, want, "{}", s.backend.name());
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
                assert_eq!(s.backend.grind(&t, Inject::hash40(), bits), want, "{} bits {bits} seed {seed} (nonce {want})", s.backend.name());
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn metal_shared_inversions_equal_cpu() {
    // several states per GPU thread share each partial round's inversion;
    // a ragged tail (n not a multiple of k) and zero lanes included
    for k in [2usize, 3, 4] {
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
        let cols: Vec<[u64; 3]> = (0..inputs * 2 * pairs)
            .map(|_| [rng(&mut r) % P, rng(&mut r) % P, rng(&mut r) % P])
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
                        assert_eq!(got, coset_leaves_reference(&job, range.clone()), "{} kept leaves {range:?}", s.backend.name());
                        assert!(vals == coset_values_reference(&job, range.clone()), "{} kept values {range:?}", s.backend.name());
                    }
                    None => assert!(!required(s.backend.name()), "{}: kept coset leaves declined", s.backend.name()),
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
