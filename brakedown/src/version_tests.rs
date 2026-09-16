use super::*;

#[test]
fn opening_checks_each_unique_payload_and_preserves_all_transcript_draws() {
    assert_eq!(OPENING_VERSION, 3);
    let poly = MultilinearPoly::new((1..=16).map(Goldilocks::new).collect());
    let point = vec![Goldilocks::new(3); 4];
    let commitment = Brakedown::commit(&poly);
    let value = poly.evaluate(&point);
    let mut prover_transcript = Transcript::new(b"unique-column-wire");
    let proof = Brakedown::open(&poly, &point, &mut prover_transcript);
    let mut verifier_transcript = Transcript::new(b"unique-column-wire");
    assert!(Brakedown::verify(
        &commitment,
        &point,
        value,
        &proof,
        &mut verifier_transcript
    ));
    assert_eq!(prover_transcript.squeeze(), verifier_transcript.squeeze());
    let Opening::TensorMerkle {
        row_combination,
        columns,
    } = &proof
    else {
        unreachable!()
    };
    assert!(columns.len() <= 12);
    assert!(columns.windows(2).all(|w| w[0].index < w[1].index));
    let check = |columns: Vec<ColumnQuery>| {
        Brakedown::verify(
            &commitment,
            &point,
            value,
            &Opening::TensorMerkle {
                row_combination: row_combination.clone(),
                columns,
            },
            &mut Transcript::new(b"unique-column-wire"),
        )
    };
    let mut duplicate = columns.clone();
    duplicate.insert(0, columns[0].clone());
    assert!(!check(duplicate));
    assert!(!check(columns[1..].to_vec()));
    let mut reordered = columns.clone();
    reordered.swap(0, 1);
    assert!(!check(reordered));
    for index in 0..columns.len() {
        let mut changed = columns.clone();
        changed[index].column[0] ^= 1;
        assert!(!check(changed));
        let mut changed = columns.clone();
        changed[index].path.pop();
        assert!(!check(changed));
    }
}

#[test]
fn systematic_encoding_is_linear_injective_and_exposes_original_symbols() {
    for n in [1, 2, 3, 8, 16, 31, 64] {
        let exp = Expander::new(n);
        let a: Vec<_> = (0..n).map(|i| Goldilocks::new(i as u64 + 1)).collect();
        let b: Vec<_> = (0..n).map(|i| -Goldilocks::new(i as u64 + 7)).collect();
        let sum: Vec<_> = a.iter().zip(&b).map(|(&a, &b)| a + b).collect();
        let ea = exp.encode(&a);
        assert_eq!(&ea[..n], &a);
        let eb = exp.encode(&b);
        assert_eq!(
            exp.encode(&sum),
            ea.iter().zip(&eb).map(|(&a, &b)| a + b).collect::<Vec<_>>()
        );
        for index in 0..n {
            let mut changed = a.clone();
            changed[index] += Goldilocks::ONE;
            let code = exp.encode(&changed);
            assert_eq!(code[index] - ea[index], Goldilocks::ONE);
            assert_ne!(code, ea);
        }
    }
    // Exhaustive collision check over all eight-symbol binary tables.
    let mut seen = std::collections::BTreeSet::new();
    let exp = Expander::new(8);
    for bits in 0..256u64 {
        let input: Vec<_> = (0..8).map(|i| Goldilocks::new((bits >> i) & 1)).collect();
        assert!(seen.insert(Brakedown::serialize(&exp.encode(&input))));
    }
}

#[test]
fn raw_commitment_authenticates_every_symbol_and_separates_version_length_and_backend() {
    assert_eq!(COMMITMENT_VERSION, 2);
    let values: Vec<_> = (1..=16).map(Goldilocks::new).collect();
    let commitment = Brakedown::commit_raw(&values);
    let (again, encoded) = Brakedown::commit_raw_with_codeword(&values);
    assert_eq!(commitment, again);
    assert_eq!(&encoded[..values.len()], &values);
    // Legacy raw roots only hashed the parity vector and must not match.
    let old = Commitment(cyber_hemera::hash(&Brakedown::serialize(
        &encoded[values.len()..],
    )));
    assert_ne!(commitment, old);
    for i in 0..values.len() {
        let mut altered = values.clone();
        altered[i] += Goldilocks::ONE;
        assert_ne!(commitment, Brakedown::commit_raw(&altered));
    }
    assert_ne!(
        Brakedown::commit_raw(&[]),
        Brakedown::commit_raw(&[Goldilocks::ZERO])
    );
    assert_ne!(commitment, Brakedown::commit(&MultilinearPoly::new(values)));
}

#[test]
fn tensor_systematic_columns_authenticate_raw_table_and_reject_legacy_root() {
    let values: Vec<_> = (1..=16).map(Goldilocks::new).collect();
    let poly = MultilinearPoly::new(values.clone());
    let matrix = Brakedown::build_matrix(&values, 2);
    assert_eq!(matrix.columns.len(), 12);
    for (index, raw) in values.chunks_exact(4).enumerate() {
        assert_eq!(matrix.columns[index], raw);
        let bytes = Brakedown::serialize(raw);
        let root = merkle::column_root(index, 12, &bytes, &matrix.tree.path(index)).unwrap();
        assert_eq!(
            tensor_commitment(root, poly.num_vars),
            Brakedown::commit(&poly)
        );
        let mut changed = raw.to_vec();
        changed[0] += Goldilocks::ONE;
        let forged = merkle::column_root(
            index,
            12,
            &Brakedown::serialize(&changed),
            &matrix.tree.path(index),
        )
        .unwrap();
        assert_ne!(
            tensor_commitment(forged, poly.num_vars),
            Brakedown::commit(&poly)
        );
    }
    let point = vec![Goldilocks::new(3); 4];
    let proof = Brakedown::open(&poly, &point, &mut Transcript::new(b"version"));
    assert!(!Brakedown::verify(
        &Commitment(matrix.tree.root()),
        &point,
        poly.evaluate(&point),
        &proof,
        &mut Transcript::new(b"version")
    ));
    assert_ne!(
        tensor_commitment(matrix.tree.root(), 3),
        Brakedown::commit(&poly)
    );
}

#[test]
fn malformed_field_bytes_paths_and_dimensions_are_rejected() {
    let poly = MultilinearPoly::new(vec![Goldilocks::ONE; 4]);
    let point = vec![Goldilocks::new(3); 2];
    let commitment = Brakedown::commit(&poly);
    let proof = Brakedown::open(&poly, &point, &mut Transcript::new(b"canonical"));
    let rejects = |proof: &Opening| {
        !Brakedown::verify(
            &commitment,
            &point,
            Goldilocks::ONE,
            proof,
            &mut Transcript::new(b"canonical"),
        )
    };
    for mutation in 0..4 {
        let mut bad = proof.clone();
        let Opening::TensorMerkle {
            row_combination,
            columns,
        } = &mut bad
        else {
            unreachable!()
        };
        match mutation {
            0 => row_combination.push(0),
            1 => row_combination[..8].copy_from_slice(&nebu::field::P.to_le_bytes()),
            2 => {
                columns[0].path.pop();
            }
            _ => columns[0].column.push(0),
        }
        assert!(rejects(&bad));
    }
    assert!(!Brakedown::verify(
        &commitment,
        &vec![Goldilocks::ONE; MAX_VARIABLES + 1],
        Goldilocks::ONE,
        &proof,
        &mut Transcript::new(b"canonical")
    ));
}

#[test]
fn unique_queries_consume_the_full_independent_sampling_schedule() {
    for k2 in [1, 2, 4, 16] {
        let mut actual = Transcript::new(b"draw-schedule-v3");
        let mut reference = Transcript::new(b"draw-schedule-v3");
        let m = expander::EXPANSION * k2;
        let mut expected = std::collections::BTreeSet::new();
        // Independent schedule: do not call num_queries or query_indices here.
        for _ in 0..100 * m {
            let challenge = reference.squeeze();
            let word = u64::from_le_bytes(challenge.as_bytes()[..8].try_into().unwrap());
            expected.insert((word % m as u64) as usize);
        }
        assert_eq!(query_indices(&mut actual, k2), expected);
        assert_eq!(actual.squeeze(), reference.squeeze());
    }
}

#[test]
fn dense_legacy_and_noncanonical_column_payloads_are_rejected() {
    let poly = MultilinearPoly::new(vec![Goldilocks::ONE; 4]);
    let point = vec![Goldilocks::new(3); 2];
    let commitment = Brakedown::commit(&poly);
    let mut proof = Brakedown::open(&poly, &point, &mut Transcript::new(b"bounded-v3"));
    let rejects = |proof: &Opening| {
        !Brakedown::verify(
            &commitment,
            &point,
            Goldilocks::ONE,
            proof,
            &mut Transcript::new(b"bounded-v3"),
        )
    };
    let Opening::TensorMerkle { columns, .. } = &mut proof else {
        unreachable!()
    };
    let original = columns[0].column.clone();
    columns[0].column = vec![0; 1024 * 1024];
    assert!(rejects(&proof));
    let Opening::TensorMerkle { columns, .. } = &mut proof else {
        unreachable!()
    };
    columns[0].column = original;
    columns[0].column[..8].copy_from_slice(&nebu::field::P.to_le_bytes());
    assert!(rejects(&proof));
    let Opening::TensorMerkle { columns, .. } = &mut proof else {
        unreachable!()
    };
    *columns = vec![columns[0].clone(); 600];
    assert!(rejects(&proof));
}

#[cfg(feature = "serde")]
#[test]
fn v3_unique_opening_roundtrips_without_accepting_dense_legacy_payload() {
    let poly = MultilinearPoly::new(vec![Goldilocks::ONE; 4]);
    let point = vec![Goldilocks::new(3); 2];
    let commitment = Brakedown::commit(&poly);
    let proof = Brakedown::open(&poly, &point, &mut Transcript::new(b"v3-serde"));
    let bytes = postcard::to_allocvec(&proof).unwrap();
    let decoded: Opening = postcard::from_bytes(&bytes).unwrap();
    assert!(Brakedown::verify(
        &commitment,
        &point,
        Goldilocks::ONE,
        &decoded,
        &mut Transcript::new(b"v3-serde")
    ));
    let Opening::TensorMerkle {
        row_combination,
        columns,
    } = decoded
    else {
        unreachable!()
    };
    let legacy = Opening::TensorMerkle {
        row_combination,
        columns: vec![columns[0].clone(); 600],
    };
    let bytes = postcard::to_allocvec(&legacy).unwrap();
    let decoded: Opening = postcard::from_bytes(&bytes).unwrap();
    assert!(!Brakedown::verify(
        &commitment,
        &point,
        Goldilocks::ONE,
        &decoded,
        &mut Transcript::new(b"v3-serde")
    ));
}
