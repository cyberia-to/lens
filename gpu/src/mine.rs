// ---
// tags: lens, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Header mining: the flat hemera sponge over a fixed-size message with a
//! 16-byte nonce, searched for a digest below a 256-bit target.
//!
//! The message is soft3's 168-byte chain header (`soft3/specs/chain.md`
//! §1.1): three full 56-byte rate blocks and the padded fourth block, four
//! permutations per hash, the nonce at bytes 24..40 of the first block.
//! Hemera absorbs by field addition of 7-byte limbs, so a nonce `n` enters
//! the first block as three lane additions of values below `2^56 < p`
//! (integer additions on disjoint bit ranges):
//!
//! | lane | header bytes | nonce bytes | added value |
//! |---|---|---|---|
//! | 3 | 21..28 | 0..4 at byte 3 | `(n mod 2^32) · 2^24` |
//! | 4 | 28..35 | 4..11 | `⌊n / 2^32⌋ mod 2^56` |
//! | 5 | 35..42 | 11..16 at byte 0 | `⌊n / 2^88⌋` (40 bits) |
//!
//! A [`Job`] carries the sponge's initial state, the four blocks with the
//! nonce lanes zero, the length lane hemera sets before its final
//! permutation, and the target. Every backend answers the same question for
//! the same `(job, base, count)`: the least `i < count` whose nonce
//! `base + i` gives a digest below the target, read as a 256-bit
//! little-endian integer of the four canonical output limbs — so a nonce
//! found on any device verifies with `hemera::hash` on any other.
//!
//! [`cpu`] is the reference the GPU kernels are checked against
//! (`tests/differential.rs`); callers mining on the CPU use hemera's batch
//! path directly.

use cyber_lens_rspcs::backend::{Backend, Cpu, State};

/// Lane of the first nonce limb and the shift of the nonce's low 32 bits in it.
pub const LANE_LO: usize = 3;
/// Bits of the nonce's first four bytes within lane 3 (bytes 3..7 of the limb).
pub const SHIFT_LO: u32 = 24;
/// Lane holding nonce bytes 4..11 whole.
pub const LANE_MID: usize = 4;
/// Lane holding nonce bytes 11..16 at its low 40 bits.
pub const LANE_HI: usize = 5;

/// A header-mining job: the sponge state before the first block, the four
/// rate blocks as lanes (nonce lanes zero), the length lane, the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Job {
    /// Sponge state before absorbing block 0 (hemera: zeros, domain lane).
    pub init: State,
    /// The four rate blocks as 7-byte limbs; `blocks[0]` has the nonce's
    /// bit ranges zero.
    pub blocks: [[u64; 8]; 4],
    /// Lane written (not added) before the final permutation and its
    /// value: hemera stores the message length in capacity lane 2.
    pub tail_lane: usize,
    pub tail: u64,
    /// The target as four little-endian 64-bit words (word 3 most
    /// significant); a digest passes when it is strictly below.
    pub target: [u64; 4],
}

/// The three lane additions of nonce `n`: `(lane 3, lane 4, lane 5)`.
pub const fn nonce_lanes(n: u128) -> (u64, u64, u64) {
    (
        ((n as u64) & 0xffff_ffff) << SHIFT_LO,
        ((n >> 32) as u64) & 0x00ff_ffff_ffff_ffff,
        ((n >> 88) as u64) & 0xff_ffff_ffff,
    )
}

/// `a < b` as 256-bit little-endian integers of four words.
pub fn below(a: &[u64; 4], b: &[u64; 4]) -> bool {
    for i in (0..4).rev() {
        if a[i] != b[i] {
            return a[i] < b[i];
        }
    }
    false
}

const P: u64 = 0xFFFF_FFFF_0000_0001;

fn add(a: u64, b: u64) -> u64 {
    let (s, c) = a.overflowing_add(b);
    let s = if c { s.wrapping_sub(P) } else { s };
    if s >= P { s - P } else { s }
}

/// The permutation inputs of nonces `base + i`, `i < count`: block 0 and the
/// nonce lanes added to the initial state.
pub fn seeds(job: &Job, base: u128, count: usize) -> Vec<State> {
    (0..count as u128)
        .map(|i| {
            let (a3, a4, a5) = nonce_lanes(base + i);
            let mut s = job.init;
            for (l, b) in s.iter_mut().zip(job.blocks[0]) {
                *l = add(*l, b);
            }
            s[LANE_LO] = add(s[LANE_LO], a3);
            s[LANE_MID] = add(s[LANE_MID], a4);
            s[LANE_HI] = add(s[LANE_HI], a5);
            s
        })
        .collect()
}

/// The remaining three blocks and the length lane applied to permuted
/// states, with `permute` (a backend's batch permutation) between them.
pub fn finish(job: &Job, states: &mut [State], mut permute: impl FnMut(&mut [State])) {
    permute(states);
    for b in 1..4 {
        for s in states.iter_mut() {
            for (l, v) in s.iter_mut().zip(job.blocks[b]) {
                *l = add(*l, v);
            }
            if b == 3 {
                s[job.tail_lane] = job.tail;
            }
        }
        permute(states);
    }
}

/// Reference search on the CPU backend: the least `i < count` whose digest
/// is below the target, or `None`.
pub fn cpu(job: &Job, base: u128, count: u32) -> Option<u32> {
    const CHUNK: usize = 1 << 12;
    let mut i = 0usize;
    while i < count as usize {
        let n = CHUNK.min(count as usize - i);
        let mut st = seeds(job, base + i as u128, n);
        finish(job, &mut st, |s| Cpu.permute(s));
        if let Some(k) = st
            .iter()
            .position(|s| below(&[s[0], s[1], s[2], s[3]], &job.target))
        {
            return Some((i + k) as u32);
        }
        i += n;
    }
    None
}

/// The digest of nonce `n` on the CPU backend: the four output limbs.
pub fn digest(job: &Job, n: u128) -> [u64; 4] {
    let mut st = seeds(job, n, 1);
    finish(job, &mut st, |s| Cpu.permute(s));
    [st[0][0], st[0][1], st[0][2], st[0][3]]
}

/// The nonces a backend tries in one dispatch: `base + i` for `i < count`
/// never carries into the high 64 bits (`count ≤ 2^32` and the host splits
/// a range at a 2^64 boundary), so a kernel adds `i` to the low word only.
pub fn no_carry(base: u128, count: u32) -> bool {
    (base as u64).checked_add(count as u64).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The job of a 168-byte message the way hemera hashes it: zero state
    /// with domain lane 11 = 0, blocks of 7-byte limbs, length in lane 10,
    /// the padded fourth block `0x01 ‖ 0…`.
    fn job_of(msg: &[u8; 168], target: [u64; 4]) -> Job {
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
        // nonce bytes 24..40 zeroed in block 0
        blocks[0][3] &= 0x00ff_ffff;
        blocks[0][4] = 0;
        blocks[0][5] &= !0xff_ffff_ffff;
        Job {
            init: [0; 16],
            blocks,
            tail_lane: 10,
            tail: 168,
            target,
        }
    }

    #[test]
    fn digest_is_hemera_hash() {
        let mut msg = [0u8; 168];
        for (i, b) in msg.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37).wrapping_add(11);
        }
        let job = job_of(&msg, [0; 4]);
        for n in [0u128, 1, 0xdead_beef, u64::MAX as u128 + 5, u128::MAX - 7] {
            msg[24..40].copy_from_slice(&n.to_le_bytes());
            let want = cyber_hemera::hash(&msg);
            let got = digest(&job, n);
            let got_bytes: Vec<u8> = got.iter().flat_map(|w| w.to_le_bytes()).collect();
            assert_eq!(got_bytes, want.as_bytes(), "nonce {n}");
        }
    }

    #[test]
    fn cpu_finds_the_least_passing_nonce() {
        let msg = [0x5au8; 168];
        // target 2^244: one in 4096 digests pass
        let job = job_of(&msg, [0, 0, 0, 1 << 52]);
        let i = cpu(&job, 1_000, 1 << 16).expect("a nonce in 2^16");
        for k in 0..=i {
            let d = digest(&job, 1_000 + k as u128);
            assert_eq!(below(&d, &job.target), k == i, "nonce offset {k}");
        }
    }

    #[test]
    fn below_is_strict_256_bit_order() {
        assert!(below(&[u64::MAX, u64::MAX, u64::MAX, 0], &[0, 0, 0, 1]));
        assert!(!below(&[0, 0, 0, 1], &[0, 0, 0, 1]));
        assert!(below(&[1, 0, 0, 1], &[2, 0, 0, 1]));
        assert!(!below(&[2, 0, 0, 1], &[1, 0, 0, 1]));
    }
}
