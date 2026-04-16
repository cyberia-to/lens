// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! jali — polynomial ring arithmetic: R_q = F_p[x]/(x^n+1)
//! the fifth algebra for provable intelligence

#![no_std]

pub mod encoding;
pub mod noise;
pub mod ntt;
pub mod ring;
pub mod sample;

#[cfg(test)]
mod vectors;

pub use noise::NoiseBudget;
pub use ring::RingElement;
