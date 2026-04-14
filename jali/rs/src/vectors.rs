// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Test vectors for jali ring arithmetic.

#[cfg(test)]
mod tests {
    use nebu::Goldilocks;
    use nebu::field::P;
    use crate::ring::RingElement;
    use crate::ntt;
    use crate::noise::NoiseBudget;
    use crate::sample;
    use crate::encoding;

    fn g(v: u64) -> Goldilocks { Goldilocks::new(v) }

    // ── Ring: zero element ───────────────────────────────────────────

    #[test]
    fn ring_zero_is_zero() {
        let z = RingElement::new(1024);
        assert!(z.is_zero());
    }

    #[test]
    fn ring_zero_add_zero() {
        let z1 = RingElement::new(1024);
        let z2 = RingElement::new(1024);
        let sum = z1.add(&z2);
        assert!(sum.is_zero());
    }

    // ── Ring: add ────────────────────────────────────────────────────

    #[test]
    fn ring_add_basic() {
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        a.coeffs[0] = g(1);
        a.coeffs[1] = g(2);
        b.coeffs[0] = g(3);
        b.coeffs[1] = g(4);
        let c = a.add(&b);
        assert_eq!(c.coeffs[0].as_u64(), 4);
        assert_eq!(c.coeffs[1].as_u64(), 6);
        for i in 2..n {
            assert!(c.coeffs[i].is_zero());
        }
    }

    #[test]
    fn ring_add_wraparound() {
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        a.coeffs[0] = Goldilocks::NEG_ONE; // p-1
        b.coeffs[0] = g(2);
        let c = a.add(&b);
        assert_eq!(c.coeffs[0].as_u64(), 1); // (p-1)+2 = p+1 = 1
    }

    #[test]
    fn ring_add_identity() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        a.coeffs[3] = g(100);
        let z = RingElement::new(n);
        let c = a.add(&z);
        assert!(c.eq_ring(&a));
    }

    #[test]
    fn ring_add_commutative() {
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        a.coeffs[0] = g(7);
        a.coeffs[1] = g(11);
        b.coeffs[0] = g(13);
        b.coeffs[1] = g(17);
        let ab = a.add(&b);
        let ba = b.add(&a);
        assert!(ab.eq_ring(&ba));
    }

    #[test]
    fn ring_add_associative() {
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        let mut c = RingElement::new(n);
        a.coeffs[0] = g(100);
        b.coeffs[0] = g(200);
        c.coeffs[0] = g(300);
        let ab_c = a.add(&b).add(&c);
        let a_bc = a.add(&b.add(&c));
        assert!(ab_c.eq_ring(&a_bc));
    }

    // ── Ring: sub ────────────────────────────────────────────────────

    #[test]
    fn ring_sub_basic() {
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        a.coeffs[0] = g(10);
        b.coeffs[0] = g(3);
        let c = a.sub(&b);
        assert_eq!(c.coeffs[0].as_u64(), 7);
    }

    #[test]
    fn ring_sub_self_is_zero() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        a.coeffs[1] = g(99);
        let c = a.sub(&a);
        assert!(c.is_zero());
    }

    #[test]
    fn ring_sub_wraparound() {
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        a.coeffs[0] = g(0);
        b.coeffs[0] = g(1);
        let c = a.sub(&b);
        assert_eq!(c.coeffs[0].as_u64(), P - 1); // 0 - 1 = -1 = p-1
    }

    // ── Ring: neg ────────────────────────────────────────────────────

    #[test]
    fn ring_neg_basic() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(1);
        let neg_a = a.neg();
        assert_eq!(neg_a.coeffs[0].as_u64(), P - 1);
    }

    #[test]
    fn ring_neg_zero() {
        let z = RingElement::new(8);
        let neg_z = z.neg();
        assert!(neg_z.is_zero());
    }

    #[test]
    fn ring_double_neg() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        a.coeffs[1] = g(99);
        let nn = a.neg().neg();
        assert!(nn.eq_ring(&a));
    }

    #[test]
    fn ring_add_neg_is_zero() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        a.coeffs[1] = g(99);
        let c = a.add(&a.neg());
        assert!(c.is_zero());
    }

    // ── Ring: scalar_mul ─────────────────────────────────────────────

    #[test]
    fn ring_scalar_mul_basic() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(3);
        a.coeffs[1] = g(5);
        let c = a.scalar_mul(g(7));
        assert_eq!(c.coeffs[0].as_u64(), 21);
        assert_eq!(c.coeffs[1].as_u64(), 35);
    }

    #[test]
    fn ring_scalar_mul_zero() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        let c = a.scalar_mul(Goldilocks::ZERO);
        assert!(c.is_zero());
    }

    #[test]
    fn ring_scalar_mul_one() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        a.coeffs[1] = g(99);
        let c = a.scalar_mul(Goldilocks::ONE);
        assert!(c.eq_ring(&a));
    }

    // ── Ring: mul (polynomial multiplication via NTT) ────────────────

    #[test]
    fn ring_mul_one_times_x() {
        // 1 * x = x in R_q
        let n = 8;
        let mut one_poly = RingElement::new(n);
        one_poly.coeffs[0] = g(1); // polynomial "1"
        let mut x_poly = RingElement::new(n);
        x_poly.coeffs[1] = g(1); // polynomial "x"
        let result = one_poly.mul(&x_poly);
        assert_eq!(result.coeffs[0].as_u64(), 0);
        assert_eq!(result.coeffs[1].as_u64(), 1);
        for i in 2..n {
            assert!(result.coeffs[i].is_zero());
        }
    }

    #[test]
    fn ring_mul_x_times_x() {
        // x * x = x^2
        let n = 8;
        let mut x_poly = RingElement::new(n);
        x_poly.coeffs[1] = g(1);
        let result = x_poly.mul(&x_poly);
        assert_eq!(result.coeffs[2].as_u64(), 1); // x^2
        for i in 0..n {
            if i != 2 {
                assert!(result.coeffs[i].is_zero());
            }
        }
    }

    #[test]
    fn ring_mul_negacyclic_wraparound() {
        // In R_q = F_p[x]/(x^n+1), x^n = -1
        // So x^(n-1) * x = x^n = -1
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[n - 1] = g(1); // x^(n-1)
        let mut b = RingElement::new(n);
        b.coeffs[1] = g(1); // x
        let result = a.mul(&b);
        // x^(n-1) * x = x^n = -1 mod (x^n+1)
        assert_eq!(result.coeffs[0].as_u64(), P - 1); // -1
        for i in 1..n {
            assert!(result.coeffs[i].is_zero());
        }
    }

    #[test]
    fn ring_mul_commutative() {
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        a.coeffs[0] = g(3);
        a.coeffs[1] = g(5);
        b.coeffs[0] = g(7);
        b.coeffs[2] = g(2);
        let ab = a.mul(&b);
        let ba = b.mul(&a);
        assert!(ab.eq_ring(&ba));
    }

    #[test]
    fn ring_mul_distributive() {
        // a * (b + c) = a*b + a*c
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        let mut c = RingElement::new(n);
        a.coeffs[0] = g(2);
        a.coeffs[1] = g(3);
        b.coeffs[0] = g(5);
        b.coeffs[1] = g(7);
        c.coeffs[0] = g(11);
        c.coeffs[2] = g(13);
        let lhs = a.mul(&b.add(&c));
        let rhs = a.mul(&b).add(&a.mul(&c));
        assert!(lhs.eq_ring(&rhs));
    }

    #[test]
    fn ring_mul_zero() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        let z = RingElement::new(n);
        let result = a.mul(&z);
        assert!(result.is_zero());
    }

    #[test]
    fn ring_mul_identity() {
        // 1 * a = a
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(3);
        a.coeffs[1] = g(5);
        a.coeffs[2] = g(7);
        let mut one = RingElement::new(n);
        one.coeffs[0] = g(1);
        let result = a.mul(&one);
        assert!(result.eq_ring(&a));
    }

    #[test]
    fn ring_mul_constant_times_poly() {
        // (c) * (a0 + a1*x) = c*a0 + c*a1*x
        let n = 8;
        let mut c_poly = RingElement::new(n);
        c_poly.coeffs[0] = g(5);
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(3);
        a.coeffs[1] = g(7);
        let result = c_poly.mul(&a);
        assert_eq!(result.coeffs[0].as_u64(), 15);
        assert_eq!(result.coeffs[1].as_u64(), 35);
        for i in 2..n {
            assert!(result.coeffs[i].is_zero());
        }
    }

    #[test]
    fn ring_mul_associative() {
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        let mut c = RingElement::new(n);
        a.coeffs[0] = g(2);
        a.coeffs[1] = g(1);
        b.coeffs[0] = g(3);
        b.coeffs[1] = g(1);
        c.coeffs[0] = g(5);
        c.coeffs[1] = g(1);
        let ab_c = a.mul(&b).mul(&c);
        let a_bc = a.mul(&b.mul(&c));
        assert!(ab_c.eq_ring(&a_bc));
    }

    // ── Ring: larger dimension ───────────────────────────────────────

    #[test]
    fn ring_mul_n1024_negacyclic() {
        let n = 1024;
        let mut a = RingElement::new(n);
        a.coeffs[n - 1] = g(1); // x^(n-1)
        let mut b = RingElement::new(n);
        b.coeffs[1] = g(1); // x
        let result = a.mul(&b);
        assert_eq!(result.coeffs[0].as_u64(), P - 1); // x^n = -1
        for i in 1..n {
            assert!(result.coeffs[i].is_zero());
        }
    }

    // ── NTT: forward/inverse roundtrip ──────────────────────────────

    #[test]
    fn ntt_roundtrip_small() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(1);
        a.coeffs[1] = g(2);
        a.coeffs[2] = g(3);
        let orig_coeffs: [u64; 8] = core::array::from_fn(|i| a.coeffs[i].as_u64());

        ntt::to_ntt(&mut a);
        assert!(a.is_ntt);
        ntt::from_ntt(&mut a);
        assert!(!a.is_ntt);

        for i in 0..n {
            assert_eq!(a.coeffs[i].as_u64(), orig_coeffs[i], "mismatch at index {}", i);
        }
    }

    #[test]
    fn ntt_roundtrip_n1024() {
        let n = 1024;
        let a = sample::sample_uniform(0xCAFE, n);
        let orig: [u64; 1024] = core::array::from_fn(|i| a.coeffs[i].as_u64());

        let mut b = a.clone();
        ntt::to_ntt(&mut b);
        ntt::from_ntt(&mut b);

        for i in 0..n {
            assert_eq!(b.coeffs[i].as_u64(), orig[i], "mismatch at index {}", i);
        }
    }

    #[test]
    fn ntt_roundtrip_all_ones() {
        let n = 8;
        let coeffs = [Goldilocks::ONE; 8];
        let a = RingElement::from_coeffs(&coeffs, n);
        let mut b = a.clone();
        ntt::to_ntt(&mut b);
        ntt::from_ntt(&mut b);
        assert!(b.eq_ring(&a));
    }

    #[test]
    fn ntt_negacyclic_property() {
        // Verify x^n = -1 in R_q by checking that NTT multiplication
        // of x^(n/2) * x^(n/2) = x^n = -1
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[n / 2] = g(1); // x^(n/2)
        let result = a.mul(&a); // x^n = -1
        assert_eq!(result.coeffs[0].as_u64(), P - 1);
        for i in 1..n {
            assert!(result.coeffs[i].is_zero());
        }
    }

    #[test]
    fn ntt_preserves_zero() {
        let n = 8;
        let mut z = RingElement::new(n);
        ntt::to_ntt(&mut z);
        for i in 0..n {
            assert!(z.coeffs[i].is_zero());
        }
    }

    #[test]
    fn ntt_linearity() {
        // NTT(a + b) = NTT(a) + NTT(b) (since NTT is a linear transform)
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        a.coeffs[0] = g(3);
        a.coeffs[1] = g(5);
        b.coeffs[0] = g(7);
        b.coeffs[2] = g(11);

        let sum = a.add(&b);
        let mut ntt_sum = sum.clone();
        ntt::to_ntt(&mut ntt_sum);

        let mut ntt_a = a.clone();
        let mut ntt_b = b.clone();
        ntt::to_ntt(&mut ntt_a);
        ntt::to_ntt(&mut ntt_b);
        let ntt_add = ntt_a.add(&ntt_b);

        assert!(ntt_sum.eq_ring(&ntt_add));
    }

    // ── Noise budget ─────────────────────────────────────────────────

    #[test]
    fn noise_fresh() {
        let nb = NoiseBudget::fresh(10);
        assert_eq!(nb.log_bound, 10);
    }

    #[test]
    fn noise_after_add() {
        let a = NoiseBudget::fresh(10);
        let b = NoiseBudget::fresh(12);
        let c = NoiseBudget::after_add(&a, &b);
        assert_eq!(c.log_bound, 13); // max(10,12) + 1
    }

    #[test]
    fn noise_after_add_equal() {
        let a = NoiseBudget::fresh(10);
        let b = NoiseBudget::fresh(10);
        let c = NoiseBudget::after_add(&a, &b);
        assert_eq!(c.log_bound, 11); // 10 + 1
    }

    #[test]
    fn noise_after_mul() {
        let a = NoiseBudget::fresh(10);
        let b = NoiseBudget::fresh(12);
        let c = NoiseBudget::after_mul(&a, &b, 1024);
        // 10 + 12 + log2(1024) = 10 + 12 + 10 = 32
        assert_eq!(c.log_bound, 32);
    }

    #[test]
    fn noise_after_mul_n4096() {
        let a = NoiseBudget::fresh(5);
        let b = NoiseBudget::fresh(5);
        let c = NoiseBudget::after_mul(&a, &b, 4096);
        // 5 + 5 + 12 = 22
        assert_eq!(c.log_bound, 22);
    }

    #[test]
    fn noise_after_bootstrap() {
        let nb = NoiseBudget::after_bootstrap(8);
        assert_eq!(nb.log_bound, 8);
    }

    #[test]
    fn noise_needs_bootstrap_yes() {
        let nb = NoiseBudget::fresh(50);
        assert!(NoiseBudget::needs_bootstrap(&nb, 40));
    }

    #[test]
    fn noise_needs_bootstrap_no() {
        let nb = NoiseBudget::fresh(20);
        assert!(!NoiseBudget::needs_bootstrap(&nb, 40));
    }

    #[test]
    fn noise_needs_bootstrap_boundary() {
        let nb = NoiseBudget::fresh(40);
        assert!(NoiseBudget::needs_bootstrap(&nb, 40));
    }

    #[test]
    fn noise_remaining() {
        let nb = NoiseBudget::fresh(20);
        assert_eq!(NoiseBudget::remaining(&nb, 50), 30);
    }

    #[test]
    fn noise_remaining_zero() {
        let nb = NoiseBudget::fresh(50);
        assert_eq!(NoiseBudget::remaining(&nb, 40), 0);
    }

    #[test]
    fn noise_chain_operations() {
        let a = NoiseBudget::fresh(5);
        let b = NoiseBudget::fresh(5);
        let after_add = NoiseBudget::after_add(&a, &b); // 6
        let after_mul = NoiseBudget::after_mul(&after_add, &a, 1024); // 6+5+10=21
        assert_eq!(after_mul.log_bound, 21);
        let after_boot = NoiseBudget::after_bootstrap(8); // 8
        assert_eq!(NoiseBudget::remaining(&after_boot, 50), 42);
    }

    // ── Sample: ternary ──────────────────────────────────────────────

    #[test]
    fn sample_ternary_range() {
        let elem = sample::sample_ternary(12345, 1024);
        for i in 0..1024 {
            let v = elem.coeffs[i].as_u64();
            assert!(v == 0 || v == 1 || v == P - 1,
                "ternary sample out of range at index {}: {}", i, v);
        }
    }

    #[test]
    fn sample_ternary_not_all_zero() {
        let elem = sample::sample_ternary(42, 1024);
        assert!(!elem.is_zero(), "ternary sample should not be all zeros");
    }

    #[test]
    fn sample_ternary_deterministic() {
        let a = sample::sample_ternary(999, 256);
        let b = sample::sample_ternary(999, 256);
        assert!(a.eq_ring(&b), "same seed should produce same result");
    }

    #[test]
    fn sample_ternary_different_seeds() {
        let a = sample::sample_ternary(1, 256);
        let b = sample::sample_ternary(2, 256);
        assert!(!a.eq_ring(&b), "different seeds should produce different results");
    }

    // ── Sample: CBD ──────────────────────────────────────────────────

    #[test]
    fn sample_cbd_bound() {
        let eta = 3;
        let elem = sample::sample_cbd(54321, 1024, eta);
        for i in 0..1024 {
            let v = elem.coeffs[i].as_u64();
            // Value should be in {0, 1, ..., eta, p-1, p-2, ..., p-eta}
            let is_small_pos = v <= eta as u64;
            let is_small_neg = v >= P - eta as u64;
            assert!(is_small_pos || is_small_neg,
                "CBD sample out of range at index {}: 0x{:X}", i, v);
        }
    }

    #[test]
    fn sample_cbd_deterministic() {
        let a = sample::sample_cbd(777, 256, 2);
        let b = sample::sample_cbd(777, 256, 2);
        assert!(a.eq_ring(&b));
    }

    #[test]
    fn sample_cbd_eta1() {
        let elem = sample::sample_cbd(111, 256, 1);
        for i in 0..256 {
            let v = elem.coeffs[i].as_u64();
            assert!(v == 0 || v == 1 || v == P - 1,
                "CBD(1) should give ternary, got 0x{:X} at {}", v, i);
        }
    }

    // ── Sample: uniform ──────────────────────────────────────────────

    #[test]
    fn sample_uniform_deterministic() {
        let a = sample::sample_uniform(42, 256);
        let b = sample::sample_uniform(42, 256);
        assert!(a.eq_ring(&b));
    }

    #[test]
    fn sample_uniform_not_zero() {
        let a = sample::sample_uniform(42, 256);
        assert!(!a.is_zero());
    }

    // ── Encoding: roundtrip ──────────────────────────────────────────

    #[test]
    fn encoding_roundtrip_small() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        a.coeffs[1] = g(0xDEADBEEF);
        a.coeffs[7] = Goldilocks::NEG_ONE;

        let mut buf = [0u8; 64]; // 8 * 8
        let written = encoding::encode_ring(&a, &mut buf);
        assert_eq!(written, 64);

        let decoded = encoding::decode_ring(&buf, n);
        assert!(decoded.eq_ring(&a));
    }

    #[test]
    fn encoding_roundtrip_n1024() {
        let n = 1024;
        let a = sample::sample_uniform(0xBEEF, n);
        let mut buf = [0u8; 8192]; // 1024 * 8
        let written = encoding::encode_ring(&a, &mut buf);
        assert_eq!(written, 8192);
        let decoded = encoding::decode_ring(&buf, n);
        assert!(decoded.eq_ring(&a));
    }

    #[test]
    fn encoding_zero() {
        let n = 8;
        let z = RingElement::new(n);
        let mut buf = [0u8; 64];
        encoding::encode_ring(&z, &mut buf);
        let decoded = encoding::decode_ring(&buf, n);
        assert!(decoded.is_zero());
    }

    #[test]
    fn encoding_preserves_exact_values() {
        let n = 4;
        let values = [g(0), g(1), g(P - 1), g(0x123456789ABCDEF0)];
        let a = RingElement::from_coeffs(&values, n);
        let mut buf = [0u8; 32]; // 4 * 8
        encoding::encode_ring(&a, &mut buf);
        let decoded = encoding::decode_ring(&buf, n);
        for i in 0..n {
            assert_eq!(decoded.coeffs[i].as_u64(), a.coeffs[i].as_u64(),
                "mismatch at index {}", i);
        }
    }

    // ── Automorphism ─────────────────────────────────────────────────

    #[test]
    fn automorphism_identity() {
        // k=0: t = 5^0 = 1, so automorphism is identity
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(3);
        a.coeffs[1] = g(5);
        a.coeffs[2] = g(7);
        let result = a.automorphism(0);
        assert!(result.eq_ring(&a));
    }

    #[test]
    fn automorphism_constant_poly() {
        // Constant polynomial is fixed by all automorphisms
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        let result = a.automorphism(3);
        assert!(result.eq_ring(&a));
    }

    #[test]
    fn automorphism_x_to_x5() {
        // k=1: x -> x^5 (for n=8, 2n=16, t=5)
        // f(x) = x -> f(x^5) = x^5
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[1] = g(1); // x
        let result = a.automorphism(1);
        // x^5 in degree-8 ring: coefficient at position 5
        assert_eq!(result.coeffs[5].as_u64(), 1);
        for i in 0..n {
            if i != 5 {
                assert!(result.coeffs[i].is_zero());
            }
        }
    }

    #[test]
    fn automorphism_negacyclic_wrap() {
        // k=1: x -> x^5 for n=4 (2n=8, t=5)
        // For f(x) = x^2, automorphism gives x^(2*5 mod 8) = x^(10 mod 8) = x^2
        // Wait: 2*5=10, 10 mod 8 = 2. So x^2 -> x^2
        // Let's use x^3 instead: 3*5=15, 15 mod 8 = 7. 7 >= 4, so coeff at 7-4=3 is negated.
        let n = 4;
        let mut a = RingElement::new(n);
        a.coeffs[3] = g(1); // x^3
        let result = a.automorphism(1);
        // 3*5 = 15, 15 mod 8 = 7, 7 >= 4, so -x^(7-4) = -x^3
        assert_eq!(result.coeffs[3].as_u64(), P - 1); // -1
        for i in 0..n {
            if i != 3 {
                assert!(result.coeffs[i].is_zero());
            }
        }
    }

    #[test]
    fn automorphism_ring_homomorphism() {
        // sigma(a + b) = sigma(a) + sigma(b)
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        a.coeffs[0] = g(3);
        a.coeffs[1] = g(5);
        b.coeffs[0] = g(7);
        b.coeffs[2] = g(11);
        let k = 1;
        let lhs = a.add(&b).automorphism(k);
        let rhs = a.automorphism(k).add(&b.automorphism(k));
        assert!(lhs.eq_ring(&rhs));
    }

    // ── Ring: eq_ring ────────────────────────────────────────────────

    #[test]
    fn eq_ring_same() {
        let n = 8;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(42);
        assert!(a.eq_ring(&a));
    }

    #[test]
    fn eq_ring_different() {
        let n = 8;
        let mut a = RingElement::new(n);
        let mut b = RingElement::new(n);
        a.coeffs[0] = g(42);
        b.coeffs[0] = g(43);
        assert!(!a.eq_ring(&b));
    }

    #[test]
    fn eq_ring_canonical() {
        // Goldilocks::new(P) should canonicalize to 0
        let n = 4;
        let a = RingElement::new(n); // all zeros
        let mut b = RingElement::new(n);
        b.coeffs[0] = Goldilocks::new(P); // should be 0 after canonicalization
        assert!(a.eq_ring(&b));
    }

    // ── Ring: from_coeffs ────────────────────────────────────────────

    #[test]
    fn from_coeffs_basic() {
        let coeffs = [g(1), g(2), g(3), g(4)];
        let a = RingElement::from_coeffs(&coeffs, 4);
        assert_eq!(a.coeffs[0].as_u64(), 1);
        assert_eq!(a.coeffs[1].as_u64(), 2);
        assert_eq!(a.coeffs[2].as_u64(), 3);
        assert_eq!(a.coeffs[3].as_u64(), 4);
        assert_eq!(a.n, 4);
    }

    // ── NTT: mul consistency with schoolbook ─────────────────────────

    #[test]
    fn ntt_mul_matches_schoolbook_small() {
        // (2 + 3x) * (5 + 7x) mod (x^4 + 1)
        // = 10 + 14x + 15x + 21x^2 = 10 + 29x + 21x^2
        // No wraparound since degree < n=4
        let n = 4;
        let mut a = RingElement::new(n);
        a.coeffs[0] = g(2);
        a.coeffs[1] = g(3);
        let mut b = RingElement::new(n);
        b.coeffs[0] = g(5);
        b.coeffs[1] = g(7);
        let result = a.mul(&b);
        assert_eq!(result.coeffs[0].as_u64(), 10);
        assert_eq!(result.coeffs[1].as_u64(), 29);
        assert_eq!(result.coeffs[2].as_u64(), 21);
        assert!(result.coeffs[3].is_zero());
    }

    // ── Additional ring tests ────────────────────────────────────────

    #[test]
    fn ring_mul_x_n_minus_1_times_x() {
        // Another negacyclic test at n=4
        let n = 4;
        let mut a = RingElement::new(n);
        a.coeffs[3] = g(2); // 2*x^3
        let mut b = RingElement::new(n);
        b.coeffs[1] = g(3); // 3*x
        let result = a.mul(&b);
        // 2*x^3 * 3*x = 6*x^4 = 6*(-1) = -6 mod p
        assert_eq!(result.coeffs[0].as_u64(), P - 6);
        for i in 1..n {
            assert!(result.coeffs[i].is_zero());
        }
    }

    #[test]
    fn ring_mul_full_wrap() {
        // (1 + x + x^2 + x^3) * x mod (x^4 + 1)
        // = x + x^2 + x^3 + x^4 = x + x^2 + x^3 - 1
        let n = 4;
        let mut a = RingElement::new(n);
        for i in 0..4 {
            a.coeffs[i] = g(1);
        }
        let mut b = RingElement::new(n);
        b.coeffs[1] = g(1); // x
        let result = a.mul(&b);
        assert_eq!(result.coeffs[0].as_u64(), P - 1); // -1
        assert_eq!(result.coeffs[1].as_u64(), 1);
        assert_eq!(result.coeffs[2].as_u64(), 1);
        assert_eq!(result.coeffs[3].as_u64(), 1);
    }
}
