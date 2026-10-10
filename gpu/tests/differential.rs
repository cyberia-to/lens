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
            Err(e) => eprintln!("{name}: skipped ({e})"),
        }
    }
    v
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
    // 20 bits: the least nonce lies beyond the first 2^16 dispatch with
    // probability 1 − 2^-... ≈ 94%; both backends must still return the least
    let seed = [7u8; 32];
    let t = hash40_template(&seed);
    let want = Cpu.grind(&t, Inject::hash40(), 20);
    for s in backends() {
        assert_eq!(
            s.backend.grind(&t, Inject::hash40(), 20),
            want,
            "{}",
            s.backend.name()
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn metal_shared_inversions_equal_cpu() {
    // several states per GPU thread share each partial round's inversion;
    // a ragged tail (n not a multiple of k) and zero lanes included
    for k in [2usize, 3, 4] {
        let Ok(m) = cyber_lens_gpu::metal::Metal::with_states_per_thread(k) else {
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
        assert_eq!(m.grind(&t, Inject::hash40(), 12), Cpu.grind(&t, Inject::hash40(), 12), "k={k}");
    }
}
