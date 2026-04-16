// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! nebu — Goldilocks prime field library.

#![no_std]

pub mod batch;
pub mod encoding;
pub mod extension;
pub mod field;
pub mod ntt;
pub mod sqrt;

#[cfg(test)]
mod vectors;

pub use extension::{Fp2, Fp3, Fp4};
pub use field::Goldilocks;
