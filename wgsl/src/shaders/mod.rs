//! WGSL shader sources for ring arithmetic on GPU.
//!
//! jali ring shaders depend on nebu's Goldilocks field and NTT shaders.
//! At load time, prepend nebu's field.wgsl and ntt.wgsl before ring.wgsl:
//!
//! ```ignore
//! let source = format!("{}\n{}\n{}", nebu_wgsl::shaders::FIELD, nebu_wgsl::shaders::NTT, jali_wgsl::shaders::RING);
//! ```

/// Ring-level operations: coefficient-wise add/sub/neg, pointwise mul, twist/untwist.
/// Requires nebu field.wgsl (Fp type, fp_add, fp_sub, fp_mul, fp_neg) and
/// nebu ntt.wgsl (butterfly operations) to be prepended at load time.
pub const RING: &str = include_str!("ring.wgsl");
