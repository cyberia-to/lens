use super::*;
use crate::tests::common::Rng;

const DOMAIN: &[u8] = b"rspcs-zk-test";

fn params(r: u8, k: u8, fin: u8) -> WhirParams {
    WhirParams { log_inv_rate: r, folding_factor: k, pow_bits: 0, security_target: 128, max_final_vars: fin, ..WhirParams::default() }
}

struct Case {
    p: WhirParams,
    m: usize,
    root: Commitment,
    data: WhirData,
    weight: Weight,
}

fn case(rng: &mut Rng, p: WhirParams, m: usize) -> Case {
    let (k0, log_leaves) = round0_shape(&p, m).unwrap();
    let c = rng.evals(m);
    let mask = ZkMask {
        g: (0..1usize << (m - k0)).map(|_| rng.e()).collect(),
        salts: (0..SALT_LIMBS << log_leaves).map(|_| rng.g()).collect(),
        block: None,
    };
    let (root, data) = commit_zk(&p, &c, mask).unwrap();
    let mut weight = Weight::default();
    for _ in 0..3 {
        weight.eq.push((rng.point(m), rng.e()));
    }
    for _ in 0..5 {
        weight.explicit.push(((rng.u64() as usize) & ((1 << m) - 1), rng.e()));
    }
    Case { p, m, root, data, weight }
}

fn prove(c: &Case) -> (ZkValues, WhirProof) {
    match open_zk(&c.p, &c.data, &c.weight, &mut Transcript::new(DOMAIN)).unwrap() {
        ZkOutcome::Opened(v, p) => (v, p),
        ZkOutcome::Resample => unreachable!("no block named"),
    }
}

fn verify(c: &Case, vals: ZkValues, proof: &WhirProof) -> Result<(), PcsError> {
    verify_zk(&c.p, &c.root, c.m, &c.weight, &vals, proof, &mut Transcript::new(DOMAIN))
}

#[test]
fn zk_openings_verify_across_shapes() {
    let mut rng = Rng::new(1);
    for (r, k, fin, m) in [(2, 2, 2, 6), (3, 3, 3, 9), (4, 4, 4, 12), (6, 4, 8, 13), (3, 2, 0, 7)] {
        let c = case(&mut rng, params(r, k, fin), m);
        let (vals, proof) = prove(&c);
        let Message::Base { evals, .. } = &c.data.message else { unreachable!() };
        assert_eq!(vals.v, c.weight.apply_base(evals));
        verify(&c, vals, &proof).unwrap_or_else(|e| panic!("{r} {k} {fin} {m}: {e:?}"));
        let bytes = proof.to_bytes();
        assert_eq!(WhirProof::from_bytes(&bytes).unwrap(), proof, "wire round-trip");
    }
}

#[test]
fn wrong_claims_are_rejected() {
    let mut rng = Rng::new(2);
    let c = case(&mut rng, params(4, 4, 4), 12);
    let (vals, proof) = prove(&c);
    let one = Fp3::ONE;
    assert!(verify(&c, ZkValues { v: vals.v + one, ..vals }, &proof).is_err(), "v");
    assert!(verify(&c, ZkValues { u: vals.u + one, ..vals }, &proof).is_err(), "u");
    assert!(verify(&c, ZkValues { nonce: vals.nonce ^ 1, ..vals }, &proof).is_err(), "nonce");
    let mut other = c.weight.clone();
    other.eq[0].1 += one;
    assert!(verify_zk(&c.p, &c.root, c.m, &other, &vals, &proof, &mut Transcript::new(DOMAIN)).is_err(), "weight");
    let mut root = c.root;
    root.0 = cyber_hemera::hash(b"other");
    assert!(verify_zk(&c.p, &root, c.m, &c.weight, &vals, &proof, &mut Transcript::new(DOMAIN)).is_err(), "root");
}

#[test]
fn every_round0_symbol_is_bound() {
    // c-coset symbols, the three G limbs and the four salt limbs of every
    // opened round-0 leaf: changing any one fails authentication
    let mut rng = Rng::new(3);
    let c = case(&mut rng, params(4, 4, 4), 12);
    let (vals, proof) = prove(&c);
    let first = if proof.rounds.is_empty() { &proof.final_opening } else { &proof.rounds[0].opening };
    let Symbols::Base(sym) = &first.symbols else { panic!("base symbols") };
    let width = first.width;
    assert_eq!(width, 16 + EXTRA);
    for pos in 0..width {
        let mut bad = proof.clone();
        let o = if bad.rounds.is_empty() { &mut bad.final_opening } else { &mut bad.rounds[0].opening };
        let Symbols::Base(s) = &mut o.symbols else { unreachable!() };
        s[pos] += Goldilocks::ONE;
        assert!(verify(&c, vals, &bad).is_err(), "symbol {pos}");
    }
    assert!(sym.len() % width == 0);
}

#[test]
fn every_bit_flip_of_a_small_zk_proof_is_rejected() {
    let mut rng = Rng::new(4);
    let c = case(&mut rng, params(2, 2, 2), 6);
    let (vals, proof) = prove(&c);
    let bytes = proof.to_bytes();
    let mut accepted = 0;
    for i in 0..bytes.len() * 8 {
        let mut b = bytes.clone();
        b[i / 8] ^= 1 << (i % 8);
        if WhirProof::from_bytes(&b).is_ok_and(|p| verify(&c, vals, &p).is_ok()) {
            accepted += 1;
        }
    }
    assert_eq!(accepted, 0);
}

#[test]
fn a_plain_commitment_refuses_a_zk_opening_and_back() {
    let mut rng = Rng::new(5);
    let p = params(4, 4, 4);
    let c = case(&mut rng, p, 12);
    assert!(std::panic::catch_unwind(|| {
        let mut t = Transcript::new(DOMAIN);
        crate::whir::prover::open(&p, &c.data, &rng_point(12), &mut t)
    })
    .is_err());
    let evals = Rng::new(9).evals(12);
    let (_, plain) = super::super::prover::commit(&p, &evals);
    assert!(open_zk(&p, &plain, &c.weight, &mut Transcript::new(DOMAIN)).is_err());
}

fn rng_point(m: usize) -> Vec<Fp3> {
    Rng::new(77).point(m)
}

#[test]
fn zk_bits_at_the_device_shapes() {
    for m in [12, 14, 16, 17] {
        let p = WhirParams { log_inv_rate: 6, folding_factor: 4, pow_bits: 16, ..WhirParams::default() };
        let b = zk_security_bits(&p, m);
        assert!(b >= 128.0, "2^{m}: {b} — {:?}", zk_terms(&p, m));
    }
}

/// `∏_{i ≥ ρ} q_i` as one eq term at `(q_{<ρ}, 1…1)`.
fn block_term(q: &[Fp3], rho: usize, scale: Fp3) -> (Vec<Fp3>, Fp3) {
    let coef = q[rho..].iter().fold(scale, |a, &x| a * x);
    (q[..rho].iter().copied().chain(core::iter::repeat_n(Fp3::ONE, q.len() - rho)).collect(), coef)
}

/// A table with a masking block at the top and a claim shaped as veil-W's:
/// `a0·eq(q, ·)` off the block, plus `Ψ` on the block — uniform weights
/// (`random`) or one geometric point.
fn block_case(seed: u64, random: bool) -> ZkOutcome {
    let mut rng = Rng::new(seed);
    let m = 12;
    let p = WhirParams { log_inv_rate: 6, folding_factor: 4, pow_bits: 0, ..WhirParams::default() };
    let (k0, log_leaves) = round0_shape(&p, m).unwrap();
    let rho = 10;
    let mask = ZkMask {
        g: (0..1usize << (m - k0)).map(|_| rng.e()).collect(),
        salts: (0..SALT_LIMBS << log_leaves).map(|_| rng.g()).collect(),
        block: Some(rho),
    };
    let c = rng.evals(m);
    let (_, data) = commit_zk(&p, &c, mask).unwrap();
    let q = rng.point(m);
    let a0 = rng.e();
    let mut weight = Weight { eq: vec![(q.clone(), a0), block_term(&q, rho, -a0)], explicit: vec![] };
    let start = (1 << m) - (1 << rho);
    if random {
        weight.explicit.extend((0..1 << rho).map(|k| (start + k, rng.e())));
    } else {
        let z = rng.e();
        let pp: Vec<Fp3> = (0..m).scan(z, |s, _| { let v = *s; *s = *s * *s; Some(v) }).collect();
        weight.eq.push(block_term(&pp, rho, rng.e()));
    }
    open_zk(&p, &data, &weight, &mut Transcript::new(DOMAIN)).unwrap()
}

#[test]
fn the_rank_check_passes_random_block_weights_and_catches_a_geometric_one() {
    for seed in 0..3 {
        assert!(matches!(block_case(seed, true), ZkOutcome::Opened(..)), "random ξ, seed {seed}");
        assert!(matches!(block_case(seed, false), ZkOutcome::Resample), "geometric Ψ, seed {seed}");
    }
}
