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

use crate::par::par_map;

pub struct Tree {
    /// `levels[0]` = leaf digests, last = `[root]`.
    levels: Vec<Vec<Hash>>,
}

impl Tree {
    /// Build over `count` leaves (a power of two); `leaf(i)` yields the
    /// canonical bytes of leaf `i`.
    pub fn build(count: usize, leaf: impl Fn(usize) -> Vec<u8> + Sync) -> Self {
        assert!(count.is_power_of_two(), "leaf count must be a power of two");
        let single = count == 1;
        let leaves = par_map(count, |i| hash_leaf(&leaf(i), i as u64, single));
        let mut levels = vec![leaves];
        while levels.last().expect("non-empty").len() > 1 {
            let prev = levels.last().expect("non-empty");
            let is_root = prev.len() == 2;
            let next = par_map(prev.len() / 2, |i| {
                hash_node(&prev[2 * i], &prev[2 * i + 1], is_root)
            });
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
