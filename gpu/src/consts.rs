//! Hemera's constants as canonical limbs, from hemera itself (the shader
//! sources are generated from these; nothing is copied by hand).

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// 128 external (4 initial rounds, then 4 terminal) and 16 internal round
/// constants, in hemera's order.
pub fn round_constants() -> [u64; 144] {
    core::array::from_fn(|i| cyber_hemera::constants::ROUND_CONSTANTS_U64[i] % P)
}

/// The internal diffusion diagonal (`M_I = 1 + diag(d)`).
pub fn diag() -> [u64; 16] {
    core::array::from_fn(|i| cyber_hemera::field::MATRIX_DIAG_16[i].as_canonical_u64())
}
