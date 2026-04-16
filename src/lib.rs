// ---
// tags: lens, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! cyber-lens — polynomial commitment: five lenses for five algebras.
//!
//! This facade crate re-exports the core trait and all five constructions.
//! For minimal dependencies, depend on `cyb-lens-core` (trait only) or
//! a specific construction crate.

// core: trait, types, transcript
pub use cyb_lens_core::*;

// constructions
pub use cyb_lens_assayer as assayer;
pub use cyb_lens_binius as binius;
pub use cyb_lens_brakedown as brakedown;
pub use cyb_lens_ikat as ikat;
pub use cyb_lens_porphyry as porphyry;

#[cfg(test)]
mod integration_tests;
