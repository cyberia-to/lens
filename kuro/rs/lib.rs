//! kuro (黒) — F₂ tower field arithmetic
//!
//! Binary field tower: F₂ → F₂² → F₂⁴ → F₂⁸ → F₂¹⁶ → F₂³² → F₂⁶⁴ → F₂¹²⁸
//!
//! Each extension defined by x² + x + α where α ∈ previous level.
//! Tower structure enables:
//! - 128 F₂ elements packed in one u128 machine word
//! - SIMD-native operations (64× data parallelism vs Goldilocks)
//! - Karatsuba multiplication over tower levels
//!
//! No hemera dependency. No nebu dependency. Pure binary algebra.
//! kuro is to F₂ what nebu is to Goldilocks.

#![no_std]
#![allow(clippy::should_implement_trait)]

pub mod batch;
pub mod encoding;
pub mod inv;
pub mod packed;
pub mod tower;

#[cfg(test)]
mod vectors;

pub use packed::Packed128;
pub use tower::{F2, F2_2, F2_4, F2_8, F2_16, F2_32, F2_64, F2_128};
