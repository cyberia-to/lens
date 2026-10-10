//! The Metal backend: hemera kernels in MSL on the Apple GPU through
//! honeycrisp's aruminium. Buffers are shared-storage (unified memory):
//! the CPU writes states where the GPU reads them, nothing crosses a bus.
//!
//! Batches below [`MIN_BATCH`] states stay on the CPU (a dispatch costs
//! ~0.2 ms); the NTT runs on the CPU backend (no Metal NTT yet).

use std::sync::Mutex;

use aruminium::{Buffer, Gpu, Pipeline, Queue};
use cyber_lens_rspcs::backend::{Backend, Cpu, Inject, State};

/// Fewer states than this run on the CPU backend.
pub const MIN_BATCH: usize = 1 << 12;
/// States per GPU thread by default (measured: `examples/gpu_bench`).
pub const PER_THREAD: usize = 1;
/// Nonces per grinding dispatch (the first dispatch tries 2^16).
const GRIND_DISPATCH: u64 = 1 << 22;

struct Inner {
    gpu: Gpu,
    queue: Queue,
    permute: Pipeline,
    grind: Pipeline,
    scatter: Pipeline,
    stage: Pipeline,
    sponge: Pipeline,
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
        let sponge = pipe("hemera_sponge")?;
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
                sponge,
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
            enc.bind(&inner.stage);
            enc.bind_buffer(&data, 0, 0);
            enc.bind_buffer(tw, 0, 1);
            for st in 0..log_n {
                let p = NttParams {
                    log_n,
                    span: 1 << st,
                    cosets,
                    pad: 0,
                };
                enc.push(bytes_of(&p), 2);
                enc.launch((total / 2, 1, 1), (group(&inner.stage), 1, 1));
            }
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
        let mut count = 1u64 << 16;
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
            count = GRIND_DISPATCH;
        }
    }
}

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
