//! Prover backends: the bulk kernels a prover spends its time in, behind
//! one trait, so the same proof comes out of a CPU, a Metal GPU or a
//! WebGPU adapter.
//!
//! Every operation is a pure function of its inputs over Goldilocks
//! (canonical `u64` limbs in, canonical limbs out). A backend changes how
//! a result is computed, never which result: grinding returns the
//! *smallest* nonce that passes, a permutation is hemera's permutation,
//! an NTT is the evaluation on the subgroup. Proofs are therefore
//! byte-identical across backends; `tests` in the GPU crate check every
//! operation against [`Cpu`] on random inputs.
//!
//! The process holds one current backend ([`current`], [`install`]);
//! [`Cpu`] until something else is installed. Library code calls
//! `current()` at the seam and never chooses: the binary (joy) picks.
//!
//! Operations:
//! - [`Backend::permute`] — hemera's permutation of many independent
//!   states (Merkle leaves and nodes, sponge blocks);
//! - [`Backend::grind`] — the least nonce whose injected permutation has
//!   `bits` low zero bits in output lane 0 (proof-of-work grinding, both
//!   the byte transcript of lens and the field-native one of zheng);
//! - [`Backend::coset_ntt`] — Reed–Solomon encoding: the NTT of
//!   `c_i · s^i` for each coset shift `s`.

use std::sync::{Arc, OnceLock, RwLock};

use cyber_hemera::field::Goldilocks as HG;
use nebu::{Fp3, Goldilocks};

/// A hemera state: sixteen canonical Goldilocks limbs.
pub type State = [u64; 16];

/// How a grinding nonce `n` enters the permutation input: the template's
/// lane `lo` gains `(n mod 2^lo_bits) · 2^lo_shift`, lane `hi` gains
/// `⌊n / 2^hi_shift⌋` (field additions; `hi_shift = 64` disables it). The
/// added values stay below `2^56 < p`, so the addition is the integer one
/// the byte encoding performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inject {
    pub lo: usize,
    pub lo_shift: u32,
    pub lo_bits: u32,
    pub hi: usize,
    pub hi_shift: u32,
}

impl Inject {
    /// The nonce is the lane's value (a field-native duplex absorbing it).
    pub const fn lane(lane: usize) -> Self {
        Self {
            lo: lane,
            lo_shift: 0,
            lo_bits: 64,
            hi: lane,
            hi_shift: 64,
        }
    }

    /// `hemera::hash(seed ‖ nonce_le)` for a 32-byte seed: the 40-byte
    /// message is one rate block of 7-byte limbs; nonce bytes 0..3 sit in
    /// limb 4 at byte offsets 4..7, bytes 3..8 in limb 5 at offsets 0..5.
    pub const fn hash40() -> Self {
        Self {
            lo: 4,
            lo_shift: 32,
            lo_bits: 24,
            hi: 5,
            hi_shift: 24,
        }
    }

    /// The permutation input for nonce `n`.
    pub fn apply(&self, template: &State, n: u64) -> State {
        let mut s = *template;
        let lo = if self.lo_bits >= 64 {
            n
        } else {
            n & ((1u64 << self.lo_bits) - 1)
        };
        s[self.lo] = add(s[self.lo], lo << self.lo_shift);
        if self.hi_shift < 64 {
            s[self.hi] = add(s[self.hi], n >> self.hi_shift);
        }
        s
    }
}

fn add(a: u64, b: u64) -> u64 {
    (Goldilocks::new(a) + Goldilocks::new(b)).as_u64()
}

/// The template of [`Inject::hash40`]: hemera's plain-hash state for the
/// message `seed ‖ 0⁸` before its single permutation.
pub fn hash40_template(seed: &[u8; 32]) -> State {
    // hemera's plain sponge: 7-byte limbs over the rate, capacity starting
    // after the rate, lane CAPACITY+2 the byte length, lane CAPACITY+3 the
    // domain (0 for `hash`); `hash40_template_is_hemera_hash` pins it.
    use cyber_hemera::{RATE, RATE_BYTES};
    const CAPACITY_START: usize = RATE;
    const INPUT_BYTES_PER_ELEMENT: usize = RATE_BYTES / RATE;
    let mut padded = [0u8; RATE_BYTES];
    padded[..32].copy_from_slice(seed);
    padded[40] = 0x01;
    let mut s = [0u64; 16];
    for (i, chunk) in padded.chunks(INPUT_BYTES_PER_ELEMENT).enumerate() {
        let mut b = [0u8; 8];
        b[..chunk.len()].copy_from_slice(chunk);
        s[i] = u64::from_le_bytes(b);
    }
    s[CAPACITY_START + 2] = 40;
    s
}

/// Leaves of a streamed Reed–Solomon commitment: the words' coefficient
/// limbs (member-major, then limb: one vector for a base word, three for
/// an Fp3 word; `k` coefficients each) over `2^log_n` points, the domain
/// split into `n/k` cosets `ω^s·H` (lens `rs`: coset `s` is the NTT of
/// `c_i·ω^{s·i}`). Leaf `a` of coset `s` hashes, member by member, the
/// symbols `u = a + t·per` (`t < 2^log_width`, `per` leaves a coset) of
/// that coset as an overwrite-mode sponge from `init`: symbols packed
/// `rate / limbs` to a block (one lane a limb), unused lanes zero.
#[derive(Clone, Debug)]
pub struct CosetLeaves<'a> {
    pub limbs: Vec<&'a [u64]>,
    pub limbs_per_member: usize,
    pub log_n: u32,
    pub log_width: u32,
    pub init: State,
    pub rate: usize,
}

impl CosetLeaves<'_> {
    pub fn k(&self) -> usize {
        self.limbs[0].len()
    }
    pub fn members(&self) -> usize {
        self.limbs.len() / self.limbs_per_member
    }
    pub fn cosets(&self) -> usize {
        (1usize << self.log_n) / self.k()
    }
    pub fn per(&self) -> usize {
        ((1usize << self.log_n) >> self.log_width) / self.cosets()
    }
    /// Symbols a block and blocks a leaf.
    pub fn blocks(&self) -> (usize, usize) {
        let spb = self.rate / self.limbs_per_member;
        let total = self.members() << self.log_width;
        (spb, total.div_ceil(spb))
    }
    /// The coset shifts `ω^s`.
    pub fn shift(&self, s: usize) -> u64 {
        crate::field::root_of_unity(self.log_n)
            .exp(s as u64)
            .as_u64()
    }
}

/// A program's line sums over tables that stay with the backend across
/// rounds: `sums` as [`Backend::line_sums`] over the current tables,
/// `fold` replaces every column `v` by `v'_i = v_{2i} + α·(v_{2i+1} − v_{2i})`,
/// `firsts` returns every column's entry 0.
pub trait LineSession {
    fn sums(&mut self, weights: &[crate::program::Limbs], xs: &[u64])
    -> Vec<crate::program::Limbs>;
    fn fold(&mut self, alpha: crate::program::Limbs);
    fn firsts(&mut self) -> Vec<crate::program::Limbs>;
    /// Every column at its current length, column-major (a caller taking
    /// the small last rounds back to the CPU).
    fn columns(&mut self) -> Vec<crate::program::Limbs>;
}

/// The CPU reference of a [`LineSession`] (tables in memory).
#[derive(Debug)]
pub struct CpuSession<'a> {
    prog: &'a crate::program::Program,
    cols: Vec<Vec<crate::program::Limbs>>,
}

impl<'a> CpuSession<'a> {
    pub fn new(
        prog: &'a crate::program::Program,
        cols: &[crate::program::Limbs],
        len: usize,
    ) -> Self {
        Self {
            prog,
            cols: cols.chunks_exact(len).map(<[_]>::to_vec).collect(),
        }
    }
}

impl LineSession for CpuSession<'_> {
    fn sums(
        &mut self,
        weights: &[crate::program::Limbs],
        xs: &[u64],
    ) -> Vec<crate::program::Limbs> {
        let flat: Vec<crate::program::Limbs> = self.cols.iter().flatten().copied().collect();
        Cpu.line_sums(self.prog, &flat, weights, xs)
    }
    fn fold(&mut self, alpha: crate::program::Limbs) {
        use crate::program::{from_fp3, to_fp3};
        let a = to_fp3(alpha);
        for c in &mut self.cols {
            let half = c.len() / 2;
            for i in 0..half {
                let (x, y) = (to_fp3(c[2 * i]), to_fp3(c[2 * i + 1]));
                c[i] = from_fp3(x + a * (y - x));
            }
            c.truncate(half);
        }
    }
    fn firsts(&mut self) -> Vec<crate::program::Limbs> {
        self.cols.iter().map(|c| c[0]).collect()
    }
    fn columns(&mut self) -> Vec<crate::program::Limbs> {
        self.cols.iter().flatten().copied().collect()
    }
}

/// The bulk kernels of a prover. Implementations must return exactly what
/// [`Cpu`] returns.
/// A hemera byte tree (lens [`crate::merkle::Tree`]) over leaves cut
/// from a codeword: leaf `j` is the little-endian bytes of symbols
/// `j + t·leaves` (`t < width`), each symbol `limbs` u64 limbs taken from
/// `symbols` (symbol-major; any representative of the residue, reduced by
/// the backend). Leaves hash as `hemera::tree::hash_leaf(bytes, j, false)`,
/// nodes as `hash_node` with the root flag at the top.
#[derive(Clone, Copy, Debug)]
pub struct ByteTree<'a> {
    pub symbols: &'a [u64],
    pub limbs: usize,
    pub width: usize,
    pub leaves: usize,
}

pub trait Backend: Send + Sync {
    /// A short name for reports (`cpu`, `metal`, `webgpu`).
    fn name(&self) -> &'static str;

    /// Hemera's permutation of every state, in place.
    fn permute(&self, states: &mut [State]) {
        Cpu.permute(states)
    }

    /// The least `n ≥ 0` with `permute(inject.apply(template, n))[0]`
    /// having its low `bits` bits zero (`1 ≤ bits < 64`).
    fn grind(&self, template: &State, inject: Inject, bits: u32) -> u64 {
        Cpu.grind(template, inject, bits)
    }

    /// An overwrite-mode sponge per item: from `init`, for each of
    /// `blocks` blocks the rate lanes `0..rate` are replaced by the
    /// item's next `rate` values of `lanes` and the state permuted; the
    /// digest is lanes `0..4`. `lanes.len() = items · blocks · rate`.
    fn sponge(&self, init: &State, rate: usize, blocks: usize, lanes: &[u64]) -> Vec<[u64; 4]> {
        Cpu.sponge(init, rate, blocks, lanes)
    }

    /// The weighted sums of a program along lines: for each `x` in `xs`,
    /// `Σ_i weights[i] · prog(v_i(x))` where input `c` of `v_i(x)` is
    /// `a + x·(b − a)`, `a = cols[c][2i]`, `b = cols[c][2i + 1]`
    /// (`cols` column-major: `cols[c·len + r]`, `len = 2·weights.len()`).
    /// Pairs of weight zero contribute nothing.
    fn line_sums(
        &self,
        prog: &crate::program::Program,
        cols: &[crate::program::Limbs],
        weights: &[crate::program::Limbs],
        xs: &[u64],
    ) -> Vec<crate::program::Limbs> {
        Cpu.line_sums(prog, cols, weights, xs)
    }

    /// A [`LineSession`] over `cols` (column-major, `len` rows each) on
    /// the backend's device, or `None` (the caller runs rounds itself).
    fn line_session<'a>(
        &'a self,
        _prog: &'a crate::program::Program,
        _cols: &[crate::program::Limbs],
        _len: usize,
    ) -> Option<Box<dyn LineSession + 'a>> {
        None
    }

    /// The levels of a Merkle tree above `leaves` (level 1 first, the root
    /// last): level `j + 1` has one node per `arities[j]` nodes of level
    /// `j`; a binary node permutes `(l ‖ r ‖ tag ‖ 0⁷)`, a 4-ary node the
    /// four children's sixteen lanes; a node's digest is lanes `0..4`.
    /// `None` where the backend does not run them.
    fn merkle_levels(
        &self,
        _leaves: &[[u64; 4]],
        _arities: &[usize],
        _tag: u64,
    ) -> Option<Vec<Vec<[u64; 4]>>> {
        None
    }

    /// The leaf digests of cosets `cosets` of `job` (coset-major, then
    /// leaf), or `None` where the backend does not run them (the caller
    /// streams on the CPU then).
    fn coset_leaves(
        &self,
        _job: &CosetLeaves<'_>,
        _cosets: core::ops::Range<usize>,
    ) -> Option<Vec<[u64; 4]>> {
        None
    }

    /// [`Backend::coset_leaves`], also writing the coset values to
    /// `values` (limb-major, then coset, then `η^u` order: entry
    /// `(j·g + c)·k + u` is limb vector `j`'s coset `cosets.start + c` at
    /// `u`; `values.len() = limbs · g · k`) — a held codeword committed in
    /// one pass. `None` where the backend does not run it (`values`
    /// untouched).
    fn coset_leaves_kept(
        &self,
        _job: &CosetLeaves<'_>,
        _cosets: core::ops::Range<usize>,
        _values: &mut [u64],
    ) -> Option<Vec<[u64; 4]>> {
        None
    }

    /// Every level of the byte tree `job` (leaf digests first, the root
    /// last), or `None` where the backend does not build it (the caller
    /// hashes on the CPU or through [`Backend::permute`] then).
    fn byte_tree(&self, _job: &ByteTree<'_>) -> Option<Vec<Vec<[u64; 4]>>> {
        None
    }

    /// For each shift `s` in `shifts`: the NTT (nebu's order) of
    /// `coeffs[i] · s^i`; output `j` is `cosets[j]` (`coeffs.len()` a power
    /// of two).
    fn coset_ntt(&self, coeffs: &[u64], shifts: &[u64]) -> Vec<Vec<u64>> {
        Cpu.coset_ntt(coeffs, shifts)
    }
}

/// The CPU backend: hemera's lane-interleaved kernel and nebu's NTT across
/// the machine's threads.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cpu;

/// States per CPU work unit (a multiple of hemera's 16-lane batch).
const CPU_CHUNK: usize = 256;
/// Nonces per CPU worker and round of grinding.
const GRIND_BATCH: u64 = 1 << 12;

fn permute_slice(states: &mut [State]) {
    // SAFETY: hemera's Goldilocks is `repr(transparent)` over `u64` and
    // every canonical limb is a valid representative.
    let h: &mut [[HG; 16]] =
        unsafe { core::slice::from_raw_parts_mut(states.as_mut_ptr().cast(), states.len()) };
    cyber_hemera::permutation::permute_batch(h);
    for s in states.iter_mut() {
        for x in s.iter_mut() {
            *x = HG::new(*x).as_canonical_u64();
        }
    }
}

impl Backend for Cpu {
    fn name(&self) -> &'static str {
        "cpu"
    }

    fn permute(&self, states: &mut [State]) {
        let t = crate::par::threads();
        if t == 1 || states.len() < 2 * CPU_CHUNK {
            return permute_slice(states);
        }
        let chunk = states.len().div_ceil(t).next_multiple_of(16);
        std::thread::scope(|s| {
            for c in states.chunks_mut(chunk) {
                s.spawn(move || permute_slice(c));
            }
        });
    }

    fn grind(&self, template: &State, inject: Inject, bits: u32) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
        assert!((1..64).contains(&bits), "grinding bits in 1..64");
        let mask = (1u64 << bits) - 1;
        // Workers claim consecutive batches from a shared counter and stop
        // once a claimed batch starts at or beyond the best nonce found:
        // every batch below the final best was claimed and searched in
        // full, so the result is the least passing nonce.
        let next = AtomicU64::new(0);
        let best = AtomicU64::new(u64::MAX);
        let work = || {
            let mut st: Vec<State> = Vec::with_capacity(GRIND_BATCH as usize);
            loop {
                let from = next.fetch_add(GRIND_BATCH, Relaxed);
                if from >= best.load(Relaxed) {
                    return;
                }
                st.clear();
                st.extend((from..from + GRIND_BATCH).map(|n| inject.apply(template, n)));
                permute_slice(&mut st);
                if let Some(i) = st.iter().position(|y| y[0] & mask == 0) {
                    best.fetch_min(from + i as u64, Relaxed);
                }
            }
        };
        let workers = crate::par::threads();
        std::thread::scope(|s| {
            for _ in 1..workers {
                s.spawn(work);
            }
            work();
        });
        best.into_inner()
    }

    fn sponge(&self, init: &State, rate: usize, blocks: usize, lanes: &[u64]) -> Vec<[u64; 4]> {
        assert!(
            (1..=16).contains(&rate) && blocks >= 1 && lanes.len().is_multiple_of(rate * blocks)
        );
        let items = lanes.len() / (rate * blocks);
        let per_item = rate * blocks;
        let run = |first: usize, out: &mut [[u64; 4]]| {
            let mut st: Vec<State> = vec![*init; out.len()];
            for b in 0..blocks {
                for (k, s) in st.iter_mut().enumerate() {
                    let at = (first + k) * per_item + b * rate;
                    s[..rate].copy_from_slice(&lanes[at..at + rate]);
                }
                permute_slice(&mut st);
            }
            for (o, s) in out.iter_mut().zip(&st) {
                o.copy_from_slice(&s[..4]);
            }
        };
        let mut out = vec![[0u64; 4]; items];
        let t = crate::par::threads();
        if t == 1 || items < 2 * CPU_CHUNK {
            run(0, &mut out);
            return out;
        }
        let chunk = items.div_ceil(t).next_multiple_of(16);
        std::thread::scope(|s| {
            for (i, c) in out.chunks_mut(chunk).enumerate() {
                let run = &run;
                s.spawn(move || run(i * chunk, c));
            }
        });
        out
    }

    fn line_sums(
        &self,
        prog: &crate::program::Program,
        cols: &[crate::program::Limbs],
        weights: &[crate::program::Limbs],
        xs: &[u64],
    ) -> Vec<crate::program::Limbs> {
        use crate::program::{from_fp3, to_fp3};
        let pairs = weights.len();
        let len = 2 * pairs;
        let ncols = prog.inputs as usize;
        assert_eq!(cols.len(), ncols * len, "cols");
        let part = |lo: usize, hi: usize| {
            let mut sums = vec![Fp3::ZERO; xs.len()];
            let mut ins = vec![Fp3::ZERO; ncols];
            let mut regs = Vec::new();
            for i in lo..hi {
                let g = to_fp3(weights[i]);
                if g == Fp3::ZERO {
                    continue;
                }
                for (k, &x) in xs.iter().enumerate() {
                    let xf = Goldilocks::new(x);
                    for (c, v) in ins.iter_mut().enumerate() {
                        let a = to_fp3(cols[c * len + 2 * i]);
                        let b = to_fp3(cols[c * len + 2 * i + 1]);
                        let d = b - a;
                        *v = a + Fp3::new(d.c0 * xf, d.c1 * xf, d.c2 * xf);
                    }
                    sums[k] += g * prog.eval(&ins, &mut regs);
                }
            }
            sums
        };
        let t = crate::par::threads();
        let chunk = pairs.div_ceil(t).max(1);
        let parts: Vec<Vec<Fp3>> = std::thread::scope(|s| {
            let hs: Vec<_> = (0..pairs)
                .step_by(chunk)
                .map(|lo| s.spawn(move || part(lo, (lo + chunk).min(pairs))))
                .collect();
            hs.into_iter()
                .map(|h| h.join().expect("line sums"))
                .collect()
        });
        let mut out = vec![Fp3::ZERO; xs.len()];
        for p in parts {
            for (o, v) in out.iter_mut().zip(p) {
                *o += v;
            }
        }
        out.into_iter().map(from_fp3).collect()
    }

    fn coset_ntt(&self, coeffs: &[u64], shifts: &[u64]) -> Vec<Vec<u64>> {
        let one = |s: u64| {
            let shift = Goldilocks::new(s);
            let mut v = Vec::with_capacity(coeffs.len());
            let mut w = Goldilocks::ONE;
            for &c in coeffs {
                v.push(Goldilocks::new(c) * w);
                w *= shift;
            }
            nebu::ntt::ntt(&mut v);
            v.into_iter().map(|x| x.as_u64()).collect::<Vec<u64>>()
        };
        if shifts.len() > 1 && shifts.len() * coeffs.len() >= 1 << 14 {
            crate::par::par_map_min(shifts.len(), 2, |j| one(shifts[j]))
        } else {
            shifts.iter().map(|&s| one(s)).collect()
        }
    }
}

/// The definition of [`Backend::merkle_levels`] on the CPU.
pub fn merkle_levels_reference(
    leaves: &[[u64; 4]],
    arities: &[usize],
    tag: u64,
) -> Vec<Vec<[u64; 4]>> {
    let mut out: Vec<Vec<[u64; 4]>> = Vec::new();
    let mut cur = leaves.to_vec();
    for &a in arities {
        let mut st: Vec<State> = cur
            .chunks_exact(a)
            .map(|ch| {
                let mut s = [0u64; 16];
                for (i, d) in ch.iter().enumerate() {
                    s[4 * i..4 * i + 4].copy_from_slice(d);
                }
                if a == 2 {
                    s[8] = tag;
                }
                s
            })
            .collect();
        Cpu.permute(&mut st);
        cur = st.iter().map(|x| [x[0], x[1], x[2], x[3]]).collect();
        out.push(cur.clone());
    }
    out
}

/// The definition of [`Backend::coset_leaves`] on the CPU (the reference
/// the device backends are tested against).
pub fn coset_leaves_reference(
    job: &CosetLeaves<'_>,
    cosets: core::ops::Range<usize>,
) -> Vec<[u64; 4]> {
    let (k, w, per, l) = (
        job.k(),
        1usize << job.log_width,
        job.per(),
        job.limbs_per_member,
    );
    let (spb, blocks) = job.blocks();
    let mut out = Vec::new();
    for s in cosets {
        let vals = Cpu.coset_ntt_one(&job.limbs, job.shift(s));
        let mut st: Vec<State> = vec![job.init; per];
        for b in 0..blocks {
            for (a, x) in st.iter_mut().enumerate() {
                let mut lane = 0;
                for j in 0..spb {
                    let t = b * spb + j;
                    if t >= job.members() * w {
                        break;
                    }
                    let (m, tt) = (t / w, t % w);
                    for q in 0..l {
                        x[lane + q] = vals[m * l + q][a + tt * per];
                    }
                    lane += l;
                }
                for y in x.iter_mut().take(job.rate).skip(lane) {
                    *y = 0;
                }
            }
            permute_slice(&mut st);
        }
        out.extend(st.iter().map(|x| [x[0], x[1], x[2], x[3]]));
        let _ = k;
    }
    out
}

/// The coset values [`Backend::coset_leaves_kept`] writes, on the CPU.
pub fn coset_values_reference(job: &CosetLeaves<'_>, cosets: core::ops::Range<usize>) -> Vec<u64> {
    let (k, g) = (job.k(), cosets.len());
    let mut out = vec![0u64; job.limbs.len() * g * k];
    for (c, s) in cosets.enumerate() {
        for (j, v) in Cpu
            .coset_ntt_one(&job.limbs, job.shift(s))
            .into_iter()
            .enumerate()
        {
            out[(j * g + c) * k..(j * g + c + 1) * k].copy_from_slice(&v);
        }
    }
    out
}

impl Cpu {
    /// One coset of every limb vector.
    fn coset_ntt_one(&self, limbs: &[&[u64]], shift: u64) -> Vec<Vec<u64>> {
        limbs
            .iter()
            .map(|c| self.coset_ntt(c, &[shift]).pop().expect("one coset"))
            .collect()
    }
}

fn slot() -> &'static RwLock<Arc<dyn Backend>> {
    static B: OnceLock<RwLock<Arc<dyn Backend>>> = OnceLock::new();
    B.get_or_init(|| RwLock::new(Arc::new(Cpu)))
}

/// The process's backend ([`Cpu`] unless one was installed).
pub fn current() -> Arc<dyn Backend> {
    slot().read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Make `b` the process's backend; returns the previous one.
pub fn install(b: Arc<dyn Backend>) -> Arc<dyn Backend> {
    let mut g = slot().write().unwrap_or_else(|e| e.into_inner());
    core::mem::replace(&mut *g, b)
}

/// Canonical limbs of nebu elements.
pub fn limbs(xs: &[Goldilocks]) -> Vec<u64> {
    xs.iter().map(|x| x.as_u64()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rand(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    #[test]
    fn hash40_template_is_hemera_hash() {
        let mut r = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..64 {
            let mut seed = [0u8; 32];
            for b in seed.iter_mut() {
                *b = rand(&mut r) as u8;
            }
            let n = rand(&mut r) >> (rand(&mut r) % 64);
            let mut msg = [0u8; 40];
            msg[..32].copy_from_slice(&seed);
            msg[32..].copy_from_slice(&n.to_le_bytes());
            let want = cyber_hemera::hash(&msg);
            let mut s = [Inject::hash40().apply(&hash40_template(&seed), n)];
            Cpu.permute(&mut s);
            let got: Vec<u8> = s[0][..4].iter().flat_map(|x| x.to_le_bytes()).collect();
            assert_eq!(got.as_slice(), want.as_bytes(), "nonce {n}");
        }
    }

    #[test]
    fn permute_matches_single_permutations() {
        let mut r = 7u64;
        let mut states: Vec<State> = (0..1000)
            .map(|_| core::array::from_fn(|_| rand(&mut r) % nebu::field::P))
            .collect();
        let want: Vec<State> = states
            .iter()
            .map(|s| {
                let mut h: [HG; 16] = core::array::from_fn(|i| HG::new(s[i]));
                cyber_hemera::permutation::permute(&mut h);
                core::array::from_fn(|i| h[i].as_canonical_u64())
            })
            .collect();
        Cpu.permute(&mut states);
        assert_eq!(states, want);
    }

    #[test]
    fn sponge_absorbs_blocks_in_order() {
        let mut r = 3u64;
        let init: State = core::array::from_fn(|i| if i == 9 { 2 } else { 0 });
        let (rate, blocks, items) = (9, 3, 700);
        let lanes: Vec<u64> = (0..items * rate * blocks)
            .map(|_| rand(&mut r) % nebu::field::P)
            .collect();
        let got = Cpu.sponge(&init, rate, blocks, &lanes);
        for (i, g) in got.iter().enumerate() {
            let mut s = init;
            for b in 0..blocks {
                let at = (i * blocks + b) * rate;
                s[..rate].copy_from_slice(&lanes[at..at + rate]);
                let mut one = [s];
                permute_slice(&mut one);
                s = one[0];
            }
            assert_eq!(g[..], s[..4]);
        }
    }

    #[test]
    fn grind_returns_the_least_nonce() {
        let t: State = core::array::from_fn(|i| i as u64 * 977);
        for bits in [1, 4, 9] {
            let n = Cpu.grind(&t, Inject::lane(3), bits);
            let mask = (1u64 << bits) - 1;
            for m in 0..=n {
                let mut s = [Inject::lane(3).apply(&t, m)];
                Cpu.permute(&mut s);
                assert_eq!(s[0][0] & mask == 0, m == n, "bits {bits} nonce {m}");
            }
        }
    }
}
