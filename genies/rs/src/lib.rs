// ---
// tags: genies, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Commutative group action over supersingular isogenies.
//!
//! Post-quantum privacy primitives: stealth addresses, VRF, threshold protocols,
//! blind signatures, anonymous credentials. The one module with a foreign prime.

#![no_std]

pub mod action;
pub mod curve;
pub mod encoding;
pub mod fq;
pub mod isogeny;
pub mod ops;

#[cfg(test)]
mod vectors;

pub use curve::{MontCurve, MontPoint};
pub use fq::Fq;
