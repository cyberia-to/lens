//! cyb-lens-binius — Binius polynomial commitment.
//!
//! Binary Reed-Solomon over kuro's F₂ tower (F₂ → F₂¹²⁸).
//! Binary-native: AND/XOR = 1 constraint each.
//!
//! See specs/binary-tower.md for the full specification.

pub use cyb_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};
