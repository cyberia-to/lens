// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! nebu — Goldilocks prime field library.

#![no_std]

pub mod field;
pub mod encoding;
pub mod ntt;
pub mod sqrt;
pub mod batch;
pub mod extension;

#[cfg(test)]
mod vectors;

pub use field::Goldilocks;
pub use extension::{Fp2, Fp3, Fp4};
