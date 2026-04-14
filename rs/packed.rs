//! Packed F₂ operations — 128 elements per u128
//!
//! SIMD-style batch operations where one machine instruction
//! operates on 128 binary field elements simultaneously.

use crate::tower::F2_128;

/// 128 F₂ elements packed in one u128.
/// XOR = vectorized addition. AND = vectorized multiplication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Packed128(pub u128);

impl Packed128 {
    pub const ZERO: Self = Packed128(0);
    pub const ONES: Self = Packed128(u128::MAX);

    /// Vectorized addition: 128 parallel XOR operations.
    #[inline(always)]
    pub fn add(self, rhs: Self) -> Self { Packed128(self.0 ^ rhs.0) }

    /// Vectorized multiplication: 128 parallel AND operations.
    #[inline(always)]
    pub fn mul(self, rhs: Self) -> Self { Packed128(self.0 & rhs.0) }

    /// Vectorized NOT: 128 parallel complement operations.
    #[inline(always)]
    pub fn not(self) -> Self { Packed128(!self.0) }

    /// Popcount: number of 1-elements in the packed vector.
    #[inline(always)]
    pub fn popcount(self) -> u32 { self.0.count_ones() }

    /// Inner product: popcount(a AND b).
    /// The binary matrix-vector multiply kernel.
    #[inline(always)]
    pub fn inner_product(self, rhs: Self) -> u32 {
        (self.0 & rhs.0).count_ones()
    }

    /// Shift left by n positions.
    #[inline(always)]
    pub fn shl(self, n: u32) -> Self { Packed128(self.0 << n) }

    /// Shift right by n positions.
    #[inline(always)]
    pub fn shr(self, n: u32) -> Self { Packed128(self.0 >> n) }

    /// Get a single bit at position i.
    #[inline(always)]
    pub fn get_bit(self, i: u32) -> u8 { ((self.0 >> i) & 1) as u8 }

    /// Set a single bit at position i.
    #[inline(always)]
    pub fn set_bit(self, i: u32) -> Self { Packed128(self.0 | (1u128 << i)) }

    /// Clear a single bit at position i.
    #[inline(always)]
    pub fn clear_bit(self, i: u32) -> Self { Packed128(self.0 & !(1u128 << i)) }

    /// Reinterpret as F₂¹²⁸ tower element.
    #[inline(always)]
    pub fn as_tower(self) -> F2_128 { F2_128(self.0) }

    /// Create from an F₂¹²⁸ tower element.
    #[inline(always)]
    pub fn from_tower(t: F2_128) -> Self { Packed128(t.0) }

    /// Hamming distance between two packed vectors.
    #[inline(always)]
    pub fn hamming_distance(self, rhs: Self) -> u32 {
        (self.0 ^ rhs.0).count_ones()
    }

    /// Rotate left by n positions within 128 bits.
    #[inline(always)]
    pub fn rotl(self, n: u32) -> Self {
        let n = n & 127;
        Packed128((self.0 << n) | (self.0 >> (128 - n)))
    }

    /// Rotate right by n positions within 128 bits.
    #[inline(always)]
    pub fn rotr(self, n: u32) -> Self {
        let n = n & 127;
        Packed128((self.0 >> n) | (self.0 << (128 - n)))
    }
}
