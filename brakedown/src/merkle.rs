//! Merkle tree over the columns of the encoded matrix.
//!
//! hemera's `tree.rs` builds Merkle trees over fixed 4096-byte content
//! chunks (BAO-compatible) — the right tool for content-addressing a byte
//! stream, but it does not map onto "one leaf per column" for our matrix
//! columns (each is `k1 · 8` bytes, essentially never a multiple of 4096).
//! This module is a small, purpose-built binary Merkle tree over
//! caller-supplied leaves, reusing hemera's leaf/node hashing convention
//! (`tree::hash_leaf`, `tree::hash_node`) and its path verifier
//! (`merkle_verify_path`) so the authentication semantics are identical to
//! the rest of the stack — only the tree-building and path-extraction
//! logic (which hemera does not expose for non-chunked leaves) is new.
//!
//! The number of columns `m` is always a power of two here (m = EXPANSION
//! · k2, k2 already a power of two), so the tree is a perfect binary tree
//! — no left-balancing needed.

use cyber_hemera::tree::{hash_leaf, hash_node};
use cyber_hemera::{merkle_verify_path, Hash, Side};

/// A Merkle tree over `m` leaves (m a power of two, m ≥ 1), one per
/// encoded-matrix column.
pub struct ColumnTree {
    /// `levels[0]` are the leaf hashes, `levels[last]` is `[root]`.
    levels: Vec<Vec<Hash>>,
}

impl ColumnTree {
    /// Build the tree over column byte-strings (already serialized).
    pub fn build(columns: &[Vec<u8>]) -> Self {
        let n = columns.len();
        assert!(n.is_power_of_two(), "column count must be a power of two, got {n}");
        let is_single = n == 1;
        let leaves: Vec<Hash> = columns
            .iter()
            .enumerate()
            .map(|(i, bytes)| hash_leaf(bytes, i as u64, is_single))
            .collect();

        let mut levels = vec![leaves];
        while levels.last().expect("levels never empty").len() > 1 {
            let prev = levels.last().expect("checked non-empty above");
            let is_root = prev.len() == 2;
            let next: Vec<Hash> = prev
                .chunks_exact(2)
                .map(|pair| hash_node(&pair[0], &pair[1], is_root))
                .collect();
            levels.push(next);
        }
        Self { levels }
    }

    pub fn root(&self) -> Hash {
        self.levels.last().expect("levels never empty")[0]
    }

    /// Leaf-to-root sibling path for `index`, in the order
    /// `cyber_hemera::merkle_verify_path` expects.
    pub fn path(&self, index: usize) -> Vec<(Hash, Side)> {
        let mut path = Vec::with_capacity(self.levels.len() - 1);
        let mut idx = index;
        for level in &self.levels[..self.levels.len() - 1] {
            let sibling = level[idx ^ 1];
            let side = if idx % 2 == 0 { Side::Right } else { Side::Left };
            path.push((sibling, side));
            idx /= 2;
        }
        path
    }
}

/// Verify a column's leaf bytes authenticate against `root` via `path`.
/// `index` is the column's position (needed to reproduce `hash_leaf`'s
/// counter and single-leaf flag).
pub fn verify_column(
    root: &Hash,
    index: usize,
    num_columns: usize,
    column_bytes: &[u8],
    path: &[(Hash, Side)],
) -> bool {
    let leaf = hash_leaf(column_bytes, index as u64, num_columns == 1);
    merkle_verify_path(root, &leaf, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_column_tree_root_is_leaf() {
        let cols = vec![b"only column".to_vec()];
        let tree = ColumnTree::build(&cols);
        assert!(verify_column(&tree.root(), 0, 1, &cols[0], &tree.path(0)));
    }

    #[test]
    fn all_leaves_verify_in_power_of_two_tree() {
        let cols: Vec<Vec<u8>> = (0u8..16).map(|i| vec![i; 8]).collect();
        let tree = ColumnTree::build(&cols);
        for (i, col) in cols.iter().enumerate() {
            let path = tree.path(i);
            assert!(verify_column(&tree.root(), i, cols.len(), col, &path));
        }
    }

    #[test]
    fn wrong_column_bytes_fail() {
        let cols: Vec<Vec<u8>> = (0u8..8).map(|i| vec![i; 8]).collect();
        let tree = ColumnTree::build(&cols);
        let path = tree.path(2);
        let wrong = vec![0xFFu8; 8];
        assert!(!verify_column(&tree.root(), 2, cols.len(), &wrong, &path));
    }

    #[test]
    fn swapped_index_fails() {
        let cols: Vec<Vec<u8>> = (0u8..8).map(|i| vec![i; 8]).collect();
        let tree = ColumnTree::build(&cols);
        let path_for_2 = tree.path(2);
        // Presenting column 3's bytes with column 2's path/index must fail.
        assert!(!verify_column(&tree.root(), 2, cols.len(), &cols[3], &path_for_2));
    }

    #[test]
    fn wrong_root_fails() {
        let cols: Vec<Vec<u8>> = (0u8..8).map(|i| vec![i; 8]).collect();
        let tree = ColumnTree::build(&cols);
        let path = tree.path(0);
        let wrong_root = cyber_hemera::hash(b"not the root");
        assert!(!verify_column(&wrong_root, 0, cols.len(), &cols[0], &path));
    }

    #[test]
    fn deterministic_root() {
        let cols: Vec<Vec<u8>> = (0u8..4).map(|i| vec![i; 4]).collect();
        assert_eq!(ColumnTree::build(&cols).root(), ColumnTree::build(&cols).root());
    }
}
