//! The Metal backend: hemera kernels in MSL on the Apple GPU through
//! honeycrisp's aruminium. Buffers are shared-storage (unified memory):
//! the CPU writes states where the GPU reads them, nothing crosses a bus.
//!
//! Batches below [`MIN_BATCH`] states stay on the CPU (a dispatch costs
//! ~0.2 ms); the NTT runs on the CPU backend (no Metal NTT yet).

use std::sync::Mutex;

use aruminium::{Buffer, Gpu, Pipeline, Queue};
use cyber_lens_rspcs::backend::{Backend, CosetLeaves, Cpu, Inject, LineSession, State};
use cyber_lens_rspcs::program::{Limbs, Program, from_fp3, to_fp3};
use nebu::Fp3;

/// Fewer states than this run on the CPU backend.
pub const MIN_BATCH: usize = 1 << 12;
/// States per GPU thread by default (measured: `examples/gpu_bench`).
pub const PER_THREAD: usize = 1;
/// The largest grinding dispatch, in nonces (the first tries 2^bits).
const GRIND_DISPATCH: u64 = 1 << 22;

struct Inner {
    gpu: Gpu,
    queue: Queue,
    permute: Pipeline,
    grind: Pipeline,
    scatter: Pipeline,
    stage: Pipeline,
    block: Pipeline,
    sponge: Pipeline,
    lines: Pipeline,
    leaves: Pipeline,
    fold: Pipeline,
    level: Pipeline,
    /// coset-leaves buffers (coefficients, values, digests), grown on demand.
    leaf_bufs: Option<(Buffer, Buffer, Buffer)>,
    /// Sponge lanes and digests buffers, grown on demand.
    sponge_bufs: Option<(Buffer, Buffer)>,
    /// Twiddles `ω_{2h}^i` at `h + i` for the last coset length used.
    twiddles: Option<(u32, Buffer)>,
    /// The states buffer, grown on demand.
    states: Option<Buffer>,
    tpl: Buffer,
    found: Buffer,
    params: Buffer,
}

/// Hemera kernels on the Metal GPU.
pub struct Metal {
    inner: Mutex<Inner>,
    k: usize,
    name: String,
}

// SAFETY: every Metal object lives in `inner` behind the mutex; one thread
// at a time encodes, submits and waits. Metal devices, queues, pipelines
// and shared buffers may be used from any thread.
unsafe impl Send for Metal {}
// SAFETY: as above — all access goes through the mutex.
unsafe impl Sync for Metal {}

impl core::fmt::Debug for Metal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Metal").field("device", &self.name).finish()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LevelParams {
    count: u32,
    arity: u32,
    tag: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FoldParams {
    alpha: [u64; 3],
    ncols: u32,
    stride: u32,
    half: u32,
    pad: u32,
}

/// A line session on the GPU: the columns stay in two shared buffers
/// (ping-pong across folds), the program in two more.
pub struct MetalSession<'a> {
    m: &'a Metal,
    prog: &'a Program,
    ops: Buffer,
    consts: Buffer,
    cols: [Buffer; 2],
    cur: usize,
    ncols: usize,
    stride: usize,
    len: usize,
}

impl LineSession for MetalSession<'_> {
    fn sums(&mut self, weights: &[Limbs], xs: &[u64]) -> Vec<Limbs> {
        let pairs = self.len / 2;
        assert_eq!(weights.len(), pairs, "weights");
        let g = self.m.lock();
        let wb = g.gpu.buffer((pairs * 24).max(16)).expect("metal: weights");
        write(&wb, weights);
        let nx = xs.len();
        assert!(nx <= 16 && nx > 0);
        let per = (LINE_DISPATCH / nx).max(1);
        let outb = g
            .gpu
            .buffer(per.min(pairs) * nx * 24)
            .expect("metal: line sums");
        let mut xa = [0u64; 16];
        xa[..nx].copy_from_slice(xs);
        let mut sums = vec![Fp3::ZERO; nx];
        let mut part = vec![[0u64; 3]; per.min(pairs) * nx];
        for first in (0..pairs).step_by(per) {
            let n = per.min(pairs - first);
            let threads = n * nx;
            let p = LineParams {
                nops: self.prog.ops.len() as u32,
                ncols: self.ncols as u32,
                len: self.stride as u32,
                nx: nx as u32,
                threads: threads as u32,
                first: first as u32,
                output: self.prog.output,
                pad: 0,
                xs: xa,
            };
            run(
                &g,
                &g.lines,
                &[
                    (&self.ops, 0),
                    (&self.consts, 1),
                    (&self.cols[self.cur], 2),
                    (&wb, 3),
                    (&outb, 4),
                ],
                Some((bytes_of(&p), 5)),
                threads,
            )
            .expect("metal line sums");
            read(&outb, &mut part[..threads]);
            for (t, v) in part[..threads].iter().enumerate() {
                sums[t % nx] += to_fp3(*v);
            }
        }
        sums.into_iter().map(from_fp3).collect()
    }
    fn fold(&mut self, alpha: Limbs) {
        let half = self.len / 2;
        let g = self.m.lock();
        let p = FoldParams {
            alpha,
            ncols: self.ncols as u32,
            stride: self.stride as u32,
            half: half as u32,
            pad: 0,
        };
        let (src, dst) = (&self.cols[self.cur], &self.cols[1 - self.cur]);
        run(
            &g,
            &g.fold,
            &[(src, 0), (dst, 1)],
            Some((bytes_of(&p), 2)),
            self.ncols * half,
        )
        .expect("metal fold");
        self.cur = 1 - self.cur;
        self.len = half;
    }
    fn columns(&mut self) -> Vec<Limbs> {
        let _g = self.m.lock();
        let base = self.cols[self.cur].contents_ptr() as *const [u64; 3];
        let mut out = Vec::with_capacity(self.ncols * self.len);
        for c in 0..self.ncols {
            // SAFETY: column `c` holds `stride ≥ len` limb triples from
            // `c·stride`; no GPU work in flight (every submit is waited for)
            out.extend_from_slice(unsafe { core::slice::from_raw_parts(base.add(c * self.stride), self.len) });
        }
        out
    }
    fn firsts(&mut self) -> Vec<Limbs> {
        let _g = self.m.lock();
        let base = self.cols[self.cur].contents_ptr() as *const u64;
        (0..self.ncols)
            .map(|c| {
                // SAFETY: the buffer holds `ncols·stride` limbs triples; no GPU work in flight
                unsafe { core::array::from_fn(|q| *base.add(3 * c * self.stride + q)) }
            })
            .collect()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CosetLeafParams {
    init: [u64; 16],
    k: u32,
    per: u32,
    g: u32,
    w: u32,
    members: u32,
    limbs: u32,
    spb: u32,
    blocks: u32,
    rate: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LineParams {
    nops: u32,
    ncols: u32,
    len: u32,
    nx: u32,
    threads: u32,
    first: u32,
    output: u32,
    pad: u32,
    xs: [u64; 16],
}

/// Registers a program may use on the device (`MAXREGS` in the kernel).
pub const MAX_REGS: u32 = 160;
/// Threads per line-sum dispatch.
const LINE_DISPATCH: usize = 1 << 16;

#[repr(C)]
#[derive(Clone, Copy)]
struct SpongeParams {
    init: [u64; 16],
    rate: u32,
    blocks: u32,
    items: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NttParams {
    log_n: u32,
    span: u32,
    cosets: u32,
    pad: u32,
}

/// Coset NTTs below this length run on the CPU backend.
pub const MIN_NTT: usize = 1 << 12;

#[repr(C)]
#[derive(Clone, Copy)]
struct GrindParams {
    base: u64,
    mask: u64,
    lo_mask: u64,
    lo: u32,
    lo_shift: u32,
    hi: u32,
    hi_shift: u32,
    count: u32,
    pad: u32,
}

fn bytes_of<T: Copy>(v: &T) -> &[u8] {
    // SAFETY: `T` is a plain `repr(C)` value without padding-sensitive
    // invariants; the bytes are only copied into a GPU buffer.
    unsafe { core::slice::from_raw_parts((v as *const T).cast(), core::mem::size_of::<T>()) }
}

fn write<T: Copy>(buf: &Buffer, v: &[T]) {
    let n = core::mem::size_of_val(v);
    assert!(n <= buf.size());
    // SAFETY: shared buffer of at least `n` bytes, no GPU work in flight
    // (every submit is waited for under the mutex).
    unsafe { core::ptr::copy_nonoverlapping(v.as_ptr().cast::<u8>(), buf.contents_ptr(), n) }
}

fn read<T: Copy>(buf: &Buffer, out: &mut [T]) {
    let n = core::mem::size_of_val(out);
    assert!(n <= buf.size());
    // SAFETY: as in `write`; the GPU finished (waited) before the read.
    unsafe { core::ptr::copy_nonoverlapping(buf.contents_ptr(), out.as_mut_ptr().cast::<u8>(), n) }
}

impl Metal {
    /// Open the default Metal device and compile the kernels
    /// ([`PER_THREAD`] states per GPU thread).
    pub fn new() -> Result<Self, String> {
        Self::with_states_per_thread(PER_THREAD)
    }

    /// As [`Self::new`] with `k` states per GPU thread (`k ≥ 1`; their
    /// partial-round inversions are shared).
    pub fn with_states_per_thread(k: usize) -> Result<Self, String> {
        assert!(k >= 1);
        let gpu = Gpu::open().map_err(|e| format!("metal: {e:?}"))?;
        let queue = gpu
            .new_command_queue()
            .map_err(|e| format!("metal queue: {e:?}"))?;
        let lib = gpu
            .compile(&crate::msl::source(k))
            .map_err(|e| format!("metal compile: {e:?}"))?;
        let pipe = |name: &str| -> Result<Pipeline, String> {
            let f = lib
                .function(name)
                .map_err(|e| format!("metal {name}: {e:?}"))?;
            gpu.pipeline(&f)
                .map_err(|e| format!("metal pipeline {name}: {e:?}"))
        };
        let permute = pipe("hemera_permute")?;
        let grind = pipe("hemera_grind")?;
        let scatter = pipe("ntt_scatter")?;
        let stage = pipe("ntt_stage")?;
        let block = pipe("ntt_block")?;
        let sponge = pipe("hemera_sponge")?;
        let lines = pipe("prog_lines")?;
        let leaves = pipe("coset_leaves")?;
        let fold = pipe("fold_cols")?;
        let level = pipe("merkle_level")?;
        let tpl = gpu.buffer(16 * 8).map_err(|e| format!("{e:?}"))?;
        let found = gpu.buffer(4).map_err(|e| format!("{e:?}"))?;
        let params = gpu
            .buffer(core::mem::size_of::<GrindParams>())
            .map_err(|e| format!("{e:?}"))?;
        let name = gpu.name();
        Ok(Self {
            inner: Mutex::new(Inner {
                gpu,
                queue,
                permute,
                grind,
                scatter,
                stage,
                block,
                sponge,
                lines,
                leaves,
                fold,
                level,
                leaf_bufs: None,
                sponge_bufs: None,
                twiddles: None,
                states: None,
                tpl,
                found,
                params,
            }),
            name,
            k,
        })
    }

    /// The device's name.
    pub fn device(&self) -> &str {
        &self.name
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn run(
    inner: &Inner,
    pipe: &Pipeline,
    bufs: &[(&Buffer, usize)],
    push: Option<(&[u8], usize)>,
    threads: usize,
) -> Result<(), String> {
    aruminium::autorelease_pool(|| {
        let cmd = inner.queue.commands().map_err(|e| format!("{e:?}"))?;
        let enc = cmd.encoder().map_err(|e| format!("{e:?}"))?;
        enc.bind(pipe);
        for &(b, i) in bufs {
            enc.bind_buffer(b, 0, i);
        }
        if let Some((d, i)) = push {
            enc.push(d, i);
        }
        let w = pipe.thread_execution_width().max(1);
        let group = (pipe.max_total_threads_per_threadgroup() / w).max(1) * w;
        enc.launch((threads, 1, 1), (group.min(256), 1, 1));
        enc.finish();
        cmd.submit();
        cmd.wait();
        match cmd.error() {
            Some(e) => Err(format!("metal command: {e}")),
            None => Ok(()),
        }
    })
}

impl Backend for Metal {
    fn name(&self) -> &'static str {
        "metal"
    }

    fn permute(&self, states: &mut [State]) {
        if states.len() < MIN_BATCH {
            return Cpu.permute(states);
        }
        let mut g = self.lock();
        let bytes = core::mem::size_of_val(states);
        if g.states.as_ref().is_none_or(|b| b.size() < bytes) {
            g.states = None;
            g.states = Some(
                g.gpu
                    .buffer(bytes.next_power_of_two())
                    .expect("metal: states buffer"),
            );
        }
        let inner = &*g;
        let buf = inner.states.as_ref().expect("states buffer");
        write(buf, states);
        let n = states.len() as u32;
        run(
            inner,
            &inner.permute,
            &[(buf, 0)],
            Some((bytes_of(&n), 1)),
            states.len().div_ceil(self.k),
        )
        .expect("metal permute");
        read(buf, states);
    }

    fn sponge(&self, init: &State, rate: usize, blocks: usize, lanes: &[u64]) -> Vec<[u64; 4]> {
        let per = rate * blocks;
        assert!((1..=16).contains(&rate) && blocks >= 1 && lanes.len() % per == 0);
        let items = lanes.len() / per;
        if items < MIN_BATCH {
            return Cpu.sponge(init, rate, blocks, lanes);
        }
        let mut g = self.lock();
        let (need_in, need_out) = (lanes.len() * 8, items * 32);
        if g.sponge_bufs
            .as_ref()
            .is_none_or(|(i, o)| i.size() < need_in || o.size() < need_out)
        {
            g.sponge_bufs = None;
            let i = g
                .gpu
                .buffer(need_in.next_power_of_two())
                .expect("metal: sponge lanes");
            let o = g
                .gpu
                .buffer(need_out.next_power_of_two())
                .expect("metal: sponge digests");
            g.sponge_bufs = Some((i, o));
        }
        let inner = &*g;
        let (inb, outb) = inner.sponge_bufs.as_ref().expect("sponge buffers");
        write(inb, lanes);
        let p = SpongeParams {
            init: *init,
            rate: rate as u32,
            blocks: blocks as u32,
            items: items as u32,
            pad: 0,
        };
        run(
            inner,
            &inner.sponge,
            &[(inb, 0), (outb, 1)],
            Some((bytes_of(&p), 2)),
            items,
        )
        .expect("metal sponge");
        let mut out = vec![[0u64; 4]; items];
        read(outb, &mut out);
        out
    }

    fn line_sums(
        &self,
        prog: &Program,
        cols: &[Limbs],
        weights: &[Limbs],
        xs: &[u64],
    ) -> Vec<Limbs> {
        let pairs = weights.len();
        if prog.regs > MAX_REGS || xs.len() > 16 || xs.is_empty() || pairs * xs.len() < MIN_BATCH {
            return Cpu.line_sums(prog, cols, weights, xs);
        }
        let len = 2 * pairs;
        assert_eq!(cols.len(), prog.inputs as usize * len, "cols");
        let g = self.lock();
        let buf = |data: &[u8]| -> Buffer {
            let b = g
                .gpu
                .buffer(data.len().max(16))
                .expect("metal: line buffer");
            write(&b, data);
            b
        };
        let ops: Vec<u32> = prog
            .ops
            .iter()
            .flat_map(|o| [o.code as u32, o.a, o.b, o.dst])
            .collect();
        let as_bytes = |v: &[u64]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
        let opsb = buf(&ops
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<u8>>());
        let constsb = buf(&as_bytes(prog.consts.as_flattened()));
        let colsb = g
            .gpu
            .buffer((cols.len() * 24).max(16))
            .expect("metal: columns");
        write(&colsb, cols);
        let wb = buf(&as_bytes(weights.as_flattened()));
        let nx = xs.len();
        let per = (LINE_DISPATCH / nx).max(1);
        let outb = g.gpu.buffer(per * nx * 24).expect("metal: line sums");
        let mut sums = vec![Fp3::ZERO; nx];
        let mut xa = [0u64; 16];
        xa[..nx].copy_from_slice(xs);
        let mut part = vec![[0u64; 3]; per * nx];
        for first in (0..pairs).step_by(per) {
            let n = per.min(pairs - first);
            let threads = n * nx;
            let p = LineParams {
                nops: prog.ops.len() as u32,
                ncols: prog.inputs,
                len: len as u32,
                nx: nx as u32,
                threads: threads as u32,
                first: first as u32,
                output: prog.output,
                pad: 0,
                xs: xa,
            };
            run(
                &g,
                &g.lines,
                &[(&opsb, 0), (&constsb, 1), (&colsb, 2), (&wb, 3), (&outb, 4)],
                Some((bytes_of(&p), 5)),
                threads,
            )
            .expect("metal line sums");
            read(&outb, &mut part[..threads]);
            for (t, v) in part[..threads].iter().enumerate() {
                sums[t % nx] += to_fp3(*v);
            }
        }
        sums.into_iter().map(from_fp3).collect()
    }

    fn line_session<'a>(
        &'a self,
        prog: &'a Program,
        cols: &[Limbs],
        len: usize,
    ) -> Option<Box<dyn LineSession + 'a>> {
        if prog.regs > MAX_REGS || len < 2 || cols.len() % len != 0 {
            return None;
        }
        let ncols = cols.len() / len;
        let g = self.lock();
        let ops: Vec<u8> = prog
            .ops
            .iter()
            .flat_map(|o| [o.code as u32, o.a, o.b, o.dst])
            .flat_map(|x| x.to_le_bytes())
            .collect();
        let opsb = g.gpu.buffer(ops.len().max(16)).ok()?;
        write(&opsb, &ops);
        let consts: Vec<u64> = prog.consts.as_flattened().to_vec();
        let constsb = g.gpu.buffer((consts.len() * 8).max(16)).ok()?;
        write(&constsb, &consts);
        let a = g.gpu.buffer(cols.len() * 24).ok()?;
        write(&a, cols);
        let b = g.gpu.buffer(cols.len() * 24).ok()?;
        drop(g);
        Some(Box::new(MetalSession {
            m: self,
            prog,
            ops: opsb,
            consts: constsb,
            cols: [a, b],
            cur: 0,
            ncols,
            stride: len,
            len,
        }))
    }

    fn merkle_levels(
        &self,
        leaves: &[[u64; 4]],
        arities: &[usize],
        tag: u64,
    ) -> Option<Vec<Vec<[u64; 4]>>> {
        if leaves.len() < MIN_BATCH || arities.iter().any(|&a| a != 2 && a != 4) {
            return None;
        }
        // every level in one buffer: leaves, then each level in turn
        let mut counts = Vec::with_capacity(arities.len());
        let mut c = leaves.len();
        for &a in arities {
            c /= a;
            counts.push(c);
        }
        let total: usize = leaves.len() + counts.iter().sum::<usize>();
        let g = self.lock();
        let buf = g.gpu.buffer(total * 32).ok()?;
        write(&buf, leaves);
        aruminium::autorelease_pool(|| {
            let cmd = g.queue.commands().expect("metal: commands");
            let enc = cmd.encoder().expect("metal: encoder");
            enc.bind(&g.level);
            let w = g.level.thread_execution_width().max(1);
            let grp = ((g.level.max_total_threads_per_threadgroup() / w).max(1) * w).min(256);
            let mut src = 0usize;
            let mut dst = leaves.len();
            for (&a, &n) in arities.iter().zip(&counts) {
                let p = LevelParams {
                    count: n as u32,
                    arity: a as u32,
                    tag,
                };
                enc.bind_buffer(&buf, src * 32, 0);
                enc.bind_buffer(&buf, dst * 32, 1);
                enc.push(bytes_of(&p), 2);
                enc.launch((n, 1, 1), (grp, 1, 1));
                src = dst;
                dst += n;
            }
            enc.finish();
            cmd.submit();
            cmd.wait();
            if let Some(e) = cmd.error() {
                panic!("metal merkle levels: {e}");
            }
        });
        let mut all = vec![[0u64; 4]; total];
        read(&buf, &mut all);
        let mut out = Vec::with_capacity(counts.len());
        let mut at = leaves.len();
        for &n in &counts {
            out.push(all[at..at + n].to_vec());
            at += n;
        }
        Some(out)
    }

    fn coset_leaves(
        &self,
        job: &CosetLeaves<'_>,
        cosets: core::ops::Range<usize>,
    ) -> Option<Vec<[u64; 4]>> {
        self.leaves_of(job, cosets, None)
    }

    fn coset_leaves_kept(
        &self,
        job: &CosetLeaves<'_>,
        cosets: core::ops::Range<usize>,
        values: &mut [u64],
    ) -> Option<Vec<[u64; 4]>> {
        self.leaves_of(job, cosets, Some(values))
    }

    fn coset_ntt(&self, coeffs: &[u64], shifts: &[u64]) -> Vec<Vec<u64>> {
        let n = coeffs.len();
        if n < MIN_NTT || shifts.is_empty() {
            return Cpu.coset_ntt(coeffs, shifts);
        }
        assert!(n.is_power_of_two() && n <= 1 << 31);
        let log_n = n.trailing_zeros();
        let mut g = self.lock();
        if g.twiddles.as_ref().is_none_or(|(l, _)| *l != log_n) {
            g.twiddles = None;
            let tw = twiddles(n);
            let b = g.gpu.buffer(n * 8).expect("metal: twiddles");
            write(&b, &tw);
            g.twiddles = Some((log_n, b));
        }
        let inner = &*g;
        let tw = &inner.twiddles.as_ref().expect("twiddles").1;
        let total = n * shifts.len();
        let cbuf = inner.gpu.buffer(n * 8).expect("metal: coefficients");
        write(&cbuf, coeffs);
        let sbuf = inner.gpu.buffer(shifts.len() * 8).expect("metal: shifts");
        write(&sbuf, shifts);
        let data = inner.gpu.buffer(total * 8).expect("metal: cosets");
        let cosets = shifts.len() as u32;
        aruminium::autorelease_pool(|| {
            let cmd = inner.queue.commands().expect("metal: commands");
            let enc = cmd.encoder().expect("metal: encoder");
            let group = |p: &Pipeline| {
                let w = p.thread_execution_width().max(1);
                ((p.max_total_threads_per_threadgroup() / w).max(1) * w).min(256)
            };
            let p0 = NttParams {
                log_n,
                span: 0,
                cosets,
                pad: 0,
            };
            enc.bind(&inner.scatter);
            enc.bind_buffer(&cbuf, 0, 0);
            enc.bind_buffer(&data, 0, 1);
            enc.bind_buffer(&sbuf, 0, 2);
            enc.push(bytes_of(&p0), 3);
            enc.launch((total, 1, 1), (group(&inner.scatter), 1, 1));
            stages(inner, &enc, &data, 0, tw, log_n, cosets as usize, &group);
            enc.finish();
            cmd.submit();
            cmd.wait();
            if let Some(e) = cmd.error() {
                panic!("metal ntt: {e}");
            }
        });
        let mut flat = vec![0u64; total];
        read(&data, &mut flat);
        flat.chunks_exact(n).map(<[u64]>::to_vec).collect()
    }

    fn grind(&self, template: &State, inject: Inject, bits: u32) -> u64 {
        assert!((1..64).contains(&bits), "grinding bits in 1..64");
        let g = self.lock();
        write(&g.tpl, template);
        let mut base = 0u64;
        // the least nonce is expected near 2^bits: dispatches start
        // there and double (a fixed 2^22 after a 2^16 probe cost a
        // 16-bit grind 4M nonces in a third of the calls)
        let mut count = 1u64 << bits.clamp(12, GRIND_DISPATCH.trailing_zeros());
        loop {
            write(&g.found, &[u32::MAX]);
            let p = GrindParams {
                base,
                mask: (1u64 << bits) - 1,
                lo_mask: if inject.lo_bits >= 64 {
                    u64::MAX
                } else {
                    (1u64 << inject.lo_bits) - 1
                },
                lo: inject.lo as u32,
                lo_shift: inject.lo_shift,
                hi: inject.hi as u32,
                hi_shift: inject.hi_shift,
                count: count as u32,
                pad: 0,
            };
            write(&g.params, &[p]);
            run(
                &g,
                &g.grind,
                &[(&g.tpl, 0), (&g.found, 1), (&g.params, 2)],
                None,
                (count as usize).div_ceil(self.k),
            )
            .expect("metal grind");
            let mut f = [0u32];
            read(&g.found, &mut f);
            if f[0] != u32::MAX {
                return base + f[0] as u64;
            }
            base += count;
            count = (2 * count).min(GRIND_DISPATCH);
        }
    }
}

impl Metal {
    /// [`Backend::coset_leaves`]; with `values`, the coset values copied out.
    fn leaves_of(
        &self,
        job: &CosetLeaves<'_>,
        cosets: core::ops::Range<usize>,
        values: Option<&mut [u64]>,
    ) -> Option<Vec<[u64; 4]>> {
        let (k, per, g) = (job.k(), job.per(), cosets.len());
        if k < MIN_NTT || job.rate > 16 || g == 0 || !k.is_power_of_two() {
            return None;
        }
        let log_k = k.trailing_zeros();
        let ml = job.limbs.len();
        let (spb, blocks) = job.blocks();
        let mut gd = self.lock();
        if gd.twiddles.as_ref().is_none_or(|(l, _)| *l != log_k) {
            gd.twiddles = None;
            let tw = twiddles(k);
            let b = gd.gpu.buffer(k * 8).expect("metal: twiddles");
            write(&b, &tw);
            gd.twiddles = Some((log_k, b));
        }
        let (need_c, need_v, need_o) = (ml * k * 8, ml * g * k * 8, g * per * 32);
        if gd
            .leaf_bufs
            .as_ref()
            .is_none_or(|(c, v, o)| c.size() < need_c || v.size() < need_v || o.size() < need_o)
        {
            gd.leaf_bufs = None;
            let c = gd
                .gpu
                .buffer(need_c.next_power_of_two())
                .expect("metal: coefficients");
            let v = gd
                .gpu
                .buffer(need_v.next_power_of_two())
                .expect("metal: coset values");
            let o = gd
                .gpu
                .buffer(need_o.next_power_of_two())
                .expect("metal: leaf digests");
            gd.leaf_bufs = Some((c, v, o));
        }
        let inner = &*gd;
        let (cbuf, vbuf, obuf) = inner.leaf_bufs.as_ref().expect("leaf buffers");
        for (j, l) in job.limbs.iter().enumerate() {
            // SAFETY: shared buffer of at least `ml·k` u64; no GPU work in flight
            unsafe {
                core::ptr::copy_nonoverlapping(
                    l.as_ptr(),
                    (cbuf.contents_ptr() as *mut u64).add(j * k),
                    k,
                )
            };
        }
        let shifts: Vec<u64> = cosets.clone().map(|s| job.shift(s)).collect();
        let sbuf = inner.gpu.buffer(g * 8).expect("metal: shifts");
        write(&sbuf, &shifts);
        let tw = &inner.twiddles.as_ref().expect("twiddles").1;
        let lp = CosetLeafParams {
            init: job.init,
            k: k as u32,
            per: per as u32,
            g: g as u32,
            w: 1 << job.log_width,
            members: job.members() as u32,
            limbs: job.limbs_per_member as u32,
            spb: spb as u32,
            blocks: blocks as u32,
            rate: job.rate as u32,
            pad: 0,
        };
        aruminium::autorelease_pool(|| {
            let cmd = inner.queue.commands().expect("metal: commands");
            let enc = cmd.encoder().expect("metal: encoder");
            let group = |p: &Pipeline| {
                let w = p.thread_execution_width().max(1);
                ((p.max_total_threads_per_threadgroup() / w).max(1) * w).min(256)
            };
            for j in 0..ml {
                let p0 = NttParams {
                    log_n: log_k,
                    span: 0,
                    cosets: g as u32,
                    pad: 0,
                };
                enc.bind(&inner.scatter);
                enc.bind_buffer(cbuf, j * k * 8, 0);
                enc.bind_buffer(vbuf, j * g * k * 8, 1);
                enc.bind_buffer(&sbuf, 0, 2);
                enc.push(bytes_of(&p0), 3);
                enc.launch((g * k, 1, 1), (group(&inner.scatter), 1, 1));
                stages(inner, &enc, vbuf, j * g * k * 8, tw, log_k, g, &group);
            }
            enc.bind(&inner.leaves);
            enc.bind_buffer(vbuf, 0, 0);
            enc.bind_buffer(obuf, 0, 1);
            enc.push(bytes_of(&lp), 2);
            enc.launch((g * per, 1, 1), (group(&inner.leaves), 1, 1));
            enc.finish();
            cmd.submit();
            cmd.wait();
            if let Some(e) = cmd.error() {
                panic!("metal coset leaves: {e}");
            }
        });
        let mut out = vec![[0u64; 4]; g * per];
        read(obuf, &mut out);
        if let Some(v) = values {
            assert_eq!(v.len(), ml * g * k, "coset values");
            read(vbuf, v);
        }
        Some(out)
    }
}

/// Encode the radix-2 stages of `cosets` NTTs of `2^log_n` in `buf` from
/// `off`: the first `min(log_n, 11)` in threadgroup memory in one dispatch,
/// the rest one dispatch each.
fn stages(
    inner: &Inner,
    enc: &aruminium::Encoder,
    buf: &Buffer,
    off: usize,
    tw: &Buffer,
    log_n: u32,
    cosets: usize,
    group: &dyn Fn(&Pipeline) -> usize,
) {
    let total = cosets << log_n;
    // a block needs 2^(fused − 1) threads in one threadgroup
    let max_t = inner.block.max_total_threads_per_threadgroup().max(1);
    let fused = log_n.min(BLOCK_LOG).min(max_t.ilog2() + 1);
    if fused >= 1 {
        enc.bind(&inner.block);
        enc.bind_buffer(buf, off, 0);
        enc.bind_buffer(tw, 0, 1);
        let p = NttParams {
            log_n,
            span: fused,
            cosets: cosets as u32,
            pad: 0,
        };
        enc.push(bytes_of(&p), 2);
        enc.launch_groups((total >> fused, 1, 1), (1 << (fused - 1), 1, 1));
    }
    enc.bind(&inner.stage);
    enc.bind_buffer(buf, off, 0);
    enc.bind_buffer(tw, 0, 1);
    for st in fused..log_n {
        let p = NttParams {
            log_n,
            span: 1 << st,
            cosets: cosets as u32,
            pad: 0,
        };
        enc.push(bytes_of(&p), 2);
        enc.launch((total / 2, 1, 1), (group(&inner.stage), 1, 1));
    }
}

/// Stages fused in threadgroup memory (2^11 elements: 16 KiB).
const BLOCK_LOG: u32 = 11;

/// `tw[h + i] = ω_{2h}^i` for every stage span `h < n` (nebu's roots:
/// `ω_m = 7^((p−1)/m)`).
fn twiddles(n: usize) -> Vec<u64> {
    use nebu::Goldilocks as G;
    let mut tw = vec![0u64; n.max(2)];
    let mut h = 1;
    while h < n {
        let w = G::new(7).exp((nebu::field::P - 1) / (2 * h) as u64);
        let mut x = G::ONE;
        for i in 0..h {
            tw[h + i] = x.as_u64();
            x *= w;
        }
        h *= 2;
    }
    tw
}
