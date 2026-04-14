//! WGSL compute shader sources for tropical semiring operations.

/// Tropical matrix multiplication shader (tiled 16x16 compute).
pub const TROPICAL_MATMUL: &str = include_str!("tropical.wgsl");
