//! Hemera's constants as canonical limbs, from hemera itself (the shader
//! sources are generated from these; nothing is copied by hand).

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Round constants in hemera's order (0.4): 192 external — 6 initial full
/// rounds then 6 terminal, 16 each — then 48 internal (index 192 + r for
/// partial round r).
pub fn round_constants() -> [u64; cyber_hemera::constants::NUM_CONSTANTS] {
    core::array::from_fn(|i| cyber_hemera::constants::ROUND_CONSTANTS_U64[i] % P)
}

/// Index of the first internal (partial-round) constant.
pub const INTERNAL: usize = cyber_hemera::constants::NUM_EXTERNAL;

/// The internal diffusion diagonal (`M_I = 1 + diag(d)`).
pub fn diag() -> [u64; 16] {
    core::array::from_fn(|i| cyber_hemera::field::MATRIX_DIAG_16[i].as_canonical_u64())
}

#[cfg(test)]
mod tests {
    /// The shaders hard-code hemera 0.4's round structure.
    #[test]
    fn shape_is_hemera_0_4() {
        use cyber_hemera::{ROUNDS_F, ROUNDS_P, WIDTH};
        assert_eq!((ROUNDS_F, ROUNDS_P, WIDTH), (12, 48, 16));
        assert_eq!(super::INTERNAL, 192);
        assert_eq!(super::round_constants().len(), 240);
    }
}
