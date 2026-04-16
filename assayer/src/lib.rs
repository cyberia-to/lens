//! cyb-lens-assayer — Assayer tropical witness-verify commitment.
//!
//! The tropical semiring (min, +) supports min (addition) and saturating
//! addition (multiplication) with identities +inf and 0. It has no
//! subtraction or inversion.
//!
//! Assayer is a wrapper protocol: it accepts tropical computation results,
//! packs the witness (optimal assignment + cost) and LP dual certificate
//! as Goldilocks field elements, and commits via Brakedown.
//!
//! Assayer does not implement the Lens trait. It delegates to Brakedown.
//!
//! See specs/tropical-semiring.md for the full specification.

pub use cyb_lens_core::{Commitment, Opening, Transcript};

use cyb_lens_brakedown::Brakedown;
use cyb_lens_core::{Lens, MultilinearPoly};
use nebu::Goldilocks;
use trop::Tropical;

/// A directed edge in the problem graph.
#[derive(Clone, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub weight: Tropical,
}

/// A tropical witness: the result of an optimization computation.
#[derive(Clone, Debug)]
pub struct TropicalWitness {
    /// The number of vertices in the problem graph.
    pub num_vertices: usize,
    /// All edges in the problem graph.
    pub edges: Vec<Edge>,
    /// The optimal assignment (indices into `edges`).
    pub assignment: Vec<usize>,
    /// The optimal cost (tropical sum of assigned weights).
    pub cost: Tropical,
    /// Source vertex (for path problems).
    pub source: usize,
    /// Target vertex (for path problems).
    pub target: usize,
}

/// LP dual certificate: distance labels proving optimality.
///
/// For shortest path: d[v] for each vertex v, satisfying:
/// - d[source] = 0
/// - d[target] = cost
/// - d[v] ≤ d[u] + w(u,v) for all edges (u,v)   (dual feasibility)
#[derive(Clone, Debug)]
pub struct DualCertificate {
    /// Distance labels (one per vertex).
    pub dual_vars: Vec<Goldilocks>,
    /// Dual objective value (should equal primal cost for strong duality).
    pub dual_objective: Goldilocks,
}

/// Assayer tropical witness-verify commitment.
pub struct Assayer;

impl Assayer {
    /// Pack a tropical witness and dual certificate into a Goldilocks polynomial
    /// suitable for Brakedown commitment.
    pub fn pack_witness(
        witness: &TropicalWitness,
        certificate: &DualCertificate,
    ) -> MultilinearPoly<Goldilocks> {
        let mut data = Vec::new();

        // Pack cost
        data.push(Goldilocks::new(witness.cost.as_u64()));

        // Pack assignment length + edge indices
        data.push(Goldilocks::new(witness.assignment.len() as u64));
        for &idx in &witness.assignment {
            data.push(Goldilocks::new(idx as u64));
        }

        // Pack edge weights
        for e in &witness.edges {
            data.push(Goldilocks::new(e.weight.as_u64()));
        }

        // Pack dual certificate
        for &d in &certificate.dual_vars {
            data.push(d);
        }
        data.push(certificate.dual_objective);

        // Pad to power of 2
        let n = data.len().next_power_of_two();
        data.resize(n, Goldilocks::ZERO);

        MultilinearPoly::new(data)
    }

    /// Commit to a tropical witness via Brakedown delegation.
    pub fn commit_witness(
        witness: &TropicalWitness,
        certificate: &DualCertificate,
    ) -> (Commitment, MultilinearPoly<Goldilocks>) {
        let poly = Self::pack_witness(witness, certificate);
        let commitment = Brakedown::commit(&poly);
        (commitment, poly)
    }

    /// Open the witness commitment at a point.
    pub fn open_witness(
        poly: &MultilinearPoly<Goldilocks>,
        point: &[Goldilocks],
        transcript: &mut Transcript,
    ) -> Opening {
        Brakedown::open(poly, point, transcript)
    }

    /// Verify a witness opening.
    pub fn verify_witness(
        commitment: &Commitment,
        point: &[Goldilocks],
        value: Goldilocks,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        Brakedown::verify(commitment, point, value, proof, transcript)
    }

    /// Verify the three tropical witness properties:
    ///
    /// 1. **structural validity**: assignment indices are in bounds,
    ///    and assigned edges form a valid path from source to target.
    /// 2. **cost correctness**: claimed cost equals the tropical product
    ///    (ordinary sum) of assigned edge weights.
    /// 3. **dual feasibility**: distance labels satisfy:
    ///    - d[source] = 0
    ///    - d[target] = cost
    ///    - ∀ (u,v) ∈ edges: d[v] ≤ d[u] + w(u,v)
    pub fn verify_tropical(witness: &TropicalWitness, certificate: &DualCertificate) -> bool {
        // ── Check 1: structural validity ─────────────────────────
        // All assignment indices must be valid edge indices.
        for &idx in &witness.assignment {
            if idx >= witness.edges.len() {
                return false;
            }
        }

        // Assigned edges must form a path from source to target.
        if !witness.assignment.is_empty() {
            let first_edge = &witness.edges[witness.assignment[0]];
            if first_edge.from != witness.source {
                return false;
            }

            // Each edge's `to` must equal the next edge's `from`.
            for w in witness.assignment.windows(2) {
                let e1 = &witness.edges[w[0]];
                let e2 = &witness.edges[w[1]];
                if e1.to != e2.from {
                    return false;
                }
            }

            let last_edge = &witness.edges[*witness.assignment.last().unwrap()];
            if last_edge.to != witness.target {
                return false;
            }
        }

        // ── Check 2: cost correctness ────────────────────────────
        // Tropical product (ordinary sum) of assigned edge weights.
        let mut computed_cost = Tropical::ONE; // multiplicative identity = 0
        for &idx in &witness.assignment {
            computed_cost = computed_cost.mul(witness.edges[idx].weight);
        }
        if computed_cost != witness.cost {
            return false;
        }

        // ── Check 3: dual feasibility ────────────────────────────
        // Must have one dual variable per vertex.
        if certificate.dual_vars.len() != witness.num_vertices {
            return false;
        }

        // Strong duality: dual objective = primal cost.
        let primal_as_goldilocks = Goldilocks::new(witness.cost.as_u64());
        if certificate.dual_objective != primal_as_goldilocks {
            return false;
        }

        // d[source] = 0.
        if certificate.dual_vars[witness.source] != Goldilocks::ZERO {
            return false;
        }

        // d[target] = cost.
        if certificate.dual_vars[witness.target] != primal_as_goldilocks {
            return false;
        }

        // Dual feasibility: ∀ (u,v,w) ∈ edges: d[v] ≤ d[u] + w.
        // In Goldilocks: d[v].as_u64() ≤ d[u].as_u64() + w.as_u64().
        for edge in &witness.edges {
            if edge.weight.is_inf() {
                continue; // infinite weight edge: constraint is trivially satisfied
            }
            let du = certificate.dual_vars[edge.from].as_u64();
            let dv = certificate.dual_vars[edge.to].as_u64();
            let w = edge.weight.as_u64();
            if dv > du.saturating_add(w) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shortest_path_graph() -> (TropicalWitness, DualCertificate) {
        // Graph: 0 →3→ 1 →2→ 2 →4→ 3
        // Shortest path 0→3: 0→1→2→3, cost = 3+2+4 = 9
        let edges = vec![
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
            }, // longer path
            Edge {
                from: 0,
                to: 3,
                weight: Tropical::from_u64(20),
            }, // direct but long
        ];

        let witness = TropicalWitness {
            num_vertices: 4,
            edges,
            assignment: vec![0, 1, 2], // edges 0→1→2→3
            cost: Tropical::from_u64(9),
            source: 0,
            target: 3,
        };

        // Distance labels: d = [0, 3, 5, 9]
        let cert = DualCertificate {
            dual_vars: vec![
                Goldilocks::new(0),
                Goldilocks::new(3),
                Goldilocks::new(5),
                Goldilocks::new(9),
            ],
            dual_objective: Goldilocks::new(9),
        };

        (witness, cert)
    }

    #[test]
    fn valid_shortest_path() {
        let (w, c) = shortest_path_graph();
        assert!(Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn commit_and_verify_roundtrip() {
        let (w, c) = shortest_path_graph();
        let (commitment, poly) = Assayer::commit_witness(&w, &c);

        let point = vec![Goldilocks::ZERO; poly.num_vars];
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"assayer");
        let proof = Assayer::open_witness(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"assayer");
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
        let (mut w, mut c) = shortest_path_graph();
        w.cost = Tropical::from_u64(999);
        c.dual_objective = Goldilocks::new(999);
        assert!(!Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn broken_path_rejected() {
        let (mut w, c) = shortest_path_graph();
        // Assignment [0, 2] means edges 0→1 then 2→3 — but 1 ≠ 2, broken path
        w.assignment = vec![0, 2];
        w.cost = Tropical::from_u64(7); // 3 + 4
        assert!(!Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn wrong_source_rejected() {
        let (mut w, c) = shortest_path_graph();
        w.assignment = vec![1, 2]; // starts at vertex 1, not 0
        w.cost = Tropical::from_u64(6);
        assert!(!Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn out_of_bounds_rejected() {
        let (mut w, c) = shortest_path_graph();
        w.assignment = vec![0, 999];
        assert!(!Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn dual_feasibility_violation_rejected() {
        let (w, mut c) = shortest_path_graph();
        // Set d[2] = 100, violating d[2] ≤ d[1] + w(1→2) = 3 + 2 = 5
        c.dual_vars[2] = Goldilocks::new(100);
        assert!(!Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn wrong_source_label_rejected() {
        let (w, mut c) = shortest_path_graph();
        c.dual_vars[0] = Goldilocks::new(1); // d[source] should be 0
        assert!(!Assayer::verify_tropical(&w, &c));
    }

    #[test]
    fn wrong_target_label_rejected() {
        let (w, mut c) = shortest_path_graph();
        c.dual_vars[3] = Goldilocks::new(5); // d[target] should be 9
        assert!(!Assayer::verify_tropical(&w, &c));
    }
}
