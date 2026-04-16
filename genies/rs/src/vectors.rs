// ---
// tags: genies, source
// crystal-type: source
// crystal-domain: comp
// ---

//! Test vectors for genies operations.
//!
//! Values computed from the prime definition and verified against
//! reference SageMath computations.

use crate::action::{Ideal, NUM_PRIMES, PRIMES};
use crate::curve::{MontCurve, MontPoint};
use crate::encoding;
use crate::fq::{Fq, PRIME};
use crate::isogeny;

// ─── Fq constants ───────────────────────────────────────────────────────────

const A: Fq = Fq::from_limbs([
    0x1c80317fa3b1799e,
    0xbdd640fb06671ad1,
    0x3eb13b9046685257,
    0x23b8c1e9392456de,
    0x1a3d1fa7bc8960a9,
    0xbd9c66b3ad3c2d6d,
    0x8b9d2434e465e150,
    0x4b95423416419f82,
]);

const B: Fq = Fq::from_limbs([
    0x0822e8f36c03119a,
    0x17fc695a07a0ca6e,
    0x3b8faa1837f8a88b,
    0x9a1de644815ef6d1,
    0x8fadc1a606cb0fb3,
    0xb74d0fb132e70629,
    0xb38a088ca65ed389,
    0x35b2d3528b8148f6,
]);

// ─── Fq arithmetic tests ────────────────────────────────────────────────────

#[test]
fn fq_zero_is_additive_identity() {
    let a = A;
    assert_eq!(Fq::add(&a, &Fq::ZERO), a);
    assert_eq!(Fq::add(&Fq::ZERO, &a), a);
}

#[test]
fn fq_one_is_multiplicative_identity() {
    let a = A;
    assert_eq!(Fq::mul(&a, &Fq::ONE), a);
    assert_eq!(Fq::mul(&Fq::ONE, &a), a);
}

#[test]
fn fq_add() {
    let expected = Fq::from_limbs([
        0x0921616ddbedc2bd,
        0x13608e60b65b3d0a,
        0x28d9b4dc5f55abbd,
        0x162be168528ff8a8,
        0x4eeee4873031a68f,
        0xc0bc6e29f25aa754,
        0x429c7bf02c86688f,
        0x1b9386f72db35eb9,
    ]);
    assert_eq!(Fq::add(&A, &B), expected);
}

#[test]
fn fq_sub() {
    let expected = Fq::from_limbs([
        0x145d488c37ae6804,
        0xa5d9d7a0fec65063,
        0x032191780e6fa9cc,
        0x899adba4b7c5600d,
        0x8a8f5e01b5be50f5,
        0x064f57027a552743,
        0xd8131ba83e070dc7,
        0x15e26ee18ac0568b,
    ]);
    assert_eq!(Fq::sub(&A, &B), expected);
}

#[test]
fn fq_mul() {
    let expected = Fq::from_limbs([
        0x5420d7db65a47926,
        0x7e31f0bfdf279089,
        0xb3d822cbb62a94f6,
        0x0be9ac73c59a2364,
        0x540589158c35fa8e,
        0x4dbb15a0af296b03,
        0xe73ef97e45d2471a,
        0x44b1c95ec12549e9,
    ]);
    assert_eq!(Fq::mul(&A, &B), expected);
}

#[test]
fn fq_square() {
    let expected = Fq::from_limbs([
        0xec52d7d085c522e8,
        0xb6cdf5eb26498a59,
        0xd27e42ccaa11e984,
        0x31706a610cceeda1,
        0xe23267261db20afe,
        0x2e2af937dc31b9ac,
        0x1df6ef43208f2d3d,
        0x12ee132a495058f8,
    ]);
    assert_eq!(Fq::square(&A), expected);
}

#[test]
fn fq_neg() {
    let neg_a = Fq::neg(&A);
    let expected = Fq::from_limbs([
        0xff01878590154edd,
        0x049bdaf951458d63,
        0x12b5f53bd8a2fcce,
        0x83f204dc2ecefe29,
        0x40bedd1ed6996924,
        0xf690a187408c5ed5,
        0x70ed8c9c79d86af9,
        0x1a1f4c5b5dcdea3d,
    ]);
    assert_eq!(neg_a, expected);
    // a + (-a) = 0
    assert_eq!(Fq::add(&A, &neg_a), Fq::ZERO);
}

#[test]
fn fq_inv() {
    let a_inv = Fq::inv(&A);
    let expected = Fq::from_limbs([
        0x333496e202823448,
        0x139122bd7d8ffb39,
        0x414db86389c0640f,
        0x4cb5bbc2605b988e,
        0x1a369a55b6b97208,
        0x25065eca4be547a6,
        0x77d01361ad78930c,
        0x1c75a0cdb2bdd0d1,
    ]);
    assert_eq!(a_inv, expected);
    // a * a^(-1) = 1
    assert_eq!(Fq::mul(&A, &a_inv), Fq::ONE);
}

#[test]
fn fq_mul_by_zero() {
    assert_eq!(Fq::mul(&A, &Fq::ZERO), Fq::ZERO);
}

#[test]
fn fq_add_commutative() {
    assert_eq!(Fq::add(&A, &B), Fq::add(&B, &A));
}

#[test]
fn fq_mul_commutative() {
    assert_eq!(Fq::mul(&A, &B), Fq::mul(&B, &A));
}

#[test]
fn fq_sub_self_is_zero() {
    assert_eq!(Fq::sub(&A, &A), Fq::ZERO);
}

#[test]
fn fq_neg_zero_is_zero() {
    assert_eq!(Fq::neg(&Fq::ZERO), Fq::ZERO);
}

#[test]
fn fq_square_of_one() {
    assert_eq!(Fq::square(&Fq::ONE), Fq::ONE);
}

#[test]
fn fq_inv_of_one() {
    assert_eq!(Fq::inv(&Fq::ONE), Fq::ONE);
}

#[test]
fn fq_small_mul() {
    let two = Fq::from_u64(2);
    let three = Fq::from_u64(3);
    let six = Fq::from_u64(6);
    assert_eq!(Fq::mul(&two, &three), six);
}

#[test]
fn fq_small_square() {
    let seven = Fq::from_u64(7);
    let fortynine = Fq::from_u64(49);
    assert_eq!(Fq::square(&seven), fortynine);
}

#[test]
fn fq_small_inv() {
    let two = Fq::from_u64(2);
    let inv2 = Fq::inv(&two);
    assert_eq!(Fq::mul(&two, &inv2), Fq::ONE);
}

#[test]
fn fq_legendre_qr() {
    let three = Fq::from_u64(3);
    assert_eq!(Fq::legendre(&three), 1);
}

#[test]
fn fq_legendre_nqr() {
    let two = Fq::from_u64(2);
    assert_eq!(Fq::legendre(&two), -1);
}

#[test]
fn fq_legendre_zero() {
    assert_eq!(Fq::legendre(&Fq::ZERO), 0);
}

#[test]
fn fq_sqrt_of_four() {
    let four = Fq::from_u64(4);
    let s = Fq::sqrt(&four).expect("4 is a QR");
    // s should be either 2 or q-2
    assert_eq!(Fq::square(&s), four);
}

#[test]
fn fq_sqrt_of_nine() {
    let nine = Fq::from_u64(9);
    let s = Fq::sqrt(&nine).expect("9 is a QR");
    assert_eq!(Fq::square(&s), nine);
}

#[test]
fn fq_sqrt_of_nqr_returns_none() {
    let two = Fq::from_u64(2);
    assert!(Fq::sqrt(&two).is_none());
}

#[test]
fn fq_distributive() {
    // a * (b + c) = a*b + a*c  where c = ONE
    let ab = Fq::mul(&A, &B);
    let ac = Fq::mul(&A, &Fq::ONE);
    let bc = Fq::add(&B, &Fq::ONE);
    let lhs = Fq::mul(&A, &bc);
    let rhs = Fq::add(&ab, &ac);
    assert_eq!(lhs, rhs);
}

// ─── Prime verification ─────────────────────────────────────────────────────

#[test]
fn prime_mod_4_is_3() {
    // q mod 4 = 3 (required for supersingular curve)
    assert_eq!(PRIME[0] & 3, 3);
}

#[test]
fn prime_plus_one_divisible_by_small_primes() {
    // q + 1 should be divisible by each of the 74 primes
    // Compute (q + 1) mod l_i for each prime
    for &ell in PRIMES.iter() {
        let mut rem = 0u128;
        let mut i = 8;
        while i > 0 {
            i -= 1;
            rem = (rem << 64) | (PRIME[i] as u128);
            rem %= ell as u128;
        }
        // rem = q mod ell
        // q + 1 mod ell should be 0
        let qp1_mod_ell = (rem + 1) % (ell as u128);
        assert_eq!(qp1_mod_ell, 0, "q+1 not divisible by {}", ell);
    }
}

#[test]
fn prime_bit_length_is_511() {
    // The top limb should have its high bit at position 510 (0-indexed)
    // which is bit 62 of limb 7 (7*64 + 62 = 510, so 511 bits)
    assert!(PRIME[7] >> 63 == 0, "bit 511 should be 0");
    assert!(PRIME[7] >> 62 == 1, "bit 510 should be 1 (511-bit number)");
}

// ─── Curve tests ─────────────────────────────────────────────────────────────

#[test]
fn curve_e0_rhs() {
    // E_0: y^2 = x^3 + x
    let e0 = MontCurve::e0();
    // rhs(4) = 64 + 4 = 68
    let four = Fq::from_u64(4);
    let rhs = e0.rhs(&four);
    assert_eq!(rhs, Fq::from_u64(68));
}

#[test]
fn curve_e0_point_on_curve() {
    let e0 = MontCurve::e0();
    // x=4: 68 is a QR
    let four = Fq::from_u64(4);
    assert!(e0.is_on_curve(&four));
    // x=1: 2 is NQR
    let one = Fq::from_u64(1);
    assert!(!e0.is_on_curve(&one));
}

#[test]
fn curve_j_invariant_e0() {
    // j(E_0) = 256 * (0 - 3)^3 / (0 - 4) = 256 * (-27) / (-4) = 1728
    let e0 = MontCurve::e0();
    let j = e0.j_invariant();
    assert_eq!(j, Fq::from_u64(1728));
}

#[test]
fn point_inf_is_inf() {
    assert!(MontPoint::inf().is_inf());
}

#[test]
fn point_from_x_not_inf() {
    let p = MontPoint::from_x(Fq::from_u64(4));
    assert!(!p.is_inf());
}

#[test]
fn point_double() {
    // Double a point on E_0
    let e0 = MontCurve::e0();
    let p = MontPoint::from_x(Fq::from_u64(4));
    let two = Fq::from_u64(2);
    let four = Fq::from_u64(4);
    let a24 = Fq::mul(&Fq::add(&e0.a, &two), &Fq::inv(&four));
    let p2 = p.xdbl(&a24);
    // The result should not be infinity (unless 4 has order 2, which it doesn't)
    assert!(!p2.is_inf());
    // The affine x-coordinate of 2P should be on E_0
    let x2 = p2.to_affine().unwrap();
    assert!(e0.is_on_curve(&x2));
}

#[test]
fn ladder_scalar_mul_identity() {
    // [1] * P = P
    let p = MontPoint::from_x(Fq::from_u64(4));
    let e0 = MontCurve::e0();
    let result = p.ladder(&[1], &e0.a);
    let x_result = result.to_affine().unwrap();
    assert_eq!(x_result, Fq::from_u64(4));
}

#[test]
fn ladder_scalar_mul_2() {
    // [2] * P via ladder should match xdbl
    let e0 = MontCurve::e0();
    let p = MontPoint::from_x(Fq::from_u64(4));
    let two = Fq::from_u64(2);
    let four = Fq::from_u64(4);
    let a24 = Fq::mul(&Fq::add(&e0.a, &two), &Fq::inv(&four));

    let via_dbl = p.xdbl(&a24);
    let via_ladder = p.ladder(&[2], &e0.a);

    let x_dbl = via_dbl.to_affine().unwrap();
    let x_lad = via_ladder.to_affine().unwrap();
    assert_eq!(x_dbl, x_lad);
}

#[test]
fn ladder_order_divides_qp1() {
    // For a point on E_0, [q+1]*P = O (since #E_0(F_q) = q+1 for supersingular)
    let e0 = MontCurve::e0();
    let p = MontPoint::from_x(Fq::from_u64(4));

    // q + 1 as limbs
    let mut qp1 = [0u64; 8];
    let mut carry = 1u64;
    let mut i = 0;
    while i < 8 {
        let (s, c) = PRIME[i].overflowing_add(carry);
        qp1[i] = s;
        carry = c as u64;
        i += 1;
    }

    let result = p.ladder(&qp1, &e0.a);
    assert!(result.is_inf());
}

// ─── Isogeny tests ──────────────────────────────────────────────────────────

#[test]
fn isogeny_3_on_e0() {
    // Find a 3-isogeny kernel point on E_0
    let e0 = MontCurve::e0();

    // Cofactor = (q+1)/3
    let mut qp1 = [0u64; 8];
    let mut carry = 1u64;
    let mut i = 0;
    while i < 8 {
        let (s, c) = PRIME[i].overflowing_add(carry);
        qp1[i] = s;
        carry = c as u64;
        i += 1;
    }
    // Divide qp1 by 3
    let mut cof = [0u64; 8];
    let mut rem = 0u128;
    i = 8;
    while i > 0 {
        i -= 1;
        rem = (rem << 64) | (qp1[i] as u128);
        cof[i] = (rem / 3) as u64;
        rem %= 3;
    }

    // Find x such that x^3+x is a QR, then cofactor multiply
    let mut x_val = 0u64;
    let kernel;
    loop {
        let x = Fq::from_u64(x_val);
        let rhs = e0.rhs(&x);
        if Fq::legendre(&rhs) == 1 {
            let p = MontPoint::from_x(x);
            let q_pt = p.ladder(&cof, &e0.a);
            if !q_pt.is_inf() {
                // Verify order: [3]*q_pt should be O
                let check = q_pt.ladder(&[3], &e0.a);
                if check.is_inf() {
                    kernel = q_pt;
                    break;
                }
            }
        }
        x_val += 1;
    }

    // Compute the 3-isogeny
    let e1 = isogeny::isogeny_codomain(&e0, &kernel, 3);

    // The codomain should be a valid curve (A'^2 != 4 for non-degenerate)
    let a2 = Fq::square(&e1.a);
    assert_ne!(a2, Fq::from_u64(4));

    // The codomain should have the same supersingular structure:
    // points on e1 should have order dividing q+1
    let p2 = MontPoint::from_x(Fq::from_u64(4));
    if e1.is_on_curve(&Fq::from_u64(4)) {
        let result = p2.ladder(&qp1, &e1.a);
        assert!(result.is_inf());
    }
}

#[test]
fn isogeny_preserves_supersingularity() {
    // After a 5-isogeny, the curve should still be supersingular
    let e0 = MontCurve::e0();

    let mut qp1 = [0u64; 8];
    let mut carry = 1u64;
    let mut i = 0;
    while i < 8 {
        let (s, c) = PRIME[i].overflowing_add(carry);
        qp1[i] = s;
        carry = c as u64;
        i += 1;
    }

    // Cofactor for ℓ=5
    let mut cof = [0u64; 8];
    let mut rem = 0u128;
    i = 8;
    while i > 0 {
        i -= 1;
        rem = (rem << 64) | (qp1[i] as u128);
        cof[i] = (rem / 5) as u64;
        rem %= 5;
    }

    let mut x_val = 0u64;
    let kernel;
    loop {
        let x = Fq::from_u64(x_val);
        let rhs = e0.rhs(&x);
        if Fq::legendre(&rhs) == 1 {
            let p = MontPoint::from_x(x);
            let q_pt = p.ladder(&cof, &e0.a);
            if !q_pt.is_inf() {
                let check = q_pt.ladder(&[5], &e0.a);
                if check.is_inf() {
                    kernel = q_pt;
                    break;
                }
            }
        }
        x_val += 1;
    }

    let e1 = isogeny::isogeny_codomain(&e0, &kernel, 5);

    // Test: find a point on e1 and verify its order divides q+1
    let mut test_x = 0u64;
    loop {
        let x = Fq::from_u64(test_x);
        if e1.is_on_curve(&x) {
            let p = MontPoint::from_x(x);
            let result = p.ladder(&qp1, &e1.a);
            assert!(
                result.is_inf(),
                "point on codomain does not have order dividing q+1"
            );
            break;
        }
        test_x += 1;
        if test_x > 100 {
            panic!("could not find point on codomain curve");
        }
    }
}

// ─── Action tests ───────────────────────────────────────────────────────────

#[test]
fn action_identity_is_noop() {
    let e0 = MontCurve::e0();
    let id = Ideal::identity();
    let result = crate::action::action(&id, &e0);
    assert_eq!(result.a, e0.a);
}

#[test]
fn action_single_prime() {
    // Apply exponent +1 for prime ℓ=3 only
    let e0 = MontCurve::e0();
    let mut exponents = [0i8; NUM_PRIMES];
    exponents[0] = 1; // e_1 = +1 for ℓ_1 = 3
    let ideal = Ideal::from_exponents(&exponents);
    let result = crate::action::action(&ideal, &e0);

    // The result should differ from E_0
    assert_ne!(result.a, e0.a, "action with e=+1 should change the curve");
}

#[test]
fn action_inverse_returns_to_origin() {
    // Apply +1 then -1 for prime 3 should return to E_0
    let e0 = MontCurve::e0();

    let mut exp_pos = [0i8; NUM_PRIMES];
    exp_pos[0] = 1;
    let ideal_pos = Ideal::from_exponents(&exp_pos);
    let e1 = crate::action::action(&ideal_pos, &e0);

    let mut exp_neg = [0i8; NUM_PRIMES];
    exp_neg[0] = -1;
    let ideal_neg = Ideal::from_exponents(&exp_neg);
    let e2 = crate::action::action(&ideal_neg, &e1);

    // e2 should be isomorphic to e0, i.e., same j-invariant
    let j0 = e0.j_invariant();
    let j2 = e2.j_invariant();
    assert_eq!(
        j0, j2,
        "applying +1 then -1 should return to E_0 (same j-invariant)"
    );
}

#[test]
fn dh_commutativity() {
    // a * (b * E_0) = b * (a * E_0)
    let e0 = MontCurve::e0();

    let mut exp_a = [0i8; NUM_PRIMES];
    exp_a[0] = 1; // ℓ=3
    exp_a[1] = 1; // ℓ=5
    let ideal_a = Ideal::from_exponents(&exp_a);

    let mut exp_b = [0i8; NUM_PRIMES];
    exp_b[0] = 1; // ℓ=3
    exp_b[2] = 1; // ℓ=7
    let ideal_b = Ideal::from_exponents(&exp_b);

    let ea = crate::action::action(&ideal_a, &e0);
    let eb = crate::action::action(&ideal_b, &e0);

    let ab = crate::action::dh(&ideal_b, &ea);
    let ba = crate::action::dh(&ideal_a, &eb);

    // Same j-invariant
    assert_eq!(
        ab.j_invariant(),
        ba.j_invariant(),
        "DH commutativity failed"
    );
}

// ─── Encoding tests ─────────────────────────────────────────────────────────

#[test]
fn encode_decode_fq_roundtrip() {
    let bytes = encoding::encode_fq(&A);
    let decoded = encoding::decode_fq(&bytes).expect("valid Fq decoding");
    assert_eq!(decoded, A);
}

#[test]
fn encode_decode_fq_zero() {
    let bytes = encoding::encode_fq(&Fq::ZERO);
    let decoded = encoding::decode_fq(&bytes).expect("valid Fq decoding");
    assert_eq!(decoded, Fq::ZERO);
}

#[test]
fn encode_decode_fq_one() {
    let bytes = encoding::encode_fq(&Fq::ONE);
    let decoded = encoding::decode_fq(&bytes).expect("valid Fq decoding");
    assert_eq!(decoded, Fq::ONE);
}

#[test]
fn decode_fq_rejects_q() {
    // Encoding q itself should be rejected (non-canonical)
    let mut bytes = [0u8; 64];
    let mut i = 0;
    while i < 8 {
        let limb_bytes = PRIME[i].to_le_bytes();
        let mut j = 0;
        while j < 8 {
            bytes[i * 8 + j] = limb_bytes[j];
            j += 1;
        }
        i += 1;
    }
    assert!(encoding::decode_fq(&bytes).is_none());
}

#[test]
fn encode_decode_curve_roundtrip() {
    let curve = MontCurve { a: A };
    let bytes = encoding::encode_curve(&curve);
    let decoded = encoding::decode_curve(&bytes).expect("valid curve decoding");
    assert_eq!(decoded, curve);
}

#[test]
fn encode_decode_ideal_roundtrip() {
    let mut exponents = [0i8; NUM_PRIMES];
    exponents[0] = 3;
    exponents[1] = -2;
    exponents[73] = 5;
    let ideal = Ideal { exponents };
    let bytes = encoding::encode_ideal(&ideal);
    let decoded = encoding::decode_ideal(&bytes).expect("valid ideal decoding");
    assert_eq!(decoded, ideal);
}

#[test]
fn decode_ideal_rejects_overflow() {
    let mut bytes = [5u8; NUM_PRIMES]; // all at midpoint (exponent = 0)
    bytes[0] = 11; // 2*5 + 1 = 11 > 2*MAX_EXPONENT = 10
    assert!(encoding::decode_ideal(&bytes).is_none());
}

#[test]
fn fq_add_associative() {
    let c = Fq::from_u64(42);
    let ab = Fq::add(&A, &B);
    let lhs = Fq::add(&ab, &c);
    let bc = Fq::add(&B, &c);
    let rhs = Fq::add(&A, &bc);
    assert_eq!(lhs, rhs);
}

#[test]
fn fq_mul_associative() {
    let c = Fq::from_u64(42);
    let ab = Fq::mul(&A, &B);
    let lhs = Fq::mul(&ab, &c);
    let bc = Fq::mul(&B, &c);
    let rhs = Fq::mul(&A, &bc);
    assert_eq!(lhs, rhs);
}

#[test]
fn fq_double_neg() {
    // -(-a) = a
    let neg_a = Fq::neg(&A);
    let neg_neg_a = Fq::neg(&neg_a);
    assert_eq!(neg_neg_a, A);
}

#[test]
fn fq_inv_of_inv() {
    // inv(inv(a)) = a
    let a_inv = Fq::inv(&A);
    let a_inv_inv = Fq::inv(&a_inv);
    assert_eq!(a_inv_inv, A);
}

#[test]
fn fq_sqrt_roundtrip() {
    // sqrt(a^2) should be a or q-a
    let a2 = Fq::square(&A);
    let s = Fq::sqrt(&a2).expect("a^2 is always a QR");
    assert_eq!(Fq::square(&s), a2);
}
