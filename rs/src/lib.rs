// ---
// tags: trop, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Tropical semiring arithmetic over u64.
//!
//! (min, +) algebra where a (+) b = min(a, b) and a (*) b = a + b.
//! No additive inverse — this is a semiring, not a field.

#![no_std]

pub mod element;
pub mod matrix;
pub mod kleene;
pub mod eigenvalue;
pub mod determinant;
pub mod dual;
pub mod encoding;

#[cfg(test)]
mod vectors;

pub use element::Tropical;
pub use matrix::TropMatrix;
pub use kleene::kleene_star;
