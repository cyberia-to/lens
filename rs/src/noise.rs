// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Noise budget tracking for lattice-based schemes.
//!
//! Tracks an upper bound on the log2 of the noise magnitude
//! through homomorphic operations. When the noise budget is
//! exhausted, decryption fails.

/// Tracks the log2 upper bound on noise magnitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoiseBudget {
    pub log_bound: u32,
}

impl NoiseBudget {
    /// Create a fresh noise budget with an initial bound.
    pub fn fresh(initial_bound: u32) -> Self {
        NoiseBudget { log_bound: initial_bound }
    }

    /// Noise budget after addition: max(a, b) + 1 (noise roughly doubles).
    pub fn after_add(a: &NoiseBudget, b: &NoiseBudget) -> NoiseBudget {
        let max_log = if a.log_bound > b.log_bound { a.log_bound } else { b.log_bound };
        NoiseBudget { log_bound: max_log + 1 }
    }

    /// Noise budget after multiplication.
    ///
    /// Multiplication increases noise roughly as:
    /// log(noise_product) ~ log(noise_a) + log(noise_b) + log(n)
    /// where n is the ring dimension.
    pub fn after_mul(a: &NoiseBudget, b: &NoiseBudget, n: usize) -> NoiseBudget {
        let log_n = (n as u32).trailing_zeros();
        NoiseBudget {
            log_bound: a.log_bound + b.log_bound + log_n,
        }
    }

    /// Noise budget after bootstrapping resets to a fixed noise level.
    pub fn after_bootstrap(bootstrap_noise: u32) -> NoiseBudget {
        NoiseBudget { log_bound: bootstrap_noise }
    }

    /// Check whether bootstrapping is needed.
    ///
    /// Returns true if the current noise exceeds the maximum tolerable budget.
    pub fn needs_bootstrap(budget: &NoiseBudget, max_budget: u32) -> bool {
        budget.log_bound >= max_budget
    }

    /// Remaining budget before decryption failure.
    ///
    /// Returns 0 if already exceeded.
    pub fn remaining(budget: &NoiseBudget, max_budget: u32) -> u32 {
        if budget.log_bound >= max_budget {
            0
        } else {
            max_budget - budget.log_bound
        }
    }
}
