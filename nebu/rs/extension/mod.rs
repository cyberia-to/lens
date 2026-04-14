// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Extension fields over Goldilocks.
//!
//! - Fp2: F_{p²} = F_p[u] / (u² − 7)       — 128-bit security
//! - Fp3: F_{p³} = F_p[t] / (t³ − t − 1)   — recursive composition
//! - Fp4: F_{p⁴} = F_p[w] / (w⁴ − 7)       — 256-bit security, recursion tower

pub mod fp2;
pub mod fp3;
pub mod fp4;

pub use fp2::Fp2;
pub use fp3::Fp3;
pub use fp4::Fp4;
