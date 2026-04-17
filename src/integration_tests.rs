//! Integration tests for all five commitment constructions.
//!
//! Tests the full commit → open → verify pipeline across
//! multiple polynomial sizes, evaluation points, and constructions.

use crate::*;

// ── generic test helpers ─────────────────────────────────────────

fn roundtrip<F: Field + core::fmt::Debug, L: Lens<F>>(
    poly: &MultilinearPoly<F>,
    point: &[F],
    domain: &[u8],
) {
    let commitment = L::commit(poly);
    let value = poly.evaluate(point);

    let mut pt = Transcript::new(domain);
    let proof = L::open(poly, point, &mut pt);

    let mut vt = Transcript::new(domain);
    assert!(
        L::verify(&commitment, point, value, &proof, &mut vt),
        "roundtrip failed for {}-var poly",
        poly.num_vars
    );
}

fn soundness_wrong_value<F: Field + core::fmt::Debug, L: Lens<F>>(
    poly: &MultilinearPoly<F>,
    point: &[F],
    domain: &[u8],
) {
    let commitment = L::commit(poly);
    let value = poly.evaluate(point);
    let wrong = value + F::ONE;

    let mut pt = Transcript::new(domain);
    let proof = L::open(poly, point, &mut pt);

    let mut vt = Transcript::new(domain);
    assert!(
        !L::verify(&commitment, point, wrong, &proof, &mut vt),
        "wrong value should be rejected"
    );
}

fn soundness_wrong_commitment<F: Field + core::fmt::Debug, L: Lens<F>>(
    poly: &MultilinearPoly<F>,
    point: &[F],
    domain: &[u8],
) {
    let value = poly.evaluate(point);
    let fake = Commitment(cyber_hemera::hash(b"fake"));

    let mut pt = Transcript::new(domain);
    let proof = L::open(poly, point, &mut pt);

    let mut vt = Transcript::new(domain);
    assert!(
        !L::verify(&fake, point, value, &proof, &mut vt),
        "wrong commitment should be rejected"
    );
}

// ── Brakedown ────────────────────────────────────────────────────

mod brakedown_tests {
    use super::*;
    use nebu::Goldilocks;

    fn poly(nv: usize, seed: u64) -> MultilinearPoly<Goldilocks> {
        let evals = (0..1usize << nv)
            .map(|i| {
                Goldilocks::new(
                    seed.wrapping_mul(0x9E3779B97F4A7C15)
                        .wrapping_add(i as u64)
                        .wrapping_mul(0x517CC1B727220A95),
                )
                .canonicalize()
            })
            .collect();
        MultilinearPoly::new(evals)
    }

    fn pt(nv: usize, seed: u64) -> Vec<Goldilocks> {
        (0..nv)
            .map(|i| Goldilocks::new(seed.wrapping_add(i as u64 * 7 + 3)).canonicalize())
            .collect()
    }

    #[test]
    fn roundtrip_1() {
        roundtrip::<_, brakedown::Brakedown>(&poly(1, 10), &pt(1, 20), b"b1");
    }
    #[test]
    fn roundtrip_2() {
        roundtrip::<_, brakedown::Brakedown>(&poly(2, 11), &pt(2, 21), b"b2");
    }
    #[test]
    fn roundtrip_4() {
        roundtrip::<_, brakedown::Brakedown>(&poly(4, 12), &pt(4, 22), b"b4");
    }
    #[test]
    fn roundtrip_8() {
        roundtrip::<_, brakedown::Brakedown>(&poly(8, 13), &pt(8, 23), b"b8");
    }
    #[test]
    fn roundtrip_10() {
        roundtrip::<_, brakedown::Brakedown>(&poly(10, 14), &pt(10, 24), b"b10");
    }
    #[test]
    fn roundtrip_12() {
        roundtrip::<_, brakedown::Brakedown>(&poly(12, 15), &pt(12, 25), b"b12");
    }
    #[test]
    fn roundtrip_14() {
        roundtrip::<_, brakedown::Brakedown>(&poly(14, 16), &pt(14, 26), b"b14");
    }

    #[test]
    fn sound_value() {
        soundness_wrong_value::<_, brakedown::Brakedown>(&poly(4, 30), &pt(4, 40), b"bsv");
    }
    #[test]
    fn sound_commit() {
        soundness_wrong_commitment::<_, brakedown::Brakedown>(&poly(4, 50), &pt(4, 60), b"bsc");
    }

    #[test]
    fn zero_poly() {
        let p = MultilinearPoly::new(vec![Goldilocks::ZERO; 8]);
        let point = pt(3, 70);
        assert_eq!(p.evaluate(&point), Goldilocks::ZERO);
        roundtrip::<_, brakedown::Brakedown>(&p, &point, b"bz");
    }

    #[test]
    fn boolean_evals_match_table() {
        let p = poly(3, 80);
        for i in 0..8usize {
            let bpt: Vec<Goldilocks> = (0..3)
                .map(|j| {
                    if (i >> j) & 1 == 1 {
                        Goldilocks::ONE
                    } else {
                        Goldilocks::ZERO
                    }
                })
                .collect();
            assert_eq!(p.evaluate(&bpt), p.evals[i], "mismatch at {i}");
        }
    }
}

// ── Binius ───────────────────────────────────────────────────────

mod binius_tests {
    use super::*;
    use kuro::F2_128;

    #[test]
    fn roundtrip_1() {
        let p = MultilinearPoly::new(vec![F2_128(0xDEAD), F2_128(0xBEEF)]);
        roundtrip::<_, binius::Binius>(&p, &[F2_128::ZERO], b"bi1");
    }

    #[test]
    fn roundtrip_2() {
        let p = MultilinearPoly::new(vec![F2_128(1), F2_128(2), F2_128(3), F2_128(4)]);
        roundtrip::<_, binius::Binius>(&p, &[F2_128::ONE, F2_128::ZERO], b"bi2");
    }

    #[test]
    fn roundtrip_4() {
        let evals: Vec<F2_128> = (0..16).map(|i| F2_128(i * 0x1111 + 1)).collect();
        let p = MultilinearPoly::new(evals);
        roundtrip::<_, binius::Binius>(&p, &[F2_128(5), F2_128(11), F2_128(7), F2_128(3)], b"bi4");
    }

    #[test]
    fn sound_value() {
        let p = MultilinearPoly::new(vec![F2_128(1), F2_128(2), F2_128(3), F2_128(4)]);
        soundness_wrong_value::<_, binius::Binius>(&p, &[F2_128::ZERO, F2_128::ZERO], b"bis");
    }

    #[test]
    fn char2_laws() {
        let a = F2_128(0x12345678ABCDEF00);
        assert_eq!(a + a, F2_128::ZERO, "a + a = 0 in char 2");
        assert_eq!(-a, a, "-a = a in char 2");
        assert_eq!(a - a, F2_128::ZERO, "a - a = 0 in char 2");
    }
}

// ── Porphyry ─────────────────────────────────────────────────────

mod porphyry_tests {
    use super::*;
    use genies::Fq;

    #[test]
    fn roundtrip_1() {
        let p = MultilinearPoly::new(vec![Fq::from_u64(42), Fq::from_u64(99)]);
        roundtrip::<_, porphyry::Porphyry>(&p, &[Fq::ZERO], b"po1");
    }

    #[test]
    fn roundtrip_2() {
        let p = MultilinearPoly::new(vec![
            Fq::from_u64(1),
            Fq::from_u64(2),
            Fq::from_u64(3),
            Fq::from_u64(4),
        ]);
        roundtrip::<_, porphyry::Porphyry>(&p, &[Fq::ONE, Fq::ONE], b"po2");
    }

    #[test]
    fn sound_value() {
        let p = MultilinearPoly::new(vec![Fq::from_u64(10), Fq::from_u64(20)]);
        soundness_wrong_value::<_, porphyry::Porphyry>(&p, &[Fq::ZERO], b"pos");
    }
}

// ── Ikat ─────────────────────────────────────────────────────────

mod ikat_tests {
    use super::*;
    use nebu::Goldilocks;

    #[test]
    fn equivalence_with_brakedown() {
        let evals: Vec<Goldilocks> = (0..16).map(|i| Goldilocks::new(i + 1)).collect();
        let p = MultilinearPoly::new(evals);
        assert_eq!(
            ikat::Ikat::commit(&p),
            brakedown::Brakedown::commit(&p),
            "Ikat and Brakedown commitments must match"
        );
    }

    #[test]
    fn roundtrip_4() {
        let evals: Vec<Goldilocks> = (0..16).map(|i| Goldilocks::new(i * 7 + 5)).collect();
        let p = MultilinearPoly::new(evals);
        let pt: Vec<Goldilocks> = (0..4).map(|i| Goldilocks::new(i + 2)).collect();
        roundtrip::<_, ikat::Ikat>(&p, &pt, b"ik4");
    }
}

// ── Assayer ──────────────────────────────────────────────────────

mod assayer_tests {
    use super::*;
    use assayer::{Assayer, DualCertificate, Edge, TropicalWitness};
    use nebu::Goldilocks;
    use trop::Tropical;

    fn graph_3_2_4() -> (TropicalWitness, DualCertificate) {
        // 0 →3→ 1 →2→ 2 →4→ 3, shortest 0→3 = 9
        let w = TropicalWitness {
            num_vertices: 4,
            edges: vec![
                Edge {
                    from: 0,
                    to: 1,
                    weight: Tropical::from_u64(3),
                },
                Edge {
                    from: 1,
                    to: 2,
                    weight: Tropical::from_u64(2),
                },
                Edge {
                    from: 2,
                    to: 3,
                    weight: Tropical::from_u64(4),
                },
                Edge {
                    from: 0,
                    to: 2,
                    weight: Tropical::from_u64(10),
                },
                Edge {
                    from: 0,
                    to: 3,
                    weight: Tropical::from_u64(20),
                },
            ],
            assignment: vec![0, 1, 2],
            cost: Tropical::from_u64(9),
            source: 0,
            target: 3,
        };
        let c = DualCertificate {
            dual_vars: vec![
                Goldilocks::new(0),
                Goldilocks::new(3),
                Goldilocks::new(5),
                Goldilocks::new(9),
            ],
            dual_objective: Goldilocks::new(9),
        };
        (w, c)
    }

    #[test]
    fn shortest_path() {
        let (w, c) = graph_3_2_4();
        assert!(Assayer::verify_tropical(&w, &c));

        let (commitment, poly) = Assayer::commit_witness(&w, &c);
        let point = vec![Goldilocks::ZERO; poly.num_vars];
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"asp");
        let proof = Assayer::open_witness(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"asp");
        assert!(Assayer::verify_witness(
            &commitment,
            &point,
            value,
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn wrong_cost_rejected() {
        let (mut w, mut c) = graph_3_2_4();
        w.cost = Tropical::from_u64(999);
        c.dual_objective = Goldilocks::new(999);
        assert!(!Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn out_of_bounds_rejected() {
        let (mut w, c) = graph_3_2_4();
        w.assignment = vec![0, 999]; // edge 999 doesn't exist
        assert!(!Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn dual_feasibility_violation() {
        let (w, mut c) = graph_3_2_4();
        c.dual_vars[2] = Goldilocks::new(100); // violates d[2] ≤ d[1] + w(1→2) = 5
        assert!(!Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn broken_path_rejected() {
        let (mut w, c) = graph_3_2_4();
        w.assignment = vec![0, 2]; // 0→1 then 2→3, gap: 1≠2
        w.cost = Tropical::from_u64(7);
        assert!(!Assayer::verify_tropical(&w, &c));
    }
}

// ── Cross-construction ──────────────────────────────────────────

mod cross_tests {
    use super::*;
    use genies::Fq;
    use kuro::F2_128;
    use nebu::Goldilocks;

    #[test]
    fn commitment_format_uniform() {
        let gp = MultilinearPoly::new(vec![Goldilocks::new(1), Goldilocks::new(2)]);
        let bp = MultilinearPoly::new(vec![F2_128(1), F2_128(2)]);
        let fp = MultilinearPoly::new(vec![Fq::from_u64(1), Fq::from_u64(2)]);

        let gc = brakedown::Brakedown::commit(&gp);
        let bc = binius::Binius::commit(&bp);
        let fc = porphyry::Porphyry::commit(&fp);

        // all commitments same byte length (hemera Hash)
        assert_eq!(gc.as_bytes().len(), bc.as_bytes().len());
        assert_eq!(bc.as_bytes().len(), fc.as_bytes().len());
        // different data → different commitments
        assert_ne!(gc, bc);
        assert_ne!(bc, fc);
    }
}
