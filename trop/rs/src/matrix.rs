// ---
// tags: trop, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Tropical matrix over fixed-size storage.
//!
//! Square matrices up to `MAX_DIM x MAX_DIM` stored in a flat array.
//! All operations use (min, +) semiring arithmetic.

use crate::element::Tropical;

/// Maximum supported matrix dimension.
pub const MAX_DIM: usize = 64;

/// Total storage slots.
const MAX_ENTRIES: usize = MAX_DIM * MAX_DIM;

/// A square tropical matrix with runtime dimension `n`.
#[derive(Clone)]
pub struct TropMatrix {
    /// Actual dimension (n x n).
    pub n: usize,
    /// Row-major flat storage; only the first n*n entries are meaningful.
    pub data: [Tropical; MAX_ENTRIES],
}

impl TropMatrix {
    /// Create an n x n matrix filled with INF (tropical zero matrix).
    ///
    /// # Panics
    /// Panics if `n > MAX_DIM`.
    pub fn new(n: usize) -> Self {
        assert!(n <= MAX_DIM, "dimension exceeds MAX_DIM");
        TropMatrix {
            n,
            data: [Tropical::INF; MAX_ENTRIES],
        }
    }

    /// Create an n x n tropical identity matrix.
    ///
    /// Diagonal entries are 0 (tropical one), off-diagonal entries are INF.
    pub fn identity(n: usize) -> Self {
        let mut m = Self::new(n);
        for i in 0..n {
            m.data[i * MAX_DIM + i] = Tropical::ONE;
        }
        m
    }

    /// Get the element at (i, j).
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> Tropical {
        debug_assert!(i < self.n && j < self.n);
        self.data[i * MAX_DIM + j]
    }

    /// Set the element at (i, j).
    #[inline]
    pub fn set(&mut self, i: usize, j: usize, val: Tropical) {
        debug_assert!(i < self.n && j < self.n);
        self.data[i * MAX_DIM + j] = val;
    }

    /// Tropical matrix addition: elementwise min.
    pub fn add(&self, other: &TropMatrix) -> TropMatrix {
        assert_eq!(self.n, other.n, "dimension mismatch");
        let n = self.n;
        let mut result = TropMatrix::new(n);
        for i in 0..n {
            for j in 0..n {
                let idx = i * MAX_DIM + j;
                result.data[idx] = self.data[idx].add(other.data[idx]);
            }
        }
        result
    }

    /// Tropical matrix multiplication.
    ///
    /// C\[i\]\[j\] = min_k (A\[i\]\[k\] + B\[k\]\[j\])
    pub fn mul(&self, other: &TropMatrix) -> TropMatrix {
        assert_eq!(self.n, other.n, "dimension mismatch");
        let n = self.n;
        let mut result = TropMatrix::new(n);
        for i in 0..n {
            for k in 0..n {
                let a_ik = self.data[i * MAX_DIM + k];
                if a_ik.is_inf() {
                    continue;
                }
                for j in 0..n {
                    let b_kj = other.data[k * MAX_DIM + j];
                    let product = a_ik.mul(b_kj);
                    let idx = i * MAX_DIM + j;
                    result.data[idx] = result.data[idx].add(product);
                }
            }
        }
        result
    }

    /// Tropical matrix exponentiation by repeated squaring.
    ///
    /// Computes A^k under tropical multiplication.
    /// A^0 = identity matrix.
    pub fn power(&self, mut k: u64) -> TropMatrix {
        let n = self.n;
        let mut result = TropMatrix::identity(n);
        let mut base = self.clone();
        while k > 0 {
            if k & 1 == 1 {
                result = result.mul(&base);
            }
            base = base.mul(&base);
            k >>= 1;
        }
        result
    }

    /// Build a matrix from a weighted edge list.
    ///
    /// Each tuple (i, j, w) sets entry (i, j) to the minimum of w and any
    /// previously inserted weight (supports multigraphs).
    pub fn from_weights(n: usize, weights: &[(usize, usize, u64)]) -> TropMatrix {
        let mut m = TropMatrix::new(n);
        for &(i, j, w) in weights {
            let current = m.get(i, j);
            let candidate = Tropical::from_u64(w);
            m.set(i, j, current.add(candidate));
        }
        m
    }
}

impl core::fmt::Debug for TropMatrix {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "TropMatrix({}x{}) [", self.n, self.n)?;
        for i in 0..self.n {
            write!(f, "  [")?;
            for j in 0..self.n {
                if j > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", self.get(i, j))?;
            }
            writeln!(f, "]")?;
        }
        write!(f, "]")
    }
}
