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

pub mod fq;
pub mod curve;
pub mod isogeny;
pub mod action;
pub mod encoding;

#[cfg(test)]
mod vectors;

pub use fq::Fq;
pub use curve::{MontCurve, MontPoint};
