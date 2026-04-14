// ---
// tags: trop, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Tropical semiring element.
//!
//! The tropical semiring (min, +) over u64:
//!   - Tropical addition: a + b = min(a, b)
//!   - Tropical multiplication: a * b = a + b (ordinary addition)
//!   - Additive identity (ZERO): +inf (u64::MAX)
//!   - Multiplicative identity (ONE): 0

/// A tropical semiring element wrapping a `u64`.
///
/// `u64::MAX` represents +inf (tropical zero, the additive identity).
/// `0` represents tropical one (the multiplicative identity).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tropical(pub u64);

impl Tropical {
    /// Tropical zero: +inf, the identity element for min.
    pub const ZERO: Tropical = Tropical(u64::MAX);

    /// Tropical one: 0, the identity element for addition.
    pub const ONE: Tropical = Tropical(0);

    /// Alias for ZERO: positive infinity.
    pub const INF: Tropical = Tropical(u64::MAX);

    /// Construct from a raw u64. `u64::MAX` is interpreted as infinity.
    #[inline]
    pub const fn from_u64(v: u64) -> Self {
        Tropical(v)
    }

    /// Extract the inner u64 value.
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Check whether this element is +inf (tropical zero).
    #[inline]
    pub const fn is_inf(self) -> bool {
        self.0 == u64::MAX
    }

    /// Check whether this element is finite (not +inf).
    #[inline]
    pub const fn is_finite(self) -> bool {
        self.0 != u64::MAX
    }

    /// Tropical addition: min(a, b).
    ///
    /// If either operand is INF, returns the other.
    #[inline]
    pub const fn add(self, other: Tropical) -> Tropical {
        if self.0 <= other.0 {
            self
        } else {
            other
        }
    }

    /// Tropical multiplication: a + b (saturating).
    ///
    /// If either operand is INF, or if the sum overflows, returns INF.
    #[inline]
    pub const fn mul(self, other: Tropical) -> Tropical {
        if self.is_inf() || other.is_inf() {
            Tropical::INF
        } else {
            let (sum, overflow) = self.0.overflowing_add(other.0);
            if overflow || sum == u64::MAX {
                Tropical::INF
            } else {
                Tropical(sum)
            }
        }
    }
}

/// Display a tropical element: finite values print as integers, infinity as "+inf".
impl core::fmt::Display for Tropical {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_inf() {
            write!(f, "+inf")
        } else {
            write!(f, "{}", self.0)
        }
    }
}
