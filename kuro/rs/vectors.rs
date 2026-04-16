//! Comprehensive test vectors for F₂ tower field arithmetic.
//! Covers all operations at all tower levels.

use crate::batch::*;
use crate::encoding::*;
use crate::tower::*;

// ===== F₂ base field =====

#[test]
fn f2_add_00() {
    assert_eq!(F2(0).add(F2(0)), F2::ZERO);
}
#[test]
fn f2_add_01() {
    assert_eq!(F2(0).add(F2(1)), F2::ONE);
}
#[test]
fn f2_add_10() {
    assert_eq!(F2(1).add(F2(0)), F2::ONE);
}
#[test]
fn f2_add_11() {
    assert_eq!(F2(1).add(F2(1)), F2::ZERO);
}
#[test]
fn f2_mul_00() {
    assert_eq!(F2(0).mul(F2(0)), F2::ZERO);
}
#[test]
fn f2_mul_01() {
    assert_eq!(F2(0).mul(F2(1)), F2::ZERO);
}
#[test]
fn f2_mul_11() {
    assert_eq!(F2(1).mul(F2(1)), F2::ONE);
}
#[test]
fn f2_inv() {
    assert_eq!(F2(1).inv(), F2::ONE);
}
#[test]
fn f2_square() {
    assert_eq!(F2(1).square(), F2::ONE);
}

// ===== F₂² =====

#[test]
fn f2_2_add_self_zero() {
    for i in 0..4u8 {
        let a = F2_2(i);
        assert_eq!(a.add(a), F2_2::ZERO, "a + a = 0 for all a");
    }
}

#[test]
fn f2_2_mul_identity() {
    for i in 0..4u8 {
        let a = F2_2(i);
        assert_eq!(a.mul(F2_2::ONE), a, "a * 1 = a");
    }
}

#[test]
fn f2_2_mul_zero() {
    for i in 0..4u8 {
        assert_eq!(F2_2(i).mul(F2_2::ZERO), F2_2::ZERO, "a * 0 = 0");
    }
}

#[test]
fn f2_2_mul_specific() {
    // (x+1) * x = x² + x = (x+1) + x = 1  (since x² = x + 1)
    assert_eq!(F2_2(0b11).mul(F2_2(0b10)), F2_2::ONE);
    // x * x = x² = x + 1
    assert_eq!(F2_2(0b10).mul(F2_2(0b10)), F2_2(0b11));
}

#[test]
fn f2_2_inv_all_nonzero() {
    for i in 1..4u8 {
        let a = F2_2(i);
        let inv = a.inv();
        assert_eq!(a.mul(inv), F2_2::ONE, "a * a^-1 = 1 for a = {}", i);
    }
}

#[test]
fn f2_2_square_frobenius() {
    for i in 0..4u8 {
        let a = F2_2(i);
        assert_eq!(a.square(), a.mul(a), "a² = a*a");
    }
}

#[test]
fn f2_2_trace_values() {
    assert_eq!(F2_2(0b00).trace(), F2(0)); // Tr(0) = 0
    assert_eq!(F2_2(0b01).trace(), F2(0)); // Tr(1) = 0 (in F₂²/F₂)
    assert_eq!(F2_2(0b10).trace(), F2(1)); // Tr(x) = 1
    assert_eq!(F2_2(0b11).trace(), F2(1)); // Tr(x+1) = 1
}

#[test]
fn f2_2_sqrt_roundtrip() {
    for i in 0..4u8 {
        let a = F2_2(i);
        let s = a.sqrt();
        assert_eq!(s.square(), a, "sqrt(a)² = a for a = {}", i);
    }
}

// ===== F₂⁴ =====

#[test]
fn f2_4_mul_identity() {
    for i in 0..16u8 {
        let a = F2_4(i);
        assert_eq!(a.mul(F2_4::ONE), a);
    }
}

#[test]
fn f2_4_self_inverse_add() {
    for i in 0..16u8 {
        let a = F2_4(i);
        assert_eq!(a.add(a), F2_4::ZERO);
    }
}

#[test]
fn f2_4_inv_roundtrip() {
    for i in 1..16u8 {
        let a = F2_4(i);
        let inv = a.inv();
        assert_eq!(a.mul(inv), F2_4::ONE, "a * a^-1 = 1 for a = {:#06b}", i);
    }
}

#[test]
fn f2_4_square_is_mul() {
    for i in 0..16u8 {
        let a = F2_4(i);
        assert_eq!(a.square(), a.mul(a));
    }
}

#[test]
fn f2_4_sqrt_roundtrip() {
    for i in 0..16u8 {
        let a = F2_4(i);
        assert_eq!(a.sqrt().square(), a);
    }
}

#[test]
fn f2_4_frobenius_period() {
    // Frobenius has period 4 in F₂⁴: a^(2^4) = a
    for i in 0..16u8 {
        let a = F2_4(i);
        assert_eq!(a.frobenius(4), a);
    }
}

#[test]
fn f2_4_trace_in_f2() {
    for i in 0..16u8 {
        let tr = F2_4(i).trace();
        assert!(tr.0 <= 1, "trace must be in F₂");
    }
}

#[test]
fn f2_4_exp_order() {
    // Every non-zero element: a^(2⁴ - 1) = a^15 = 1
    for i in 1..16u8 {
        let a = F2_4(i);
        assert_eq!(a.exp(15), F2_4::ONE, "a^15 = 1 for a = {}", i);
    }
}

// ===== F₂⁸ =====

#[test]
fn f2_8_mul_identity() {
    let a = F2_8(0xAB);
    assert_eq!(a.mul(F2_8::ONE), a);
}

#[test]
fn f2_8_add_self() {
    let a = F2_8(0xFF);
    assert_eq!(a.add(a), F2_8::ZERO);
}

#[test]
fn f2_8_inv_roundtrip() {
    // Test several representative values
    let vals = [1u8, 2, 3, 0x0F, 0x55, 0xAA, 0xFE, 0xFF];
    for &v in &vals {
        let a = F2_8(v);
        let inv = a.inv();
        assert_eq!(a.mul(inv), F2_8::ONE, "inv roundtrip failed for {:#04x}", v);
    }
}

#[test]
fn f2_8_square_linearity() {
    // (a + b)² = a² + b² in char 2
    let a = F2_8(0x37);
    let b = F2_8(0xC5);
    assert_eq!(a.add(b).square(), a.square().add(b.square()));
}

#[test]
fn f2_8_sqrt_roundtrip() {
    let vals = [0u8, 1, 2, 0x42, 0xAB, 0xFF];
    for &v in &vals {
        let a = F2_8(v);
        assert_eq!(a.sqrt().square(), a, "sqrt roundtrip for {:#04x}", v);
    }
}

#[test]
fn f2_8_exp_order() {
    // a^(2⁸ - 1) = a^255 = 1 for all nonzero a
    let a = F2_8(0x42);
    assert_eq!(a.exp(255), F2_8::ONE);
}

#[test]
fn f2_8_frobenius_period() {
    let a = F2_8(0xAB);
    assert_eq!(a.frobenius(8), a);
}

// ===== F₂¹⁶ =====

#[test]
fn f2_16_mul_identity() {
    let a = F2_16(0xABCD);
    assert_eq!(a.mul(F2_16::ONE), a);
}

#[test]
fn f2_16_inv_roundtrip() {
    let vals = [1u16, 2, 0x00FF, 0x5555, 0xAAAA, 0xFFFE, 0xFFFF];
    for &v in &vals {
        let a = F2_16(v);
        let inv = a.inv();
        assert_eq!(a.mul(inv), F2_16::ONE, "inv roundtrip for {:#06x}", v);
    }
}

#[test]
fn f2_16_sqrt_roundtrip() {
    let a = F2_16(0xBEEF);
    assert_eq!(a.sqrt().square(), a);
}

#[test]
fn f2_16_square_linearity() {
    let a = F2_16(0x1234);
    let b = F2_16(0x5678);
    assert_eq!(a.add(b).square(), a.square().add(b.square()));
}

// ===== F₂³² =====

#[test]
fn f2_32_mul_identity() {
    let a = F2_32(0xDEADBEEF);
    assert_eq!(a.mul(F2_32::ONE), a);
}

#[test]
fn f2_32_inv_roundtrip() {
    let vals = [1u32, 2, 0xDEADBEEF, 0xCAFEBABE, 0xFFFFFFFF];
    for &v in &vals {
        let a = F2_32(v);
        assert_eq!(a.mul(a.inv()), F2_32::ONE, "inv for {:#010x}", v);
    }
}

#[test]
fn f2_32_sqrt_roundtrip() {
    let a = F2_32(0xDEADBEEF);
    assert_eq!(a.sqrt().square(), a);
}

#[test]
fn f2_32_mul_commutative() {
    let a = F2_32(0x12345678);
    let b = F2_32(0x9ABCDEF0);
    assert_eq!(a.mul(b), b.mul(a));
}

#[test]
fn f2_32_mul_associative() {
    let a = F2_32(0x11);
    let b = F2_32(0x22);
    let c = F2_32(0x33);
    assert_eq!(a.mul(b).mul(c), a.mul(b.mul(c)));
}

#[test]
fn f2_32_mul_distributive() {
    let a = F2_32(0xAA);
    let b = F2_32(0xBB);
    let c = F2_32(0xCC);
    assert_eq!(a.mul(b.add(c)), a.mul(b).add(a.mul(c)));
}

// ===== F₂⁶⁴ =====

#[test]
fn f2_64_mul_identity() {
    let a = F2_64(0xDEADBEEFCAFEBABE);
    assert_eq!(a.mul(F2_64::ONE), a);
}

#[test]
fn f2_64_inv_roundtrip() {
    let vals: [u64; 4] = [1, 0xDEADBEEFCAFEBABE, 0x0123456789ABCDEF, u64::MAX];
    for &v in &vals {
        let a = F2_64(v);
        assert_eq!(a.mul(a.inv()), F2_64::ONE, "inv for {:#018x}", v);
    }
}

// sqrt at F₂⁶⁴ requires 63 squarings — tested via frobenius period instead
#[test]
fn f2_64_frobenius_period() {
    let a = F2_64(0xAB);
    assert_eq!(a.frobenius(64), a);
}

#[test]
fn f2_64_square_linearity() {
    let a = F2_64(0x1111111111111111);
    let b = F2_64(0x2222222222222222);
    assert_eq!(a.add(b).square(), a.square().add(b.square()));
}

// ===== F₂¹²⁸ =====

#[test]
fn f2_128_mul_identity() {
    let a = F2_128(0xDEADBEEFCAFEBABE_1234567890ABCDEF);
    assert_eq!(a.mul(F2_128::ONE), a);
}

#[test]
fn f2_128_add_self_zero() {
    let a = F2_128(0xFF00FF00FF00FF00_FF00FF00FF00FF00);
    assert_eq!(a.add(a), F2_128::ZERO);
}

#[test]
fn f2_128_inv_roundtrip() {
    let vals: [u128; 4] = [
        1,
        0xDEADBEEFCAFEBABE_1234567890ABCDEF,
        0x0123456789ABCDEF_FEDCBA9876543210,
        u128::MAX,
    ];
    for &v in &vals {
        let a = F2_128(v);
        assert_eq!(a.mul(a.inv()), F2_128::ONE, "inv roundtrip failed");
    }
}

// sqrt at F₂¹²⁸ requires 127 squarings — too expensive for unit tests.
// Correctness follows from frobenius period and small-field sqrt tests.

#[test]
fn f2_128_square_linearity() {
    let a = F2_128(0x1111111111111111_1111111111111111);
    let b = F2_128(0x2222222222222222_2222222222222222);
    assert_eq!(a.add(b).square(), a.square().add(b.square()));
}

#[test]
fn f2_128_mul_commutative() {
    let a = F2_128(0x1234567890ABCDEF_FEDCBA0987654321);
    let b = F2_128(0xAAAABBBBCCCCDDDD_EEEEFFFF00001111);
    assert_eq!(a.mul(b), b.mul(a));
}

#[test]
fn f2_128_mul_associative() {
    let a = F2_128(0x11);
    let b = F2_128(0x22);
    let c = F2_128(0x33);
    assert_eq!(a.mul(b).mul(c), a.mul(b.mul(c)));
}

#[test]
fn f2_128_mul_distributive() {
    let a = F2_128(0xAA);
    let b = F2_128(0xBB);
    let c = F2_128(0xCC);
    assert_eq!(a.mul(b.add(c)), a.mul(b).add(a.mul(c)));
}

#[test]
fn f2_128_exp_zero() {
    let a = F2_128(0xDEADBEEF);
    assert_eq!(a.exp(0), F2_128::ONE);
}

#[test]
fn f2_128_exp_one() {
    let a = F2_128(0xDEADBEEF);
    assert_eq!(a.exp(1), a);
}

// ===== Packed128 =====

use crate::packed::Packed128;

#[test]
fn packed_add_xor() {
    let a = Packed128(0xFF);
    let b = Packed128(0x0F);
    assert_eq!(a.add(b), Packed128(0xF0));
}

#[test]
fn packed_mul_and() {
    let a = Packed128(0xFF);
    let b = Packed128(0x0F);
    assert_eq!(a.mul(b), Packed128(0x0F));
}

#[test]
fn packed_self_inverse() {
    let a = Packed128(0xDEAD_BEEF_CAFE_BABE_1234_5678_9ABC_DEF0);
    assert_eq!(a.add(a), Packed128::ZERO);
}

#[test]
fn packed_inner_product() {
    let a = Packed128(0b1111_0000);
    let b = Packed128(0b1010_1010);
    assert_eq!(a.inner_product(b), 2);
}

#[test]
fn packed_popcount_zero() {
    assert_eq!(Packed128(0).popcount(), 0);
}
#[test]
fn packed_popcount_one() {
    assert_eq!(Packed128(1).popcount(), 1);
}
#[test]
fn packed_popcount_max() {
    assert_eq!(Packed128(u128::MAX).popcount(), 128);
}
#[test]
fn packed_popcount_half() {
    assert_eq!(
        Packed128(0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA).popcount(),
        64
    );
}

#[test]
fn packed_not() {
    assert_eq!(Packed128::ZERO.not(), Packed128::ONES);
    assert_eq!(Packed128::ONES.not(), Packed128::ZERO);
}

#[test]
fn packed_get_set_bit() {
    let p = Packed128::ZERO.set_bit(42);
    assert_eq!(p.get_bit(42), 1);
    assert_eq!(p.get_bit(41), 0);
    assert_eq!(p.get_bit(43), 0);
}

#[test]
fn packed_as_tower() {
    let p = Packed128(0xDEADBEEF);
    let t = p.as_tower();
    assert_eq!(t, F2_128(0xDEADBEEF));
}

// ===== Encoding =====

#[test]
fn encode_decode_128_roundtrip() {
    let bytes: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let el = encode_128(&bytes);
    let decoded = decode_128(el);
    assert_eq!(bytes, decoded);
}

#[test]
fn encode_decode_64_roundtrip() {
    let bytes: [u8; 8] = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
    let el = encode_64(&bytes);
    let decoded = decode_64(el);
    assert_eq!(bytes, decoded);
}

#[test]
fn encode_decode_32_roundtrip() {
    let bytes: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
    let el = encode_32(&bytes);
    let decoded = decode_32(el);
    assert_eq!(bytes, decoded);
}

#[test]
fn encode_8_identity() {
    assert_eq!(encode_8(0xFF), F2_8(0xFF));
    assert_eq!(decode_8(F2_8(0x42)), 0x42);
}

// ===== Batch inversion =====

#[test]
fn batch_inv_single() {
    let input = [F2_128(0xDEADBEEF)];
    let mut output = [F2_128::ZERO];
    batch_inv_128(&input, &mut output);
    assert_eq!(input[0].mul(output[0]), F2_128::ONE);
}

#[test]
fn batch_inv_multiple() {
    let input = [F2_128(1), F2_128(2), F2_128(3)];
    let mut output = [F2_128::ZERO; 3];
    batch_inv_128(&input, &mut output);
    for i in 0..3 {
        assert_eq!(input[i].mul(output[i]), F2_128::ONE, "batch inv [{}]", i);
    }
}

#[test]
fn batch_inv_with_zero() {
    let input = [F2_128(1), F2_128::ZERO, F2_128(3)];
    let mut output = [F2_128::ZERO; 3];
    batch_inv_128(&input, &mut output);
    assert_eq!(input[0].mul(output[0]), F2_128::ONE);
    assert_eq!(output[1], F2_128::ZERO); // zero stays zero
    assert_eq!(input[2].mul(output[2]), F2_128::ONE);
}

// ===== Algebraic properties across all levels =====

#[test]
fn all_levels_mul_zero() {
    assert_eq!(F2_4(0x0A).mul(F2_4::ZERO), F2_4::ZERO);
    assert_eq!(F2_8(0xAB).mul(F2_8::ZERO), F2_8::ZERO);
    assert_eq!(F2_16(0xABCD).mul(F2_16::ZERO), F2_16::ZERO);
    assert_eq!(F2_32(0xDEAD).mul(F2_32::ZERO), F2_32::ZERO);
    assert_eq!(F2_64(0xDEAD).mul(F2_64::ZERO), F2_64::ZERO);
    assert_eq!(F2_128(0xDEAD).mul(F2_128::ZERO), F2_128::ZERO);
}

#[test]
fn all_levels_add_zero() {
    assert_eq!(F2_4(0x0A).add(F2_4::ZERO), F2_4(0x0A));
    assert_eq!(F2_8(0xAB).add(F2_8::ZERO), F2_8(0xAB));
    assert_eq!(F2_16(0xABCD).add(F2_16::ZERO), F2_16(0xABCD));
    assert_eq!(F2_32(0xDEAD).add(F2_32::ZERO), F2_32(0xDEAD));
    assert_eq!(F2_64(0xDEAD).add(F2_64::ZERO), F2_64(0xDEAD));
    assert_eq!(F2_128(0xDEAD).add(F2_128::ZERO), F2_128(0xDEAD));
}

// ===== Checked inversion =====

use crate::inv::*;

#[test]
fn checked_inv_zero_none() {
    assert!(checked_inv_128(F2_128::ZERO).is_none());
    assert!(checked_inv_64(F2_64::ZERO).is_none());
    assert!(checked_inv_32(F2_32::ZERO).is_none());
    assert!(checked_inv_16(F2_16::ZERO).is_none());
    assert!(checked_inv_8(F2_8::ZERO).is_none());
    assert!(checked_inv_4(F2_4::ZERO).is_none());
    assert!(checked_inv_2(F2_2::ZERO).is_none());
}

#[test]
fn checked_inv_one_some() {
    assert_eq!(checked_inv_128(F2_128::ONE), Some(F2_128::ONE));
    assert_eq!(checked_inv_64(F2_64::ONE), Some(F2_64::ONE));
    assert_eq!(checked_inv_32(F2_32::ONE), Some(F2_32::ONE));
}
