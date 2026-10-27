//! Shared test harness for both schemes.

use cyber_lens_core::{Commitment, MultilinearPcs, PcsError, Transcript};
use nebu::field::P;
use nebu::{Fp3, Goldilocks};

use crate::field::ml_eval_base;
use crate::{TensorRsProof, WhirProof};

pub const DOMAIN: &[u8] = b"rspcs-test";

/// splitmix64.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn g(&mut self) -> Goldilocks {
        Goldilocks::new(self.u64() % P)
    }
    pub fn e(&mut self) -> Fp3 {
        Fp3::new(self.g(), self.g(), self.g())
    }
    pub fn evals(&mut self, m: usize) -> Vec<Goldilocks> {
        (0..1usize << m).map(|_| self.g()).collect()
    }
    pub fn point(&mut self, m: usize) -> Vec<Fp3> {
        (0..m).map(|_| self.e()).collect()
    }
}

/// Canonical wire form of a proof.
pub trait Wire: Sized {
    fn bytes(&self) -> Vec<u8>;
    fn parse(b: &[u8]) -> Result<Self, PcsError>;
}

impl Wire for TensorRsProof {
    fn bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
    fn parse(b: &[u8]) -> Result<Self, PcsError> {
        Self::from_bytes(b)
    }
}

impl Wire for WhirProof {
    fn bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
    fn parse(b: &[u8]) -> Result<Self, PcsError> {
        Self::from_bytes(b)
    }
}

pub struct Run<S: MultilinearPcs> {
    pub commitment: Commitment,
    pub data: S::ProverData,
    pub point: Vec<Fp3>,
    pub value: Fp3,
    pub proof: S::Proof,
    pub evals: Vec<Goldilocks>,
}

pub fn run<S: MultilinearPcs>(params: &S::Params, m: usize, seed: u64) -> Run<S> {
    let mut rng = Rng::new(seed);
    let evals = rng.evals(m);
    let point = rng.point(m);
    let (commitment, data) = S::commit(params, &evals);
    let mut t = Transcript::new(DOMAIN);
    let (value, proof) = S::open(params, &data, &point, &mut t);
    Run {
        commitment,
        data,
        point,
        value,
        proof,
        evals,
    }
}

pub fn check<S: MultilinearPcs>(
    params: &S::Params,
    c: &Commitment,
    m: usize,
    point: &[Fp3],
    value: Fp3,
    proof: &S::Proof,
) -> Result<(), PcsError> {
    S::verify(
        params,
        c,
        m,
        point,
        value,
        proof,
        &mut Transcript::new(DOMAIN),
    )
}

/// Honest proof verifies, value is the true evaluation, and prover and
/// verifier transcripts end in the same state.
pub fn completeness<S: MultilinearPcs>(params: &S::Params, m: usize, seed: u64) {
    let r = run::<S>(params, m, seed);
    assert_eq!(r.value, ml_eval_base(&r.evals, &r.point), "value m={m}");
    let mut pt = Transcript::new(DOMAIN);
    let _ = S::open(params, &r.data, &r.point, &mut pt);
    let mut vt = Transcript::new(DOMAIN);
    S::verify(
        params,
        &r.commitment,
        m,
        &r.point,
        r.value,
        &r.proof,
        &mut vt,
    )
    .unwrap_or_else(|e| panic!("{} m={m} {params:?}: {e}", S::NAME));
    assert_eq!(pt.squeeze(), vt.squeeze(), "transcripts diverge m={m}");
}

/// Wrong value, wrong point, wrong commitment and wrong size all fail.
pub fn rejects_wrong_claims<S: MultilinearPcs>(params: &S::Params, m: usize, seed: u64) {
    let r = run::<S>(params, m, seed);
    let ok =
        |c: &Commitment, mm: usize, p: &[Fp3], v: Fp3| check::<S>(params, c, mm, p, v, &r.proof);
    assert!(ok(&r.commitment, m, &r.point, r.value).is_ok());
    assert!(
        ok(&r.commitment, m, &r.point, r.value + Fp3::ONE).is_err(),
        "wrong value"
    );
    for k in 0..m {
        let mut p = r.point.clone();
        p[k] += Fp3::ONE;
        assert!(
            ok(&r.commitment, m, &p, r.value).is_err(),
            "wrong point coord {k}"
        );
    }
    let other = run::<S>(params, m, seed ^ 0xDEAD);
    assert!(
        ok(&other.commitment, m, &r.point, r.value).is_err(),
        "wrong commitment"
    );
    if m > 1 {
        assert!(
            ok(&r.commitment, m - 1, &r.point[1..], r.value).is_err(),
            "wrong size"
        );
    }
    let mut t = Transcript::new(b"another-domain");
    assert!(
        S::verify(
            params,
            &r.commitment,
            m,
            &r.point,
            r.value,
            &r.proof,
            &mut t
        )
        .is_err()
    );
}

/// Flip every bit of the serialized proof; each flip must fail to parse
/// or be rejected. Returns the number of flips tried.
pub fn bitflip_scan<S: MultilinearPcs>(params: &S::Params, m: usize, seed: u64) -> usize
where
    S::Proof: Wire + Sync,
    S::Params: Sync,
{
    let r = run::<S>(params, m, seed);
    let bytes = r.proof.bytes();
    assert_eq!(S::Proof::parse(&bytes).expect("roundtrip").bytes(), bytes);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = bytes.len().div_ceil(threads);
    let (c, point, value) = (r.commitment, &r.point, r.value);
    let accepted: Vec<(usize, u8)> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..bytes.len())
            .step_by(chunk)
            .map(|start| {
                let bytes = &bytes;
                s.spawn(move || {
                    let mut bad = Vec::new();
                    for i in start..(start + chunk).min(bytes.len()) {
                        for bit in 0..8 {
                            let mut b = bytes.clone();
                            b[i] ^= 1 << bit;
                            if let Ok(p) = S::Proof::parse(&b)
                                && check::<S>(params, &c, m, point, value, &p).is_ok()
                            {
                                bad.push((i, bit));
                            }
                        }
                    }
                    bad
                })
            })
            .collect();
        hs.into_iter()
            .flat_map(|h| h.join().expect("scan worker"))
            .collect()
    });
    assert!(
        accepted.is_empty(),
        "{}: accepted flips {accepted:?}",
        S::NAME
    );
    eprintln!(
        "{} m={m}: {} proof bytes, {} flips rejected",
        S::NAME,
        bytes.len(),
        bytes.len() * 8
    );
    bytes.len() * 8
}

/// Same input → identical proof bytes.
pub fn deterministic<S: MultilinearPcs>(params: &S::Params, m: usize, seed: u64)
where
    S::Proof: Wire,
{
    let a = run::<S>(params, m, seed).proof.bytes();
    let b = run::<S>(params, m, seed).proof.bytes();
    assert_eq!(a, b);
}
