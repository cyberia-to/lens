//! Binary hemera Merkle tree over a power-of-two number of leaves, with
//! deduplicated multi-openings.
//!
//! Leaf `i` with bytes `b` hashes to `hemera::tree::hash_leaf(b, i, single)`
//! (chunk flag, counter `i` in the capacity); inner nodes are
//! `hemera::tree::hash_node(l, r, is_root)` (parent flag). The flags
//! domain-separate leaves from inner nodes and the root from interior
//! nodes; the counter binds each leaf to its position. Digests are 32
//! bytes.
//!
//! A multi-opening of a sorted set of distinct leaf indices carries only
//! the sibling digests the verifier cannot recompute, level by level from
//! the leaves up, in increasing index order within a level. The verifier
//! consumes the list exactly; a short, long or reordered list fails.

use cyber_hemera::Hash;
use cyber_hemera::tree::{hash_leaf, hash_leaf_batch, hash_node, hash_node_batch};
use cyber_lens_core::PcsError;

use crate::par::{par_map, par_map_min};

pub struct Tree {
    /// `levels[0]` = leaf digests, last = `[root]`.
    levels: Vec<Vec<Hash>>,
}

impl Tree {
    /// Build over `count` leaves (a power of two); `leaf(i)` yields the
    /// canonical bytes of leaf `i`. The digests are hemera's
    /// (`hash_leaf`, `hash_node`) whichever backend computes them: hemera's
    /// batched kernel on the CPU, [`crate::backend::Backend::permute`]
    /// batches on a device (`leaf_digests_device`).
    pub fn build(count: usize, leaf: impl Fn(usize) -> Vec<u8> + Sync) -> Self {
        assert!(count.is_power_of_two(), "leaf count must be a power of two");
        let single = count == 1;
        let backend = crate::backend::current();
        let device = backend.name() != "cpu";
        let leaves = if device && count >= DEVICE_MIN {
            leaf_digests_device(&*backend, count, &leaf, single)
        } else {
            leaf_digests_cpu(count, &leaf, single)
        };
        let mut levels = vec![leaves];
        while levels.last().expect("non-empty").len() > 1 {
            let prev = levels.last().expect("non-empty");
            let is_root = prev.len() == 2;
            let next = if device && prev.len() / 2 >= DEVICE_MIN {
                nodes_device(&*backend, prev, is_root)
            } else {
                nodes_cpu(prev, is_root)
            };
            levels.push(next);
        }
        Self { levels }
    }

    pub fn root(&self) -> Hash {
        self.levels.last().expect("non-empty")[0]
    }

    /// Sibling digests for the sorted, distinct `indices`.
    pub fn open(&self, indices: &[usize]) -> Vec<Hash> {
        let mut out = Vec::new();
        let mut known: Vec<usize> = indices.to_vec();
        for level in &self.levels[..self.levels.len() - 1] {
            let mut next = Vec::with_capacity(known.len());
            let mut i = 0;
            while i < known.len() {
                let idx = known[i];
                let sib = idx ^ 1;
                if i + 1 < known.len() && known[i + 1] == sib {
                    i += 2;
                } else {
                    out.push(level[sib]);
                    i += 1;
                }
                next.push(idx >> 1);
            }
            next.dedup();
            known = next;
        }
        out
    }
}

/// Recompute the root from leaf digests at sorted distinct `indices` in
/// a tree of `2^depth` leaves and compare with `root`.
pub fn verify(
    root: &Hash,
    depth: usize,
    indices: &[usize],
    leaves: Vec<Hash>,
    siblings: &[Hash],
) -> Result<(), PcsError> {
    if indices.is_empty() || indices.len() != leaves.len() {
        return Err(PcsError::Shape);
    }
    if indices.windows(2).any(|w| w[0] >= w[1]) || indices[indices.len() - 1] >> depth != 0 {
        return Err(PcsError::Shape);
    }
    let mut nodes: Vec<(usize, Hash)> = indices.iter().copied().zip(leaves).collect();
    let mut sib_iter = siblings.iter();
    // Every parent of one level is independent of the others: collect the
    // level's (left, right) pairs, then hash them in one batched call
    // (hemera runs 16 compressions interleaved). Siblings are consumed in
    // the same order as before — level by level, increasing index.
    for level in 0..depth {
        let is_root = level + 1 == depth;
        let mut idx_next = Vec::with_capacity(nodes.len());
        let mut pairs = Vec::with_capacity(nodes.len());
        let mut i = 0;
        while i < nodes.len() {
            let (idx, h) = nodes[i];
            if i + 1 < nodes.len() && nodes[i + 1].0 == idx ^ 1 {
                pairs.push((h, nodes[i + 1].1));
                i += 2;
            } else {
                let s = sib_iter.next().ok_or(PcsError::Merkle)?;
                i += 1;
                pairs.push(if idx & 1 == 0 { (h, *s) } else { (*s, h) });
            }
            idx_next.push(idx >> 1);
        }
        let mut out = vec![Hash::from_bytes([0; 32]); pairs.len()];
        hash_node_batch(&pairs, is_root, &mut out);
        nodes = idx_next.into_iter().zip(out).collect();
    }
    if sib_iter.next().is_some() {
        return Err(PcsError::Merkle);
    }
    if nodes.len() == 1 && nodes[0].1 == *root {
        Ok(())
    } else {
        Err(PcsError::Merkle)
    }
}

/// Leaf digests of opened leaves, `leaves[q] = (bytes, index)`, as the tree
/// computes them; equal-length leaves are hashed in one batched call.
pub fn leaf_digests(leaves: &[(&[u8], usize)], count: usize) -> Vec<Hash> {
    let items: Vec<(&[u8], u64)> = leaves.iter().map(|&(b, i)| (b, i as u64)).collect();
    let mut out = vec![Hash::from_bytes([0; 32]); items.len()];
    hash_leaf_batch(&items, count == 1, &mut out);
    out
}

/// Leaf digest as the tree computes it.
pub fn leaf_digest(bytes: &[u8], index: usize, count: usize) -> Hash {
    hash_leaf(bytes, index as u64, count == 1)
}

/// Sorted distinct copy of sampled indices.
pub fn dedup_sorted(mut idx: Vec<usize>) -> Vec<usize> {
    idx.sort_unstable();
    idx.dedup();
    idx
}

/// Leaves (or nodes) below which the device is not worth a dispatch.
const DEVICE_MIN: usize = 1 << 12;
/// Leaves per device group: bounds the staging memory.
const DEVICE_GROUP: usize = 1 << 16;
/// Leaves per CPU work unit (a multiple of hemera's 16-lane batch).
const CPU_GROUP: usize = 1 << 8;

// hemera's sponge and tree lanes (hemera `sponge.rs`, `tree.rs`): the
// plain-mode sponge starts at zero (DOMAIN_HASH = 0 in capacity[3]),
// absorbs 7-byte little-endian lanes by field addition, pads with 0x01
// and writes the byte length into capacity[2] (lane 10) before the last
// permutation; tree states carry a counter in lane 8 and flags in lane 9.
const RATE_BYTES: usize = 56;
const LANE_LENGTH: usize = 10;
const LANE_COUNTER: usize = 8;
const LANE_FLAGS: usize = 9;
const FLAG_ROOT: u64 = 1;
const FLAG_PARENT: u64 = 2;
const FLAG_CHUNK: u64 = 4;
const P: u64 = 0xFFFF_FFFF_0000_0001;

fn digest_lanes(h: &Hash) -> [u64; 4] {
    let b = h.as_bytes();
    core::array::from_fn(|i| u64::from_le_bytes(b[8 * i..8 * i + 8].try_into().expect("8 bytes")))
}

fn lanes_digest(l: &[u64]) -> Hash {
    let mut b = [0u8; 32];
    for i in 0..4 {
        b[8 * i..8 * i + 8].copy_from_slice(&l[i].to_le_bytes());
    }
    Hash::from_bytes(b)
}

/// Add the 56-byte block `bytes` (shorter: zero-extended) into the rate.
fn absorb7(state: &mut [u64; 16], bytes: &[u8]) {
    for (lane, chunk) in state.iter_mut().zip(bytes.chunks(7)) {
        let mut buf = [0u8; 8];
        buf[..chunk.len()].copy_from_slice(chunk);
        // both summands are below p; one conditional subtraction reduces
        let (sum, carry) = lane.overflowing_add(u64::from_le_bytes(buf));
        *lane = if carry || sum >= P { sum.wrapping_sub(P) } else { sum };
    }
}

fn leaf_digests_cpu(count: usize, leaf: &(impl Fn(usize) -> Vec<u8> + Sync), single: bool) -> Vec<Hash> {
    let groups = par_map_min(count.div_ceil(CPU_GROUP), 2, |g| {
        let lo = g * CPU_GROUP;
        let hi = (lo + CPU_GROUP).min(count);
        let bytes: Vec<Vec<u8>> = (lo..hi).map(leaf).collect();
        let items: Vec<(&[u8], u64)> = bytes.iter().zip(lo..hi).map(|(b, i)| (b.as_slice(), i as u64)).collect();
        let mut out = vec![Hash::from_bytes([0; 32]); items.len()];
        hash_leaf_batch(&items, single, &mut out);
        out
    });
    groups.into_iter().flatten().collect()
}

fn nodes_cpu(prev: &[Hash], is_root: bool) -> Vec<Hash> {
    let n = prev.len() / 2;
    let groups = par_map_min(n.div_ceil(CPU_GROUP), 2, |g| {
        let lo = g * CPU_GROUP;
        let hi = (lo + CPU_GROUP).min(n);
        let pairs: Vec<(Hash, Hash)> = (lo..hi).map(|i| (prev[2 * i], prev[2 * i + 1])).collect();
        let mut out = vec![Hash::from_bytes([0; 32]); pairs.len()];
        hash_node_batch(&pairs, is_root, &mut out);
        out
    });
    groups.into_iter().flatten().collect()
}

/// `hash_leaf(leaf(i), i, single)` for every leaf, as rounds of batched
/// permutations on `backend`: the sponge blocks of every leaf of a group,
/// then the re-derivation with the counter and the chunk flag. Leaves of
/// unequal length take the CPU path.
fn leaf_digests_device(
    backend: &dyn crate::backend::Backend,
    count: usize,
    leaf: &(impl Fn(usize) -> Vec<u8> + Sync),
    single: bool,
) -> Vec<Hash> {
    let mut out = Vec::with_capacity(count);
    let mut lo = 0;
    while lo < count {
        let hi = (lo + DEVICE_GROUP).min(count);
        let bytes: Vec<Vec<u8>> = par_map(hi - lo, |k| leaf(lo + k));
        let len = bytes[0].len();
        if bytes.iter().any(|b| b.len() != len) {
            let items: Vec<(&[u8], u64)> =
                bytes.iter().zip(lo..hi).map(|(b, i)| (b.as_slice(), i as u64)).collect();
            out.extend(items.iter().map(|&(b, i)| hash_leaf(b, i, single)));
            lo = hi;
            continue;
        }
        let full = len / RATE_BYTES;
        let mut states = vec![[0u64; 16]; hi - lo];
        for b in 0..full {
            par_fill(&mut states, |k, st| absorb7(st, &bytes[k][b * RATE_BYTES..(b + 1) * RATE_BYTES]));
            backend.permute(&mut states);
        }
        par_fill(&mut states, |k, st| {
            let rem = &bytes[k][full * RATE_BYTES..];
            let mut padded = [0u8; RATE_BYTES];
            padded[..rem.len()].copy_from_slice(rem);
            padded[rem.len()] = 0x01;
            absorb7(st, &padded);
            st[LANE_LENGTH] = len as u64;
        });
        backend.permute(&mut states);
        let flags = FLAG_CHUNK | if single { FLAG_ROOT } else { 0 };
        par_fill(&mut states, |k, st| {
            let base = [st[0], st[1], st[2], st[3]];
            *st = [0u64; 16];
            st[..4].copy_from_slice(&base);
            st[LANE_COUNTER] = (lo + k) as u64;
            st[LANE_FLAGS] = flags;
        });
        backend.permute(&mut states);
        out.extend(states.iter().map(|st| lanes_digest(&st[..4])));
        lo = hi;
    }
    out
}

fn nodes_device(backend: &dyn crate::backend::Backend, prev: &[Hash], is_root: bool) -> Vec<Hash> {
    let flags = FLAG_PARENT | if is_root { FLAG_ROOT } else { 0 };
    let mut states = vec![[0u64; 16]; prev.len() / 2];
    par_fill(&mut states, |i, st| {
        st[..4].copy_from_slice(&digest_lanes(&prev[2 * i]));
        st[4..8].copy_from_slice(&digest_lanes(&prev[2 * i + 1]));
        st[LANE_FLAGS] = flags;
    });
    backend.permute(&mut states);
    states.iter().map(|st| lanes_digest(&st[..4])).collect()
}

/// `f(index, item)` over `v` in parallel contiguous chunks.
fn par_fill<T: Send>(v: &mut [T], f: impl Fn(usize, &mut T) + Sync) {
    let t = crate::par::threads();
    let n = v.len();
    if t == 1 || n < 2 * CPU_GROUP {
        v.iter_mut().enumerate().for_each(|(i, x)| f(i, x));
        return;
    }
    let chunk = n.div_ceil(t);
    let f = &f;
    std::thread::scope(|s| {
        for (c, part) in v.chunks_mut(chunk).enumerate() {
            s.spawn(move || part.iter_mut().enumerate().for_each(|(i, x)| f(c * chunk + i, x)));
        }
    });
}

#[cfg(test)]
mod device_tests {
    use super::*;
    use crate::backend::{Backend, Cpu, State};

    /// The CPU kernels under another name: exercises the device paths.
    struct Renamed;
    impl Backend for Renamed {
        fn name(&self) -> &'static str {
            "renamed-cpu"
        }
        fn permute(&self, states: &mut [State]) {
            Cpu.permute(states)
        }
    }

    fn bytes(i: usize, len: usize) -> Vec<u8> {
        (0..len).map(|k| (i * 131 + k * 7 + (k >> 3)) as u8 ^ 0xa5).collect()
    }

    #[test]
    fn device_leaves_are_hemera_leaves_at_every_block_boundary() {
        for len in [0, 1, 7, 55, 56, 57, 111, 112, 113, 128, 384, 1000] {
            for count in [1usize, 2, 64] {
                let leaf = |i: usize| bytes(i, len);
                let got = leaf_digests_device(&Renamed, count, &leaf, count == 1);
                let want: Vec<Hash> = (0..count).map(|i| hash_leaf(&leaf(i), i as u64, count == 1)).collect();
                assert_eq!(got, want, "len {len} count {count}");
            }
        }
    }

    #[test]
    fn device_and_cpu_nodes_are_hemera_nodes() {
        let leaves: Vec<Hash> = (0..64).map(|i| hash_leaf(&bytes(i, 40), i as u64, false)).collect();
        for root in [false, true] {
            let want: Vec<Hash> = (0..32).map(|i| hash_node(&leaves[2 * i], &leaves[2 * i + 1], root)).collect();
            assert_eq!(nodes_device(&Renamed, &leaves, root), want);
            assert_eq!(nodes_cpu(&leaves, root), want);
        }
    }

    #[test]
    fn batched_cpu_leaves_equal_single_leaves() {
        let leaf = |i: usize| bytes(i, 128);
        let n = 3 * CPU_GROUP + 16;
        let want: Vec<Hash> = (0..n).map(|i| hash_leaf(&leaf(i), i as u64, false)).collect();
        assert_eq!(leaf_digests_cpu(n, &leaf, false), want);
    }

    #[test]
    fn unequal_leaves_fall_back_to_single_hashing() {
        let leaf = |i: usize| bytes(i, 10 + i % 3);
        let want: Vec<Hash> = (0..8).map(|i| hash_leaf(&leaf(i), i as u64, false)).collect();
        assert_eq!(leaf_digests_device(&Renamed, 8, &leaf, false), want);
    }
}
