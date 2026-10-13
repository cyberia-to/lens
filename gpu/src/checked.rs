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

/// Whether `op` is checked (`LENS_CHECK=op,op,…`; unset: every op).
fn on(op: &str) -> bool {
    match std::env::var("LENS_CHECK") {
        Ok(v) => v.split(',').any(|x| x == op),
        Err(_) => true,
    }
}

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
        if !on("permute") {
            return self.0.permute(states);
        }
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
        if !on("grind") {
            return d;
        }
        assert_eq!(
            d,
            Cpu.grind(template, inject, bits),
            "grind {bits} bits {inject:?}"
        );
        d
    }
    fn sponge(&self, init: &State, rate: usize, blocks: usize, lanes: &[u64]) -> Vec<[u64; 4]> {
        let d = self.0.sponge(init, rate, blocks, lanes);
        if !on("sponge") {
            return d;
        }
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
        if !on("lines") {
            return d;
        }
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
        if !on("session") {
            return Some(dev);
        }
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
        if !on("leaves") {
            return Some(d);
        }
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
        if !on("levels") {
            return Some(d);
        }
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
        if !on("ntt") {
            return d;
        }
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

/// Spot checks: another backend's results verified on a few sampled
/// outputs per call (cheap enough not to change a run's timing much).
pub struct Spot(pub Arc<dyn Backend>);

fn pick(n: usize, k: usize) -> impl Iterator<Item = usize> {
    (0..k).map(move |j| (j * 0x9e37_79b9usize).wrapping_add(j * j * 7919) % n.max(1))
}

impl Backend for Spot {
    fn name(&self) -> &'static str {
        "spot"
    }
    fn permute(&self, states: &mut [State]) {
        let input: Vec<State> = pick(states.len(), 8).map(|i| states[i]).collect();
        self.0.permute(states);
        for (j, i) in pick(states.len(), 8).enumerate() {
            let mut w = [input[j]];
            Cpu.permute(&mut w);
            assert_eq!(
                states[i],
                w[0],
                "spot permute: state {i} of {}",
                states.len()
            );
        }
    }
    fn grind(&self, template: &State, inject: Inject, bits: u32) -> u64 {
        let n = self.0.grind(template, inject, bits);
        let mut s = [inject.apply(template, n)];
        Cpu.permute(&mut s);
        assert_eq!(
            s[0][0] & ((1u64 << bits) - 1),
            0,
            "spot grind: nonce {n} fails"
        );
        n
    }
    fn sponge(&self, init: &State, rate: usize, blocks: usize, lanes: &[u64]) -> Vec<[u64; 4]> {
        let d = self.0.sponge(init, rate, blocks, lanes);
        let per = rate * blocks;
        for i in pick(d.len(), 8) {
            let want = Cpu.sponge(init, rate, blocks, &lanes[i * per..(i + 1) * per]);
            assert_eq!(d[i], want[0], "spot sponge: item {i} of {}", d.len());
        }
        d
    }
    fn line_sums(
        &self,
        prog: &Program,
        cols: &[Limbs],
        weights: &[Limbs],
        xs: &[u64],
    ) -> Vec<Limbs> {
        self.0.line_sums(prog, cols, weights, xs)
    }
    fn line_session<'a>(
        &'a self,
        prog: &'a Program,
        cols: &[Limbs],
        len: usize,
    ) -> Option<Box<dyn LineSession + 'a>> {
        self.0.line_session(prog, cols, len)
    }
    fn coset_leaves(
        &self,
        job: &CosetLeaves<'_>,
        cosets: core::ops::Range<usize>,
    ) -> Option<Vec<[u64; 4]>> {
        let d = self.0.coset_leaves(job, cosets.clone())?;
        let per = job.per();
        let c = (cosets.start * 7 + 3) % cosets.len();
        let want = coset_leaves_reference(job, cosets.start + c..cosets.start + c + 1);
        assert_eq!(
            &d[c * per..(c + 1) * per],
            &want[..],
            "spot coset leaves: coset {} of {cosets:?} (k {})",
            cosets.start + c,
            job.k()
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
        // the first level at sampled nodes, and every level from its predecessor at node 0
        let mut prev: Vec<[u64; 4]> = leaves.to_vec();
        for (lv, (&a, level)) in arities.iter().zip(&d).enumerate() {
            for i in pick(level.len(), 4) {
                let want = merkle_levels_reference(&prev[a * i..a * (i + 1)], &[a], tag);
                assert_eq!(
                    level[i],
                    want[0][0],
                    "spot merkle: level {} node {i} of {}",
                    lv + 1,
                    level.len()
                );
            }
            prev = level.clone();
        }
        Some(d)
    }
    fn coset_ntt(&self, coeffs: &[u64], shifts: &[u64]) -> Vec<Vec<u64>> {
        let d = self.0.coset_ntt(coeffs, shifts);
        // one sampled evaluation: coset s at index j is ĉ(shift_s · ω^j)
        use nebu::Goldilocks as G;
        let n = coeffs.len();
        if n >= 2 && !shifts.is_empty() {
            let s = n % shifts.len();
            let j = (n / 3) % n;
            let w =
                G::new(7).exp((nebu::field::P - 1) / n as u64).exp(j as u64) * G::new(shifts[s]);
            let v = coeffs
                .iter()
                .rev()
                .fold(G::ZERO, |acc, &c| acc * w + G::new(c));
            assert_eq!(
                d[s][j],
                v.as_u64(),
                "spot coset ntt: coset {s} index {j} of {n}"
            );
        }
        d
    }
}

/// Records `op input-hash output-hash` per call to `LENS_RECORD` (a file),
/// to compare two backends' runs of one proof without slowing either.
pub struct Record(pub Arc<dyn Backend>, pub std::sync::Mutex<std::fs::File>);

fn h64(words: impl IntoIterator<Item = u64>) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for w in words {
        h = (h ^ w).wrapping_mul(0x0000_0100_0000_01b3);
        h ^= h >> 29;
    }
    h
}

impl Record {
    fn log(&self, op: &str, i: u64, o: u64) {
        use std::io::Write;
        let mut f = self.1.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(f, "{op} {i:016x} {o:016x}");
    }
}

impl Backend for Record {
    fn name(&self) -> &'static str {
        "record"
    }
    fn permute(&self, states: &mut [State]) {
        let i = h64(states.iter().flatten().copied());
        self.0.permute(states);
        self.log("permute", i, h64(states.iter().flatten().copied()));
    }
    fn grind(&self, template: &State, inject: Inject, bits: u32) -> u64 {
        let i = h64(template.iter().copied().chain([
            bits as u64,
            inject.lo as u64,
            inject.lo_shift as u64,
        ]));
        let n = self.0.grind(template, inject, bits);
        self.log("grind", i, n);
        n
    }
    fn sponge(&self, init: &State, rate: usize, blocks: usize, lanes: &[u64]) -> Vec<[u64; 4]> {
        let i = h64(init
            .iter()
            .copied()
            .chain([rate as u64, blocks as u64])
            .chain(lanes.iter().copied()));
        let d = self.0.sponge(init, rate, blocks, lanes);
        self.log("sponge", i, h64(d.iter().flatten().copied()));
        d
    }
    fn line_sums(
        &self,
        prog: &Program,
        cols: &[Limbs],
        weights: &[Limbs],
        xs: &[u64],
    ) -> Vec<Limbs> {
        let i = h64(cols
            .iter()
            .flatten()
            .copied()
            .chain(weights.iter().flatten().copied()));
        let d = self.0.line_sums(prog, cols, weights, xs);
        self.log("lines", i, h64(d.iter().flatten().copied()));
        d
    }
    fn line_session<'a>(
        &'a self,
        prog: &'a Program,
        cols: &[Limbs],
        len: usize,
    ) -> Option<Box<dyn LineSession + 'a>> {
        self.0.line_session(prog, cols, len)
    }
    fn coset_leaves(
        &self,
        job: &CosetLeaves<'_>,
        cosets: core::ops::Range<usize>,
    ) -> Option<Vec<[u64; 4]>> {
        let i = h64(job.limbs.iter().flat_map(|l| l.iter().copied()).chain([
            job.log_n as u64,
            cosets.start as u64,
            cosets.end as u64,
        ]));
        let d = self.0.coset_leaves(job, cosets.clone())?;
        self.log("leaves", i, h64(d.iter().flatten().copied()));
        Some(d)
    }
    fn merkle_levels(
        &self,
        leaves: &[[u64; 4]],
        arities: &[usize],
        tag: u64,
    ) -> Option<Vec<Vec<[u64; 4]>>> {
        let i = h64(leaves
            .iter()
            .flatten()
            .copied()
            .chain(arities.iter().map(|&a| a as u64)));
        let d = self.0.merkle_levels(leaves, arities, tag)?;
        self.log("levels", i, h64(d.iter().flatten().flatten().copied()));
        Some(d)
    }
    fn coset_ntt(&self, coeffs: &[u64], shifts: &[u64]) -> Vec<Vec<u64>> {
        let i = h64(coeffs.iter().copied().chain(shifts.iter().copied()));
        let d = self.0.coset_ntt(coeffs, shifts);
        self.log("ntt", i, h64(d.iter().flatten().copied()));
        d
    }
}
