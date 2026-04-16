//! cyb-lens-ikat — Ikat polynomial commitment.
//!
//! NTT-batched Brakedown over jali's R_q = F_p[x]/(x^n+1).
//! Commits NTT evaluation slots (Goldilocks scalars) via batched
//! expander-graph encoding. Ring structure enables NTT batching,
//! automorphism arguments, and noise tracking.
//!
//! See specs/polynomial-ring.md for the full specification.

pub use cyb_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};
