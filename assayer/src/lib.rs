//! cyb-lens-assayer — Assayer tropical witness-verify commitment.
//!
//! The tropical semiring (min, +) has no subtraction or inversion —
//! classical polynomial commitment over trop is impossible.
//!
//! Assayer is a wrapper protocol: it accepts tropical computation,
//! produces an optimal assignment (witness) and LP dual feasibility
//! certificate, packs both as a Goldilocks polynomial, and commits
//! via Brakedown.
//!
//! Assayer does not implement the Lens trait. It delegates to Brakedown.
//!
//! See specs/tropical-semiring.md for the full specification.

pub use cyb_lens_core::{Commitment, Opening, Transcript};
