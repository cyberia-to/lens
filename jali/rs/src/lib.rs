// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! jali — polynomial ring arithmetic: R_q = F_p[x]/(x^n+1)
//! the fifth algebra for provable intelligence

#![no_std]

pub mod ring;
pub mod ntt;
pub mod noise;
pub mod sample;
pub mod encoding;

#[cfg(test)]
mod vectors;

pub use ring::RingElement;
pub use noise::NoiseBudget;
