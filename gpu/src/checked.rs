//! A backend that runs another and the CPU reference side by side and
//! panics, naming the operation, on the first difference — for finding a
//! device kernel that disagrees inside a whole proof (`select` it with
//! `check-metal` / `check-webgpu`).

use std::sync::Arc;

use cyber_lens_rspcs::backend::{
    Backend, CosetLeaves, Cpu, CpuSession, Inject, LineSession, State, coset_leaves_reference,
    merkle_levels_reference,
};
use cyber_lens_rspcs::program::{Limbs, Program};

pub struct Checked(pub Arc<dyn Backend>);

struct CheckedSession<'a> {
    dev: Box<dyn LineSession + 'a>,
    cpu: CpuSession<'a>,
    round: usize,
}

impl LineSession for CheckedSession<'_> {
    fn sums(&mut self, weights: &[Limbs], xs: &[u64]) -> Vec<Limbs> {
        let d = self.dev.sums(weights, xs);
        assert_eq!(
            d,
            self.cpu.sums(weights, xs),
            "line session sums, round {}",
            self.round
        );
        d
    }
    fn fold(&mut self, alpha: Limbs) {
        self.dev.fold(alpha);
        self.cpu.fold(alpha);
        self.round += 1;
    }
    fn firsts(&mut self) -> Vec<Limbs> {
        let d = self.dev.firsts();
        assert_eq!(d, self.cpu.firsts(), "line session firsts");
        d
    }
}

impl Backend for Checked {
    fn name(&self) -> &'static str {
        "checked"
    }
    fn permute(&self, states: &mut [State]) {
        let mut want = states.to_vec();
        Cpu.permute(&mut want);
        self.0.permute(states);
        assert!(
            states == want.as_slice(),
            "permute ({} states)",
            states.len()
        );
    }
    fn grind(&self, template: &State, inject: Inject, bits: u32) -> u64 {
        let d = self.0.grind(template, inject, bits);
        assert_eq!(
            d,
            Cpu.grind(template, inject, bits),
            "grind {bits} bits {inject:?}"
        );
        d
    }
    fn sponge(&self, init: &State, rate: usize, blocks: usize, lanes: &[u64]) -> Vec<[u64; 4]> {
        let d = self.0.sponge(init, rate, blocks, lanes);
        assert_eq!(
            d,
            Cpu.sponge(init, rate, blocks, lanes),
            "sponge rate {rate} blocks {blocks} items {}",
            d.len()
        );
        d
    }
    fn line_sums(
        &self,
        prog: &Program,
        cols: &[Limbs],
        weights: &[Limbs],
        xs: &[u64],
    ) -> Vec<Limbs> {
        let d = self.0.line_sums(prog, cols, weights, xs);
        assert_eq!(d, Cpu.line_sums(prog, cols, weights, xs), "line sums");
        d
    }
    fn line_session<'a>(
        &'a self,
        prog: &'a Program,
        cols: &[Limbs],
        len: usize,
    ) -> Option<Box<dyn LineSession + 'a>> {
        let dev = self.0.line_session(prog, cols, len)?;
        Some(Box::new(CheckedSession {
            dev,
            cpu: CpuSession::new(prog, cols, len),
            round: 0,
        }))
    }
    fn coset_leaves(
        &self,
        job: &CosetLeaves<'_>,
        cosets: core::ops::Range<usize>,
    ) -> Option<Vec<[u64; 4]>> {
        let d = self.0.coset_leaves(job, cosets.clone())?;
        assert_eq!(
            d,
            coset_leaves_reference(job, cosets.clone()),
            "coset leaves k {} log_n {} cosets {cosets:?}",
            job.k(),
            job.log_n
        );
        Some(d)
    }
    fn merkle_levels(
        &self,
        leaves: &[[u64; 4]],
        arities: &[usize],
        tag: u64,
    ) -> Option<Vec<Vec<[u64; 4]>>> {
        let d = self.0.merkle_levels(leaves, arities, tag)?;
        assert_eq!(
            d,
            merkle_levels_reference(leaves, arities, tag),
            "merkle levels {} leaves {arities:?}",
            leaves.len()
        );
        Some(d)
    }
    fn coset_ntt(&self, coeffs: &[u64], shifts: &[u64]) -> Vec<Vec<u64>> {
        let d = self.0.coset_ntt(coeffs, shifts);
        assert_eq!(
            d,
            Cpu.coset_ntt(coeffs, shifts),
            "coset ntt {} × {}",
            shifts.len(),
            coeffs.len()
        );
        d
    }
}
