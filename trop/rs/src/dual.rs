// ---
// tags: trop, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Max-plus (dual tropical) semiring.
//!
//! The max-plus semiring (max, +):
//!   - Addition: a + b = max(a, b)
//!   - Multiplication: a * b = a + b (ordinary)
//!   - Additive identity (ZERO): 0 representing -inf
//!   - Multiplicative identity (ONE): 1 representing 0
//!
//! Encoding: `MaxPlus(0)` = -inf (identity for max). Values 1..u64::MAX
//! represent the integers 0..(u64::MAX - 2) via a +1 offset.

/// A max-plus semiring element.
///
/// `MaxPlus(0)` is -inf (additive identity).
/// `MaxPlus(v)` for v >= 1 represents the value (v - 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaxPlus(pub u64);

impl MaxPlus {
    /// The additive identity: -inf (identity for max).
    pub const ZERO: MaxPlus = MaxPlus(0);

    /// The multiplicative identity: represents the value 0.
    pub const ONE: MaxPlus = MaxPlus(1);

    /// Alias for ZERO: negative infinity.
    pub const NEG_INF: MaxPlus = MaxPlus(0);

    /// Check whether this is -inf.
    #[inline]
    pub const fn is_neg_inf(self) -> bool {
        self.0 == 0
    }

    /// Check whether this is finite.
    #[inline]
    pub const fn is_finite(self) -> bool {
        self.0 != 0
    }

    /// Create a MaxPlus element representing the value `v`.
    /// Panics if v == u64::MAX (reserved).
    #[inline]
    pub const fn from_val(v: u64) -> Self {
        assert!(v < u64::MAX, "MaxPlus: value too large");
        MaxPlus(v + 1)
    }

    /// Extract the represented value. Returns `None` if -inf.
    #[inline]
    pub const fn to_val(self) -> Option<u64> {
        if self.0 == 0 { None } else { Some(self.0 - 1) }
    }

    /// Max-plus addition: max(a, b).
    #[inline]
    pub const fn add(self, other: MaxPlus) -> MaxPlus {
        if self.0 >= other.0 { self } else { other }
    }

    /// Max-plus multiplication: a + b (saturating), with -inf absorbing.
    #[inline]
    pub const fn mul(self, other: MaxPlus) -> MaxPlus {
        if self.is_neg_inf() || other.is_neg_inf() {
            MaxPlus::ZERO
        } else {
            // self.0 - 1 + other.0 - 1 => self.0 + other.0 - 2, then +1 for encoding
            // = self.0 + other.0 - 1
            let (sum, overflow) = self.0.overflowing_add(other.0);
            if overflow {
                // Saturate to maximum representable value
                MaxPlus(u64::MAX)
            } else {
                MaxPlus(sum - 1)
            }
        }
    }
}

impl core::fmt::Display for MaxPlus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_neg_inf() {
            write!(f, "-inf")
        } else {
            write!(f, "{}", self.0 - 1)
        }
    }
}
