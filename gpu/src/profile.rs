//! A backend that runs another and adds up the wall time of every
//! operation per kind (`prof-metal`, `prof-webgpu`, `prof-cpu`): where a
//! prover's wall time goes on the device side, host copies included.
//! [`report`] prints the totals and resets them.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use cyber_lens_rspcs::backend::{Backend, CosetLeaves, Inject, LineSession, State};
use cyber_lens_rspcs::program::{Limbs, Program};

pub struct Profile(pub Arc<dyn Backend>);

/// Per operation: calls, wall seconds, items (states, symbols, nonces).
type Totals = Vec<(&'static str, u64, f64, u64)>;

static TOTALS: Mutex<Totals> = Mutex::new(Vec::new());

fn add(op: &'static str, t: Instant, items: u64) {
    let dt = t.elapsed().as_secs_f64();
    if std::env::var_os("LENS_PROFILE_EACH").is_some() {
        eprintln!("    [{op} {:.1} ms, {items} items]", dt * 1e3);
    }
    let mut g = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
    match g.iter_mut().find(|e| e.0 == op) {
        Some(e) => {
            e.1 += 1;
            e.2 += dt;
            e.3 += items;
        }
        None => g.push((op, 1, dt, items)),
    }
}

/// The totals since the last report, one line per operation, and reset.
pub fn report() -> String {
    let mut g = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
    let mut s = String::new();
    for (op, n, dt, items) in g.iter() {
        s.push_str(&format!("  {op:14} {n:6} calls {:9.1} ms  {items} items\n", dt * 1e3));
    }
    g.clear();
    s
}

struct ProfSession<'a>(Box<dyn LineSession + 'a>);

impl LineSession for ProfSession<'_> {
    fn sums(&mut self, weights: &[Limbs], xs: &[u64]) -> Vec<Limbs> {
        let t = Instant::now();
        let r = self.0.sums(weights, xs);
        add("session.sums", t, (weights.len() * xs.len()) as u64);
        r
    }
    fn fold(&mut self, alpha: Limbs) {
        let t = Instant::now();
        self.0.fold(alpha);
        add("session.fold", t, 0);
    }
    fn firsts(&mut self) -> Vec<Limbs> {
        self.0.firsts()
    }
    fn columns(&mut self) -> Vec<Limbs> {
        let t = Instant::now();
        let r = self.0.columns();
        add("session.columns", t, r.len() as u64);
        r
    }
}

impl Backend for Profile {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn permute(&self, states: &mut [State]) {
        let t = Instant::now();
        self.0.permute(states);
        add("permute", t, states.len() as u64);
    }
    fn grind(&self, template: &State, inject: Inject, bits: u32) -> u64 {
        let t = Instant::now();
        let n = self.0.grind(template, inject, bits);
        add("grind", t, n);
        n
    }
    fn sponge(&self, init: &State, rate: usize, blocks: usize, lanes: &[u64]) -> Vec<[u64; 4]> {
        let t = Instant::now();
        let r = self.0.sponge(init, rate, blocks, lanes);
        add("sponge", t, (lanes.len() / rate) as u64);
        r
    }
    fn line_sums(&self, prog: &Program, cols: &[Limbs], weights: &[Limbs], xs: &[u64]) -> Vec<Limbs> {
        let t = Instant::now();
        let r = self.0.line_sums(prog, cols, weights, xs);
        add("line_sums", t, (weights.len() * xs.len()) as u64);
        r
    }
    fn line_session<'a>(&'a self, prog: &'a Program, cols: &[Limbs], len: usize) -> Option<Box<dyn LineSession + 'a>> {
        let t = Instant::now();
        let s = self.0.line_session(prog, cols, len);
        add("line_session", t, cols.len() as u64);
        s.map(|s| Box::new(ProfSession(s)) as Box<dyn LineSession + 'a>)
    }
    fn merkle_levels(&self, leaves: &[[u64; 4]], arities: &[usize], tag: u64) -> Option<Vec<Vec<[u64; 4]>>> {
        let t = Instant::now();
        let r = self.0.merkle_levels(leaves, arities, tag);
        add("merkle_levels", t, leaves.len() as u64);
        r
    }
    fn coset_leaves(&self, job: &CosetLeaves<'_>, cosets: core::ops::Range<usize>) -> Option<Vec<[u64; 4]>> {
        let t = Instant::now();
        let n = (cosets.len() * job.k() * job.limbs.len()) as u64;
        let r = self.0.coset_leaves(job, cosets);
        add("coset_leaves", t, n);
        r
    }
    fn coset_leaves_kept(&self, job: &CosetLeaves<'_>, cosets: core::ops::Range<usize>, values: &mut [u64]) -> Option<Vec<[u64; 4]>> {
        let t = Instant::now();
        let n = (cosets.len() * job.k() * job.limbs.len()) as u64;
        let r = self.0.coset_leaves_kept(job, cosets, values);
        add("leaves_kept", t, n);
        r
    }
    fn coset_ntt(&self, coeffs: &[u64], shifts: &[u64]) -> Vec<Vec<u64>> {
        let t = Instant::now();
        let r = self.0.coset_ntt(coeffs, shifts);
        add("coset_ntt", t, (coeffs.len() * shifts.len()) as u64);
        r
    }
}
