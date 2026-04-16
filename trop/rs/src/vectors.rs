// ---
// tags: trop, test
// crystal-type: source
// crystal-domain: comp
// ---

//! Test vectors for the tropical semiring library.

#[cfg(test)]
use crate::determinant::determinant;
#[cfg(test)]
use crate::dual::MaxPlus;
#[cfg(test)]
use crate::eigenvalue::eigenvalue;
#[cfg(test)]
use crate::element::Tropical;
#[cfg(test)]
use crate::encoding::{decode_element, decode_matrix, encode_element, encode_matrix};
#[cfg(test)]
use crate::kleene::kleene_star;
#[cfg(test)]
use crate::matrix::TropMatrix;

// ============================================================
// Element tests
// ============================================================

#[test]
fn element_add_basic() {
    let a = Tropical::from_u64(3);
    let b = Tropical::from_u64(7);
    assert_eq!(a.add(b), Tropical::from_u64(3));
}

#[test]
fn element_add_equal() {
    let a = Tropical::from_u64(5);
    assert_eq!(a.add(a), Tropical::from_u64(5));
}

#[test]
fn element_add_identity_left() {
    let a = Tropical::from_u64(42);
    assert_eq!(Tropical::INF.add(a), a);
}

#[test]
fn element_add_identity_right() {
    let a = Tropical::from_u64(42);
    assert_eq!(a.add(Tropical::INF), a);
}

#[test]
fn element_add_both_inf() {
    assert_eq!(Tropical::INF.add(Tropical::INF), Tropical::INF);
}

#[test]
fn element_add_commutativity() {
    let a = Tropical::from_u64(10);
    let b = Tropical::from_u64(20);
    assert_eq!(a.add(b), b.add(a));
}

#[test]
fn element_add_associativity() {
    let a = Tropical::from_u64(5);
    let b = Tropical::from_u64(3);
    let c = Tropical::from_u64(8);
    assert_eq!(a.add(b).add(c), a.add(b.add(c)));
}

#[test]
fn element_mul_basic() {
    let a = Tropical::from_u64(3);
    let b = Tropical::from_u64(7);
    assert_eq!(a.mul(b), Tropical::from_u64(10));
}

#[test]
fn element_mul_identity_left() {
    let a = Tropical::from_u64(42);
    assert_eq!(Tropical::ONE.mul(a), a);
}

#[test]
fn element_mul_identity_right() {
    let a = Tropical::from_u64(42);
    assert_eq!(a.mul(Tropical::ONE), a);
}

#[test]
fn element_mul_zero_left() {
    let a = Tropical::from_u64(42);
    assert_eq!(Tropical::INF.mul(a), Tropical::INF);
}

#[test]
fn element_mul_zero_right() {
    let a = Tropical::from_u64(42);
    assert_eq!(a.mul(Tropical::INF), Tropical::INF);
}

#[test]
fn element_mul_commutativity() {
    let a = Tropical::from_u64(10);
    let b = Tropical::from_u64(20);
    assert_eq!(a.mul(b), b.mul(a));
}

#[test]
fn element_mul_associativity() {
    let a = Tropical::from_u64(5);
    let b = Tropical::from_u64(3);
    let c = Tropical::from_u64(8);
    assert_eq!(a.mul(b).mul(c), a.mul(b.mul(c)));
}

#[test]
fn element_mul_overflow() {
    let a = Tropical::from_u64(u64::MAX - 1);
    let b = Tropical::from_u64(2);
    assert_eq!(a.mul(b), Tropical::INF);
}

#[test]
fn element_mul_near_max() {
    // (MAX-2) + 1 = MAX-1, which is finite
    let a = Tropical::from_u64(u64::MAX - 2);
    let b = Tropical::from_u64(1);
    assert_eq!(a.mul(b), Tropical::from_u64(u64::MAX - 1));
}

#[test]
fn element_mul_sum_equals_max() {
    // (MAX-1) + 1 = MAX, which is INF, should be treated as INF
    let a = Tropical::from_u64(u64::MAX - 1);
    let b = Tropical::from_u64(1);
    assert_eq!(a.mul(b), Tropical::INF);
}

#[test]
fn element_distributivity() {
    // a * (b + c) = (a*b) + (a*c) in tropical: a + min(b,c) = min(a+b, a+c)
    let a = Tropical::from_u64(2);
    let b = Tropical::from_u64(3);
    let c = Tropical::from_u64(5);
    assert_eq!(a.mul(b.add(c)), a.mul(b).add(a.mul(c)));
}

#[test]
fn element_distributivity_with_inf() {
    let a = Tropical::from_u64(10);
    let b = Tropical::INF;
    let c = Tropical::from_u64(5);
    assert_eq!(a.mul(b.add(c)), a.mul(b).add(a.mul(c)));
}

#[test]
fn element_is_inf() {
    assert!(Tropical::INF.is_inf());
    assert!(Tropical::ZERO.is_inf());
    assert!(!Tropical::ONE.is_inf());
    assert!(!Tropical::from_u64(42).is_inf());
}

#[test]
fn element_is_finite() {
    assert!(!Tropical::INF.is_finite());
    assert!(Tropical::ONE.is_finite());
    assert!(Tropical::from_u64(0).is_finite());
}

#[test]
fn element_zero_is_inf() {
    assert_eq!(Tropical::ZERO, Tropical::INF);
}

#[test]
fn element_one_is_zero() {
    assert_eq!(Tropical::ONE.as_u64(), 0);
}

#[test]
fn element_constants() {
    assert_eq!(Tropical::ZERO.as_u64(), u64::MAX);
    assert_eq!(Tropical::ONE.as_u64(), 0);
    assert_eq!(Tropical::INF.as_u64(), u64::MAX);
}

// ============================================================
// Matrix tests
// ============================================================

#[test]
fn matrix_new_all_inf() {
    let m = TropMatrix::new(3);
    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(m.get(i, j), Tropical::INF);
        }
    }
}

#[test]
fn matrix_identity_diagonal() {
    let m = TropMatrix::identity(3);
    for i in 0..3 {
        assert_eq!(m.get(i, i), Tropical::ONE);
    }
}

#[test]
fn matrix_identity_off_diagonal() {
    let m = TropMatrix::identity(3);
    assert_eq!(m.get(0, 1), Tropical::INF);
    assert_eq!(m.get(1, 0), Tropical::INF);
    assert_eq!(m.get(0, 2), Tropical::INF);
}

#[test]
fn matrix_set_get() {
    let mut m = TropMatrix::new(2);
    m.set(0, 1, Tropical::from_u64(5));
    assert_eq!(m.get(0, 1), Tropical::from_u64(5));
    assert_eq!(m.get(0, 0), Tropical::INF);
}

#[test]
fn matrix_add_elementwise_min() {
    let mut a = TropMatrix::new(2);
    a.set(0, 0, Tropical::from_u64(3));
    a.set(0, 1, Tropical::from_u64(7));
    let mut b = TropMatrix::new(2);
    b.set(0, 0, Tropical::from_u64(5));
    b.set(0, 1, Tropical::from_u64(2));
    let c = a.add(&b);
    assert_eq!(c.get(0, 0), Tropical::from_u64(3));
    assert_eq!(c.get(0, 1), Tropical::from_u64(2));
}

#[test]
fn matrix_mul_identity() {
    let mut a = TropMatrix::new(2);
    a.set(0, 0, Tropical::from_u64(1));
    a.set(0, 1, Tropical::from_u64(3));
    a.set(1, 0, Tropical::from_u64(2));
    a.set(1, 1, Tropical::from_u64(4));
    let id = TropMatrix::identity(2);
    let r = a.mul(&id);
    assert_eq!(r.get(0, 0), Tropical::from_u64(1));
    assert_eq!(r.get(0, 1), Tropical::from_u64(3));
    assert_eq!(r.get(1, 0), Tropical::from_u64(2));
    assert_eq!(r.get(1, 1), Tropical::from_u64(4));
}

#[test]
fn matrix_mul_2x2() {
    // A = [[1, 3], [2, 4]], B = [[0, 1], [2, 0]]
    // C[0][0] = min(1+0, 3+2) = min(1, 5) = 1
    // C[0][1] = min(1+1, 3+0) = min(2, 3) = 2
    // C[1][0] = min(2+0, 4+2) = min(2, 6) = 2
    // C[1][1] = min(2+1, 4+0) = min(3, 4) = 3
    let mut a = TropMatrix::new(2);
    a.set(0, 0, Tropical::from_u64(1));
    a.set(0, 1, Tropical::from_u64(3));
    a.set(1, 0, Tropical::from_u64(2));
    a.set(1, 1, Tropical::from_u64(4));

    let mut b = TropMatrix::new(2);
    b.set(0, 0, Tropical::from_u64(0));
    b.set(0, 1, Tropical::from_u64(1));
    b.set(1, 0, Tropical::from_u64(2));
    b.set(1, 1, Tropical::from_u64(0));

    let c = a.mul(&b);
    assert_eq!(c.get(0, 0), Tropical::from_u64(1));
    assert_eq!(c.get(0, 1), Tropical::from_u64(2));
    assert_eq!(c.get(1, 0), Tropical::from_u64(2));
    assert_eq!(c.get(1, 1), Tropical::from_u64(3));
}

#[test]
fn matrix_mul_with_inf() {
    // Sparse matrix multiplication
    let mut a = TropMatrix::new(2);
    a.set(0, 0, Tropical::from_u64(1));
    // a[0][1] = INF, a[1][0] = INF
    a.set(1, 1, Tropical::from_u64(2));

    let mut b = TropMatrix::new(2);
    b.set(0, 1, Tropical::from_u64(3));
    b.set(1, 0, Tropical::from_u64(4));

    let c = a.mul(&b);
    assert_eq!(c.get(0, 0), Tropical::INF);
    assert_eq!(c.get(0, 1), Tropical::from_u64(4)); // 1+3
    assert_eq!(c.get(1, 0), Tropical::from_u64(6)); // 2+4
    assert_eq!(c.get(1, 1), Tropical::INF);
}

#[test]
fn matrix_power_zero() {
    let mut a = TropMatrix::new(2);
    a.set(0, 0, Tropical::from_u64(5));
    a.set(0, 1, Tropical::from_u64(3));
    let r = a.power(0);
    // Should be identity
    assert_eq!(r.get(0, 0), Tropical::ONE);
    assert_eq!(r.get(0, 1), Tropical::INF);
    assert_eq!(r.get(1, 1), Tropical::ONE);
}

#[test]
fn matrix_power_one() {
    let mut a = TropMatrix::new(2);
    a.set(0, 0, Tropical::from_u64(5));
    a.set(0, 1, Tropical::from_u64(3));
    a.set(1, 0, Tropical::from_u64(2));
    a.set(1, 1, Tropical::from_u64(7));
    let r = a.power(1);
    assert_eq!(r.get(0, 0), Tropical::from_u64(5));
    assert_eq!(r.get(0, 1), Tropical::from_u64(3));
}

#[test]
fn matrix_power_two() {
    // A^2 = A * A (tropical)
    let mut a = TropMatrix::new(2);
    a.set(0, 0, Tropical::from_u64(1));
    a.set(0, 1, Tropical::from_u64(3));
    a.set(1, 0, Tropical::from_u64(2));
    a.set(1, 1, Tropical::from_u64(4));
    let r = a.power(2);
    let expected = a.mul(&a);
    for i in 0..2 {
        for j in 0..2 {
            assert_eq!(r.get(i, j), expected.get(i, j));
        }
    }
}

#[test]
fn matrix_from_weights() {
    let m = TropMatrix::from_weights(3, &[(0, 1, 5), (1, 2, 3), (0, 2, 10)]);
    assert_eq!(m.get(0, 1), Tropical::from_u64(5));
    assert_eq!(m.get(1, 2), Tropical::from_u64(3));
    assert_eq!(m.get(0, 2), Tropical::from_u64(10));
    assert_eq!(m.get(0, 0), Tropical::INF);
}

#[test]
fn matrix_from_weights_duplicate_takes_min() {
    let m = TropMatrix::from_weights(2, &[(0, 1, 5), (0, 1, 3)]);
    assert_eq!(m.get(0, 1), Tropical::from_u64(3));
}

#[test]
fn matrix_add_zero() {
    let mut a = TropMatrix::new(2);
    a.set(0, 0, Tropical::from_u64(3));
    a.set(1, 1, Tropical::from_u64(7));
    let zero = TropMatrix::new(2); // all INF
    let r = a.add(&zero);
    assert_eq!(r.get(0, 0), Tropical::from_u64(3));
    assert_eq!(r.get(1, 1), Tropical::from_u64(7));
}

// ============================================================
// Kleene star tests
// ============================================================

#[test]
fn kleene_star_single_node() {
    let m = TropMatrix::new(1);
    let star = kleene_star(&m);
    assert_eq!(star.get(0, 0), Tropical::ONE); // distance 0 -> 0 is 0
}

#[test]
fn kleene_star_triangle() {
    // 0 --(1)--> 1 --(2)--> 2 --(3)--> 0
    let m = TropMatrix::from_weights(3, &[(0, 1, 1), (1, 2, 2), (2, 0, 3)]);
    let star = kleene_star(&m);
    assert_eq!(star.get(0, 0), Tropical::ONE); // self-loop: 0
    assert_eq!(star.get(0, 1), Tropical::from_u64(1)); // 0 -> 1: weight 1
    assert_eq!(star.get(0, 2), Tropical::from_u64(3)); // 0 -> 1 -> 2: weight 3
    assert_eq!(star.get(1, 0), Tropical::from_u64(5)); // 1 -> 2 -> 0: weight 5
    assert_eq!(star.get(2, 0), Tropical::from_u64(3)); // 2 -> 0: weight 3
}

#[test]
fn kleene_star_shortest_path_diamond() {
    // 0 --(1)--> 1 --(1)--> 3
    // 0 --(2)--> 2 --(1)--> 3
    // Shortest 0->3: via 1, cost = 2
    let m = TropMatrix::from_weights(4, &[(0, 1, 1), (0, 2, 2), (1, 3, 1), (2, 3, 1)]);
    let star = kleene_star(&m);
    assert_eq!(star.get(0, 3), Tropical::from_u64(2));
    assert_eq!(star.get(0, 2), Tropical::from_u64(2));
}

#[test]
fn kleene_star_disconnected() {
    // 0 --(5)--> 1, no path to 2
    let m = TropMatrix::from_weights(3, &[(0, 1, 5)]);
    let star = kleene_star(&m);
    assert_eq!(star.get(0, 1), Tropical::from_u64(5));
    assert_eq!(star.get(0, 2), Tropical::INF); // unreachable
    assert_eq!(star.get(1, 0), Tropical::INF); // unreachable
}

#[test]
fn kleene_star_complete_graph() {
    // K3 with weights: 0->1=2, 0->2=4, 1->0=3, 1->2=1, 2->0=5, 2->1=6
    let m = TropMatrix::from_weights(
        3,
        &[
            (0, 1, 2),
            (0, 2, 4),
            (1, 0, 3),
            (1, 2, 1),
            (2, 0, 5),
            (2, 1, 6),
        ],
    );
    let star = kleene_star(&m);
    assert_eq!(star.get(0, 0), Tropical::ONE);
    assert_eq!(star.get(0, 1), Tropical::from_u64(2));
    assert_eq!(star.get(0, 2), Tropical::from_u64(3)); // 0->1->2 = 2+1 = 3
    assert_eq!(star.get(1, 0), Tropical::from_u64(3));
    assert_eq!(star.get(2, 1), Tropical::from_u64(6));
}

#[test]
fn kleene_star_identity_idempotent() {
    let id = TropMatrix::identity(3);
    let star = kleene_star(&id);
    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(star.get(i, j), id.get(i, j));
        }
    }
}

// ============================================================
// Eigenvalue tests
// ============================================================

#[test]
fn eigenvalue_self_loop() {
    // Single node with self-loop weight 5.
    // Only cycle: length 1, weight 5. Mean = 5.
    let m = TropMatrix::from_weights(1, &[(0, 0, 5)]);
    assert_eq!(eigenvalue(&m), Tropical::from_u64(5));
}

#[test]
fn eigenvalue_triangle_uniform() {
    // Cycle: 0->1->2->0 with weight 3+3+3=9, length 3. Mean = 3.
    let m = TropMatrix::from_weights(3, &[(0, 1, 3), (1, 2, 3), (2, 0, 3)]);
    assert_eq!(eigenvalue(&m), Tropical::from_u64(3));
}

#[test]
fn eigenvalue_two_node_cycle() {
    // 0->1 weight 2, 1->0 weight 4. Cycle weight = 6, length 2. Mean = 3.
    let m = TropMatrix::from_weights(2, &[(0, 1, 2), (1, 0, 4)]);
    assert_eq!(eigenvalue(&m), Tropical::from_u64(3));
}

#[test]
fn eigenvalue_no_cycle() {
    // DAG: 0->1->2, no cycles
    let m = TropMatrix::from_weights(3, &[(0, 1, 1), (1, 2, 2)]);
    assert_eq!(eigenvalue(&m), Tropical::INF);
}

#[test]
fn eigenvalue_self_loop_zero() {
    // Self-loop of weight 0: mean = 0.
    let m = TropMatrix::from_weights(1, &[(0, 0, 0)]);
    assert_eq!(eigenvalue(&m), Tropical::from_u64(0));
}

#[test]
fn eigenvalue_multiple_cycles() {
    // 0->1->0 weight 2+4=6 length 2, mean=3
    // 0->1->2->0 weight 2+1+3=6 length 3, mean=2
    // Minimum mean cycle weight = 2
    let m = TropMatrix::from_weights(3, &[(0, 1, 2), (1, 0, 4), (1, 2, 1), (2, 0, 3)]);
    assert_eq!(eigenvalue(&m), Tropical::from_u64(2));
}

// ============================================================
// Determinant tests
// ============================================================

#[test]
fn determinant_1x1() {
    let mut m = TropMatrix::new(1);
    m.set(0, 0, Tropical::from_u64(7));
    assert_eq!(determinant(&m), Tropical::from_u64(7));
}

#[test]
fn determinant_identity() {
    let id = TropMatrix::identity(3);
    assert_eq!(determinant(&id), Tropical::from_u64(0));
}

#[test]
fn determinant_2x2() {
    // [[1, 3], [2, 4]]
    // perm (0,1): 1+4=5
    // perm (1,0): 3+2=5
    // det = min(5, 5) = 5
    let mut m = TropMatrix::new(2);
    m.set(0, 0, Tropical::from_u64(1));
    m.set(0, 1, Tropical::from_u64(3));
    m.set(1, 0, Tropical::from_u64(2));
    m.set(1, 1, Tropical::from_u64(4));
    assert_eq!(determinant(&m), Tropical::from_u64(5));
}

#[test]
fn determinant_2x2_asymmetric() {
    // [[1, 10], [2, 4]]
    // perm (0,1): 1+4=5
    // perm (1,0): 10+2=12
    // det = 5
    let mut m = TropMatrix::new(2);
    m.set(0, 0, Tropical::from_u64(1));
    m.set(0, 1, Tropical::from_u64(10));
    m.set(1, 0, Tropical::from_u64(2));
    m.set(1, 1, Tropical::from_u64(4));
    assert_eq!(determinant(&m), Tropical::from_u64(5));
}

#[test]
fn determinant_3x3() {
    // Assignment problem:
    // [[0, 1, 2],
    //  [2, 0, 1],
    //  [1, 2, 0]]
    // All permutations yield: identity=0+0+0=0, which is optimal
    let mut m = TropMatrix::new(3);
    m.set(0, 0, Tropical::from_u64(0));
    m.set(0, 1, Tropical::from_u64(1));
    m.set(0, 2, Tropical::from_u64(2));
    m.set(1, 0, Tropical::from_u64(2));
    m.set(1, 1, Tropical::from_u64(0));
    m.set(1, 2, Tropical::from_u64(1));
    m.set(2, 0, Tropical::from_u64(1));
    m.set(2, 1, Tropical::from_u64(2));
    m.set(2, 2, Tropical::from_u64(0));
    assert_eq!(determinant(&m), Tropical::from_u64(0));
}

#[test]
fn determinant_with_inf() {
    // [[0, INF], [INF, 0]]
    // perm (0,1): 0+0=0
    // perm (1,0): INF+INF=INF
    // det = 0
    let mut m = TropMatrix::new(2);
    m.set(0, 0, Tropical::from_u64(0));
    m.set(1, 1, Tropical::from_u64(0));
    assert_eq!(determinant(&m), Tropical::from_u64(0));
}

#[test]
fn determinant_all_inf() {
    let m = TropMatrix::new(2);
    assert_eq!(determinant(&m), Tropical::INF);
}

#[test]
fn determinant_empty() {
    let m = TropMatrix::new(0);
    assert_eq!(determinant(&m), Tropical::ONE);
}

// ============================================================
// Dual (MaxPlus) tests
// ============================================================

#[test]
fn maxplus_add_basic() {
    let a = MaxPlus::from_val(3);
    let b = MaxPlus::from_val(7);
    assert_eq!(a.add(b), MaxPlus::from_val(7));
}

#[test]
fn maxplus_add_identity() {
    let a = MaxPlus::from_val(5);
    assert_eq!(MaxPlus::ZERO.add(a), a);
    assert_eq!(a.add(MaxPlus::ZERO), a);
}

#[test]
fn maxplus_mul_basic() {
    let a = MaxPlus::from_val(3);
    let b = MaxPlus::from_val(7);
    assert_eq!(a.mul(b).to_val(), Some(10));
}

#[test]
fn maxplus_mul_identity() {
    let a = MaxPlus::from_val(5);
    assert_eq!(a.mul(MaxPlus::ONE).to_val(), Some(5));
    assert_eq!(MaxPlus::ONE.mul(a).to_val(), Some(5));
}

#[test]
fn maxplus_mul_zero() {
    let a = MaxPlus::from_val(5);
    assert_eq!(a.mul(MaxPlus::ZERO), MaxPlus::ZERO);
    assert_eq!(MaxPlus::ZERO.mul(a), MaxPlus::ZERO);
}

#[test]
fn maxplus_is_neg_inf() {
    assert!(MaxPlus::ZERO.is_neg_inf());
    assert!(MaxPlus::NEG_INF.is_neg_inf());
    assert!(!MaxPlus::ONE.is_neg_inf());
}

#[test]
fn maxplus_to_val() {
    assert_eq!(MaxPlus::ZERO.to_val(), None);
    assert_eq!(MaxPlus::ONE.to_val(), Some(0));
    assert_eq!(MaxPlus::from_val(42).to_val(), Some(42));
}

#[test]
fn maxplus_commutativity() {
    let a = MaxPlus::from_val(10);
    let b = MaxPlus::from_val(20);
    assert_eq!(a.add(b), b.add(a));
    assert_eq!(a.mul(b), b.mul(a));
}

#[test]
fn maxplus_constants() {
    assert_eq!(MaxPlus::ZERO.0, 0);
    assert_eq!(MaxPlus::ONE.0, 1);
    assert_eq!(MaxPlus::NEG_INF.0, 0);
}

// ============================================================
// Encoding tests
// ============================================================

#[test]
fn encoding_element_roundtrip() {
    let t = Tropical::from_u64(0x0102030405060708);
    let bytes = decode_element(t);
    let recovered = encode_element(&bytes);
    assert_eq!(recovered, t);
}

#[test]
fn encoding_element_inf() {
    let bytes = decode_element(Tropical::INF);
    assert_eq!(bytes, [0xFF; 8]);
    let recovered = encode_element(&bytes);
    assert_eq!(recovered, Tropical::INF);
}

#[test]
fn encoding_element_zero() {
    let bytes = decode_element(Tropical::ONE);
    assert_eq!(bytes, [0; 8]);
}

#[test]
fn encoding_matrix_roundtrip() {
    let mut m = TropMatrix::new(2);
    m.set(0, 0, Tropical::from_u64(1));
    m.set(0, 1, Tropical::from_u64(2));
    m.set(1, 0, Tropical::from_u64(3));
    m.set(1, 1, Tropical::from_u64(4));

    let mut buf = [0u8; 2 * 2 * 8];
    decode_matrix(&m, &mut buf);
    let recovered = encode_matrix(2, &buf);
    for i in 0..2 {
        for j in 0..2 {
            assert_eq!(recovered.get(i, j), m.get(i, j));
        }
    }
}

#[test]
fn encoding_matrix_with_inf() {
    let m = TropMatrix::new(2); // all INF
    let mut buf = [0u8; 2 * 2 * 8];
    decode_matrix(&m, &mut buf);
    // All bytes should be 0xFF
    for &b in buf.iter() {
        assert_eq!(b, 0xFF);
    }
}

// ============================================================
// Integration / property tests
// ============================================================

#[test]
fn integration_shortest_path_4node() {
    // Graph:
    //   0 --(1)--> 1 --(2)--> 3
    //   0 --(4)--> 2 --(1)--> 3
    //   1 --(1)--> 2
    // Shortest 0->3: 0->1->2->3 = 1+1+1 = 3? Wait:
    // 0->1=1, 1->2=1, 2->3=1 => total=3
    // 0->1=1, 1->3=2 => total=3
    // 0->2=4, 2->3=1 => total=5
    // Shortest = 3 (both paths)
    let m = TropMatrix::from_weights(4, &[(0, 1, 1), (1, 3, 2), (0, 2, 4), (2, 3, 1), (1, 2, 1)]);
    let star = kleene_star(&m);
    assert_eq!(star.get(0, 3), Tropical::from_u64(3));
}

#[test]
fn integration_power_vs_manual() {
    // Verify A^3 via power matches A*A*A
    let m = TropMatrix::from_weights(3, &[(0, 1, 2), (1, 2, 3), (2, 0, 1)]);
    let cube_power = m.power(3);
    let cube_manual = m.mul(&m).mul(&m);
    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(cube_power.get(i, j), cube_manual.get(i, j));
        }
    }
}

#[test]
fn integration_kleene_star_via_powers() {
    // For a 3-node graph, A* = I + A + A^2
    let m = TropMatrix::from_weights(3, &[(0, 1, 2), (1, 2, 3)]);
    let star = kleene_star(&m);
    let id = TropMatrix::identity(3);
    let a1 = m.clone();
    let a2 = m.mul(&m);
    let manual_star = id.add(&a1).add(&a2);
    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(star.get(i, j), manual_star.get(i, j));
        }
    }
}

#[test]
fn integration_matrix_mul_zero() {
    // A * zero_matrix = zero_matrix (since INF absorbs)
    let mut a = TropMatrix::new(2);
    a.set(0, 0, Tropical::from_u64(1));
    a.set(0, 1, Tropical::from_u64(2));
    a.set(1, 0, Tropical::from_u64(3));
    a.set(1, 1, Tropical::from_u64(4));
    let zero = TropMatrix::new(2);
    let r = a.mul(&zero);
    for i in 0..2 {
        for j in 0..2 {
            assert_eq!(r.get(i, j), Tropical::INF);
        }
    }
}

#[test]
fn integration_determinant_permutation_matrix() {
    // Permutation matrix (tropical): perm = (1, 2, 0)
    // [[INF, 0, INF], [INF, INF, 0], [0, INF, INF]]
    // Only valid perm: sigma=(1,2,0) with cost 0+0+0=0
    let mut m = TropMatrix::new(3);
    m.set(0, 1, Tropical::from_u64(0));
    m.set(1, 2, Tropical::from_u64(0));
    m.set(2, 0, Tropical::from_u64(0));
    assert_eq!(determinant(&m), Tropical::from_u64(0));
}
