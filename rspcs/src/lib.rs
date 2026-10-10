// ---
// tags: lens, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! cyber-lens-rspcs — hash-only multilinear polynomial commitments over
//! Goldilocks with Reed–Solomon codes, hemera Merkle trees and Fp3
//! challenges.
//!
//! Two schemes implement `cyber_lens_core::MultilinearPcs`:
//! - [`TensorRs`]: Ligero geometry (rows RS-encoded, column tree, proximity
//!   test + evaluation over shared columns), unique-decoding analysis.
//! - [`Whir`]: WHIR (Arnon, Chiesa, Fenzi, Yogev, eprint 2024/1586) with
//!   constrained folding, out-of-domain samples and grinding; parameters
//!   from proven bounds up to the Johnson radius.
//!
//! Specifications: `specs/tensor-rs.md`, `specs/whir.md`.

pub mod backend;
pub mod field;
pub mod merkle;
mod par;
pub mod pow;
pub mod rs;
pub mod soundness;
pub mod tensor;
pub mod whir;
pub mod wire;

pub use soundness::Regime;
pub use tensor::{TensorConfig, TensorRs, TensorRsData, TensorRsParams, TensorRsProof};
pub use whir::{Whir, WhirConfig, WhirData, WhirParams, WhirProof};

#[cfg(test)]
mod tests;
