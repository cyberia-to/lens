//! Exact polynomial openings for PUBLIC data, with full witness disclosure.
//!
//! Every raw column is authenticated and checked. Verification is linear in
//! the table size; there is no code-distance or sampling soundness assumption.
//! This backend is not zero knowledge. See specs/public-tensor.md.

use crate::{Commitment, Lens, MultilinearPoly, Opening, Transcript};
use crate::{matrix, merkle::ColumnTree, tensor::evaluate_small};
use cyber_lens_core::ColumnQuery;
use nebu::Goldilocks;

const DOMAIN: &[u8] = b"lens-public-tensor-v1";
const MAX_VARIABLES: usize = 20;

pub struct PublicTensor;

impl PublicTensor {
    /// Recover the complete canonical public table after authenticating its
    /// versioned commitment, dimensions, every column and canonical path.
    ///
    /// This performs no VM evaluation. A proof-system caller can use the
    /// authenticated table to check its algebraic constraints exactly.
    pub fn authenticated_evaluations(
        commitment: &Commitment,
        variables: usize,
        opening: &Opening,
    ) -> Option<Vec<Goldilocks>> {
        authenticated_table(commitment, variables, opening)
    }
}

fn serialize(values: &[Goldilocks]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|v| v.as_u64().to_le_bytes())
        .collect()
}

fn decode(bytes: &[u8], elements: usize) -> Option<Vec<Goldilocks>> {
    if bytes.len() != elements.checked_mul(8)? {
        return None;
    }
    bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|chunk| {
            let value = u64::from_le_bytes(*chunk);
            (value < nebu::field::P).then(|| Goldilocks::new(value))
        })
        .collect()
}

fn commitment(tree: &ColumnTree, variables: usize) -> Commitment {
    let mut bytes = DOMAIN.to_vec();
    bytes.extend_from_slice(&(variables as u64).to_le_bytes());
    bytes.extend_from_slice(tree.root().as_bytes());
    Commitment(cyber_hemera::hash(&bytes))
}

fn columns(poly: &MultilinearPoly<Goldilocks>) -> Vec<Vec<u8>> {
    let (row_vars, _) = matrix::split_vars(poly.num_vars);
    poly.evals
        .chunks_exact(1 << row_vars)
        .map(serialize)
        .collect()
}

fn absorb_claim(t: &mut Transcript, c: &Commitment, point: &[Goldilocks], value: Goldilocks) {
    t.absorb(DOMAIN);
    t.absorb(c.as_bytes());
    t.absorb(&(point.len() as u64).to_le_bytes());
    t.absorb(&serialize(point));
    t.absorb(&value.as_u64().to_le_bytes());
}

/// Verify the whole raw table, including canonical paths. Rebuilding its root
/// also prevents a malicious tree from supplying duplicate labelled positions.
fn authenticated_table(
    c: &Commitment,
    variables: usize,
    opening: &Opening,
) -> Option<Vec<Goldilocks>> {
    if variables > MAX_VARIABLES {
        return None;
    }
    let Opening::TensorMerkle { columns, .. } = opening else {
        return None;
    };
    let (rv, cv) = matrix::split_vars(variables);
    let rows = 1 << rv;
    let count = 1 << cv;
    if columns.len() != count {
        return None;
    }
    let mut table = Vec::with_capacity(1 << variables);
    let mut bytes = Vec::with_capacity(count);
    for (index, column) in columns.iter().enumerate() {
        if column.index != index || column.path.len() != cv {
            return None;
        }
        table.extend(decode(&column.column, rows)?);
        bytes.push(column.column.clone());
    }
    let tree = ColumnTree::build(&bytes);
    if commitment(&tree, variables) != *c {
        return None;
    }
    if columns
        .iter()
        .any(|column| column.path != tree.path(column.index))
    {
        return None;
    }
    Some(table)
}

impl Lens<Goldilocks> for PublicTensor {
    fn commit(poly: &MultilinearPoly<Goldilocks>) -> Commitment {
        assert!(poly.num_vars <= MAX_VARIABLES, "public tensor size limit");
        assert_eq!(
            poly.evals.len(),
            1 << poly.num_vars,
            "invalid polynomial dimensions"
        );
        commitment(&ColumnTree::build(&columns(poly)), poly.num_vars)
    }

    fn open(
        poly: &MultilinearPoly<Goldilocks>,
        point: &[Goldilocks],
        t: &mut Transcript,
    ) -> Opening {
        assert_eq!(point.len(), poly.num_vars, "point dimension mismatch");
        assert!(poly.num_vars <= MAX_VARIABLES, "public tensor size limit");
        assert_eq!(
            poly.evals.len(),
            1 << poly.num_vars,
            "invalid polynomial dimensions"
        );
        let cols = columns(poly);
        let tree = ColumnTree::build(&cols);
        let c = commitment(&tree, poly.num_vars);
        let (rv, _) = matrix::split_vars(poly.num_vars);
        let weights = matrix::eq_table(&point[..rv]);
        let combinations: Vec<Goldilocks> = poly
            .evals
            .chunks_exact(1 << rv)
            .map(|column| {
                weights
                    .iter()
                    .zip(column)
                    .fold(Goldilocks::ZERO, |s, (&w, &x)| s + w * x)
            })
            .collect();
        let value = evaluate_small(&combinations, &point[rv..]);
        absorb_claim(t, &c, point, value);
        Opening::TensorMerkle {
            row_combination: serialize(&combinations),
            columns: cols
                .into_iter()
                .enumerate()
                .map(|(index, column)| ColumnQuery {
                    index,
                    column,
                    path: tree.path(index),
                })
                .collect(),
        }
    }

    fn verify(
        c: &Commitment,
        point: &[Goldilocks],
        value: Goldilocks,
        opening: &Opening,
        t: &mut Transcript,
    ) -> bool {
        let Some(table) = authenticated_table(c, point.len(), opening) else {
            return false;
        };
        let Opening::TensorMerkle {
            row_combination, ..
        } = opening
        else {
            return false;
        };
        let (rv, cv) = matrix::split_vars(point.len());
        let Some(combinations) = decode(row_combination, 1 << cv) else {
            return false;
        };
        let weights = matrix::eq_table(&point[..rv]);
        for (column, &claimed) in table.chunks_exact(1 << rv).zip(&combinations) {
            let actual = weights
                .iter()
                .zip(column)
                .fold(Goldilocks::ZERO, |s, (&w, &x)| s + w * x);
            if actual != claimed {
                return false;
            }
        }
        if evaluate_small(&combinations, &point[rv..]) != value {
            return false;
        }
        absorb_claim(t, c, point, value);
        true
    }

    fn batch_open(
        poly: &MultilinearPoly<Goldilocks>,
        points: &[(Vec<Goldilocks>, Goldilocks)],
        t: &mut Transcript,
    ) -> Opening {
        t.absorb(b"lens-public-tensor-batch-v1");
        t.absorb(&(points.len() as u64).to_le_bytes());
        let c = Self::commit(poly);
        for (point, value) in points {
            absorb_claim(t, &c, point, *value);
        }
        match points.first() {
            Some((point, _)) => Self::open(poly, point, t),
            None => Opening::TensorMerkle {
                row_combination: vec![],
                columns: vec![],
            },
        }
    }

    fn batch_verify(
        c: &Commitment,
        points: &[(Vec<Goldilocks>, Goldilocks)],
        opening: &Opening,
        t: &mut Transcript,
    ) -> bool {
        let Some((first, value)) = points.first() else {
            return false;
        };
        if first.len() > MAX_VARIABLES || points.iter().any(|(p, _)| p.len() != first.len()) {
            return false;
        }
        t.absorb(b"lens-public-tensor-batch-v1");
        t.absorb(&(points.len() as u64).to_le_bytes());
        for (point, value) in points {
            absorb_claim(t, c, point, *value);
        }
        if !Self::verify(c, first, *value, opening, t) {
            return false;
        }
        let Some(table) = authenticated_table(c, first.len(), opening) else {
            return false;
        };
        let poly = MultilinearPoly::new(table);
        points
            .iter()
            .all(|(point, value)| poly.evaluate(point) == *value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyber_hemera::Side;
    fn polynomial(variables: usize) -> MultilinearPoly<Goldilocks> {
        MultilinearPoly::new(
            (0..1usize << variables)
                .map(|i| Goldilocks::new((i * i + 9) as u64))
                .collect(),
        )
    }
    fn point(variables: usize) -> Vec<Goldilocks> {
        (0..variables)
            .map(|i| Goldilocks::new(i as u64 + 3))
            .collect()
    }
    fn transcript() -> Transcript {
        Transcript::new(b"public-tensor-tests")
    }

    #[test]
    fn exact_opening_authenticates_every_element_at_even_and_odd_dimensions() {
        for variables in [0, 1, 2, 6, 7, 9] {
            let poly = polynomial(variables);
            let p = point(variables);
            let c = PublicTensor::commit(&poly);
            let o = PublicTensor::open(&poly, &p, &mut transcript());
            assert!(PublicTensor::verify(
                &c,
                &p,
                poly.evaluate(&p),
                &o,
                &mut transcript()
            ));
            assert_eq!(
                PublicTensor::authenticated_evaluations(&c, variables, &o),
                Some(poly.evals.clone())
            );
            assert!(!PublicTensor::verify(
                &c,
                &p,
                poly.evaluate(&p) + Goldilocks::ONE,
                &o,
                &mut transcript()
            ));
            let Opening::TensorMerkle { columns, .. } = &o else {
                unreachable!()
            };
            assert_eq!(
                columns.iter().map(|q| q.column.len()).sum::<usize>(),
                8 * poly.evals.len()
            );
        }
    }

    #[test]
    fn malformed_columns_paths_and_combinations_are_rejected() {
        let poly = polynomial(6);
        let p = point(6);
        let c = PublicTensor::commit(&poly);
        let value = poly.evaluate(&p);
        let proof = PublicTensor::open(&poly, &p, &mut transcript());
        for attack in 0..9 {
            let mut bad = proof.clone();
            let Opening::TensorMerkle {
                row_combination,
                columns,
            } = &mut bad
            else {
                unreachable!()
            };
            match attack {
                0 => {
                    columns.pop();
                }
                1 => columns.swap(0, 1),
                2 => columns[1] = columns[0].clone(),
                3 => columns[0].column[0] ^= 1,
                4 => row_combination[..8].copy_from_slice(&nebu::field::P.to_le_bytes()),
                5 => columns[0].column.push(0),
                6 => columns[0].path[0].1 = Side::Left,
                7 => {
                    columns[0].path.pop();
                }
                8 => row_combination.push(0),
                _ => unreachable!(),
            }
            assert!(
                !PublicTensor::verify(&c, &p, value, &bad, &mut transcript()),
                "attack {attack}"
            );
        }
        assert!(!PublicTensor::verify(
            &c,
            &point(MAX_VARIABLES + 1),
            value,
            &proof,
            &mut transcript()
        ));
    }

    #[test]
    fn batching_checks_every_supplied_point_and_value() {
        let poly = polynomial(6);
        let c = PublicTensor::commit(&poly);
        let mut points: Vec<_> = (2..5)
            .map(|k| {
                let p = vec![Goldilocks::new(k); 6];
                let v = poly.evaluate(&p);
                (p, v)
            })
            .collect();
        let proof = PublicTensor::batch_open(&poly, &points, &mut transcript());
        assert!(PublicTensor::batch_verify(
            &c,
            &points,
            &proof,
            &mut transcript()
        ));
        points[1].1 += Goldilocks::ONE;
        assert!(!PublicTensor::batch_verify(
            &c,
            &points,
            &proof,
            &mut transcript()
        ));
        points[1].1 -= Goldilocks::ONE;
        points[2].0[0] += Goldilocks::ONE;
        assert!(!PublicTensor::batch_verify(
            &c,
            &points,
            &proof,
            &mut transcript()
        ));
        assert!(!PublicTensor::batch_verify(
            &c,
            &[],
            &proof,
            &mut transcript()
        ));
    }

    #[test]
    fn commitment_is_versioned_and_backend_specific() {
        let poly = polynomial(6);
        assert_ne!(PublicTensor::commit(&poly), crate::Brakedown::commit(&poly));
        let p = point(6);
        let old = crate::Brakedown::open(&poly, &p, &mut transcript());
        assert!(!PublicTensor::verify(
            &PublicTensor::commit(&poly),
            &p,
            poly.evaluate(&p),
            &old,
            &mut transcript()
        ));
    }
}
