//! cyb-lens-porphyry — Porphyry polynomial commitment.
//!
//! Brakedown instantiated over genies' F_q (CSIDH-512 prime, 512 bits).
//! Same expander-graph structure, wider field elements (64 bytes each).
//! Privacy workloads: CSIDH key exchange, VDF, blind/ring signatures.
//!
//! See specs/isogeny-curves.md for the full specification.

pub use cyb_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};
