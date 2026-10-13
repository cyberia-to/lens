//! Proof-of-work grinding on the Fiat–Shamir transcript (hemera).
//!
//! Before a verifier challenge with `bits > 0` both sides squeeze a
//! 32-byte seed `c` from the transcript. The prover finds the smallest
//! `nonce` such that the first limb of `hemera::hash(c ‖ nonce_le)` has its
//! `bits` low bits zero; the nonce is sent and absorbed. With `bits = 0`
//! nothing is squeezed, sent or absorbed. A cheating prover must spend
//! about `2^bits` hemera evaluations per attempt at the next challenge,
//! which adds `bits` to that round's soundness in the usual
//! hash-query accounting (grinding as in the ethSTARK documentation,
//! eprint 2021/582).

use cyber_lens_core::Transcript;

fn ok(seed: &[u8; 32], nonce: u64, bits: u32) -> bool {
    let mut input = [0u8; 40];
    input[..32].copy_from_slice(seed);
    input[32..].copy_from_slice(&nonce.to_le_bytes());
    let h = cyber_hemera::hash(&input);
    let limb = u64::from_le_bytes(h.as_bytes()[..8].try_into().expect("8 bytes"));
    limb & ((1u64 << bits) - 1) == 0
}

/// Grind and absorb; returns the nonce (`None` when `bits = 0`). The
/// search runs on the process's [`crate::backend`]; every backend returns
/// the least passing nonce.
pub fn grind(t: &mut Transcript, bits: u32) -> Option<u64> {
    if bits == 0 {
        return None;
    }
    assert!(bits < 64);
    let seed = *t.squeeze().as_bytes();
    let template = crate::backend::hash40_template(&seed);
    let nonce = crate::backend::current().grind(&template, crate::backend::Inject::hash40(), bits);
    debug_assert!(ok(&seed, nonce, bits));
    t.absorb_u64(nonce);
    Some(nonce)
}

/// Check and absorb a nonce.
pub fn check(t: &mut Transcript, bits: u32, nonce: u64) -> bool {
    if bits == 0 {
        return true;
    }
    let seed = *t.squeeze().as_bytes();
    if !ok(&seed, nonce, bits) {
        return false;
    }
    t.absorb_u64(nonce);
    true
}
