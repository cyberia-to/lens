//! WebGPU device-resident paths: line sessions (zerocheck columns stay on
//! the device), coset NTTs with a workgroup-memory block of stages,
//! streamed Reed–Solomon leaves, Merkle levels. WGSL in
//! [`crate::wgsl2`]; at most four storage buffers per kernel.

use cyber_lens_rspcs::backend::{CosetLeaves, LineSession};
use cyber_lens_rspcs::program::{Limbs, Program, from_fp3, to_fp3};
use nebu::Fp3;
use wgpu::util::DeviceExt;

/// Invocations per workgroup of the 64-wide kernels.
const WG: u32 = 64;
/// Workgroups along x before a dispatch wraps into y.
const ROW: u32 = 1 << 15;
/// Stages fused in workgroup memory (512 elements, 256 invocations).
const BLOCK_LOG: u32 = 9;

pub struct Ext {
    lines: wgpu::ComputePipeline,
    fold: wgpu::ComputePipeline,
    scatter: wgpu::ComputePipeline,
    stage: wgpu::ComputePipeline,
    block: wgpu::ComputePipeline,
    leaves: wgpu::ComputePipeline,
    level: wgpu::ComputePipeline,
}

fn pipe(device: &wgpu::Device, src: &str, entry: &str) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(entry), source: wgpu::ShaderSource::Wgsl(src.into()) });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry),
        layout: None,
        module: &module,
        entry_point: Some(entry),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

impl Ext {
    pub fn new(device: &wgpu::Device) -> Self {
        let ntt = crate::wgsl2::ntt();
        Self {
            lines: pipe(device, &crate::wgsl2::lines_session(), "prog_lines_s"),
            fold: pipe(device, &crate::wgsl2::fold_cols(), "fold_cols"),
            scatter: pipe(device, &ntt, "ntt_scatter"),
            stage: pipe(device, &ntt, "ntt_stage"),
            block: pipe(device, &ntt, "ntt_block"),
            leaves: pipe(device, &crate::wgsl2::coset_leaves(), "coset_leaves"),
            level: pipe(device, &crate::wgsl2::merkle_level(), "merkle_level"),
        }
    }
}

/// The device, queue and hemera constants an operation runs with.
pub struct Ctx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub consts: &'a wgpu::Buffer,
    pub ext: &'a Ext,
    /// Largest storage binding, bytes.
    pub max_storage: u64,
}

fn u32s(v: &[u64]) -> Vec<u32> {
    v.iter().flat_map(|&x| [x as u32, (x >> 32) as u32]).collect()
}
fn bytes(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}
fn limbs_bytes(v: &[u64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// `(x, y)` workgroups and the row stride in invocations for `n` items.
fn grid(n: usize) -> (u32, u32, u32) {
    let groups = (n as u32).div_ceil(WG).max(1);
    let x = groups.min(ROW);
    (x, groups.div_ceil(x), x * WG)
}

impl Ctx<'_> {
    fn storage(&self, data: &[u8], rw: bool) -> wgpu::Buffer {
        let mut usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;
        if !rw {
            usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        }
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: if data.is_empty() { &[0u8; 16] } else { data }, usage })
    }
    fn empty(&self, size: u64) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: size.max(16),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
    fn uniform(&self, words: &[u32]) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: &bytes(words), usage: wgpu::BufferUsages::UNIFORM })
    }
    fn pass(&self, enc: &mut wgpu::CommandEncoder, p: &wgpu::ComputePipeline, entries: &[(u32, wgpu::BindingResource<'_>)], groups: (u32, u32)) {
        let entries: Vec<wgpu::BindGroupEntry<'_>> = entries.iter().map(|(b, r)| wgpu::BindGroupEntry { binding: *b, resource: r.clone() }).collect();
        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &p.get_bind_group_layout(0), entries: &entries });
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
        pass.set_pipeline(p);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(groups.0, groups.1, 1);
    }
    fn submit_read(&self, mut enc: wgpu::CommandEncoder, src: &wgpu::Buffer, offset: u64, size: u64) -> Vec<u8> {
        let read = self.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: size.max(16), usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        enc.copy_buffer_to_buffer(src, offset, &read, 0, size);
        self.queue.submit([enc.finish()]);
        let slice = read.slice(..size);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("webgpu: map"));
        self.device.poll(wgpu::PollType::wait_indefinitely()).expect("webgpu: poll");
        let out = slice.get_mapped_range().to_vec();
        read.unmap();
        out
    }
    fn encoder(&self) -> wgpu::CommandEncoder {
        self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }
    fn binding<'b>(buf: &'b wgpu::Buffer, offset: u64, size: u64) -> wgpu::BindingResource<'b> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: buf, offset, size: core::num::NonZeroU64::new(size) })
    }

    /// Coset NTTs of `cosets` shifts, `2^log_n` long, in `data` from
    /// `off` (bytes): coset powers and bit reversal from `coef` at
    /// `coef_off`, a workgroup-memory block of stages, then single stages.
    #[allow(clippy::too_many_arguments)]
    fn ntt(&self, enc: &mut wgpu::CommandEncoder, coef: &wgpu::Buffer, coef_off: u64, data: &wgpu::Buffer, off: u64, shifts: &wgpu::Buffer, tw: &wgpu::Buffer, log_n: u32, cosets: usize) {
        let n = 1usize << log_n;
        let total = n * cosets;
        let (x, y, row) = grid(total);
        let u = self.uniform(&[log_n, 0, cosets as u32, row]);
        self.pass(enc, &self.ext.scatter, &[(1, Self::binding(coef, coef_off, (n * 8) as u64)), (2, Self::binding(data, off, (total * 8) as u64)), (3, shifts.as_entire_binding()), (4, u.as_entire_binding())], (x, y));
        let fused = log_n.min(BLOCK_LOG);
        if fused >= 1 {
            let blocks = (total >> fused) as u32;
            let bx = blocks.min(ROW);
            let u = self.uniform(&[log_n, fused, cosets as u32, bx]);
            self.pass(enc, &self.ext.block, &[(2, Self::binding(data, off, (total * 8) as u64)), (4, u.as_entire_binding()), (5, tw.as_entire_binding())], (bx, blocks.div_ceil(bx)));
        }
        let (x, y, row) = grid(total / 2);
        for st in fused..log_n {
            let u = self.uniform(&[log_n, 1 << st, cosets as u32, row]);
            self.pass(enc, &self.ext.stage, &[(2, Self::binding(data, off, (total * 8) as u64)), (4, u.as_entire_binding()), (5, tw.as_entire_binding())], (x, y));
        }
    }
}

/// `tw[h + i] = ω_{2h}^i` (nebu's roots).
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

pub fn coset_ntt(c: &Ctx<'_>, coeffs: &[u64], shifts: &[u64]) -> Option<Vec<Vec<u64>>> {
    let n = coeffs.len();
    let total = n * shifts.len();
    if total as u64 * 8 > c.max_storage || n < 2 {
        return None;
    }
    let log_n = n.trailing_zeros();
    let coef = c.storage(&limbs_bytes(coeffs), false);
    let sh = c.storage(&limbs_bytes(shifts), false);
    let tw = c.storage(&limbs_bytes(&twiddles(n)), false);
    let data = c.empty((total * 8) as u64);
    let mut enc = c.encoder();
    c.ntt(&mut enc, &coef, 0, &data, 0, &sh, &tw, log_n, shifts.len());
    let raw = c.submit_read(enc, &data, 0, (total * 8) as u64);
    let flat: Vec<u64> = raw.chunks_exact(8).map(|b| u64::from_le_bytes(b.try_into().expect("8"))).collect();
    Some(flat.chunks_exact(n).map(<[u64]>::to_vec).collect())
}

pub fn coset_leaves(c: &Ctx<'_>, job: &CosetLeaves<'_>, cosets: core::ops::Range<usize>) -> Option<Vec<[u64; 4]>> {
    let (k, per, g) = (job.k(), job.per(), cosets.len());
    let ml = job.limbs.len();
    let vals_bytes = (ml * g * k * 8) as u64;
    if k < 1 << 12 || job.rate > 16 || g == 0 || vals_bytes > c.max_storage || ((g * k * 8) % 256) != 0 {
        return None;
    }
    let log_k = k.trailing_zeros();
    let (spb, blocks) = job.blocks();
    let coef: Vec<u64> = job.limbs.iter().flat_map(|l| l.iter().copied()).collect();
    let coefb = c.storage(&limbs_bytes(&coef), false);
    let shifts: Vec<u64> = cosets.clone().map(|s| job.shift(s)).collect();
    let sh = c.storage(&limbs_bytes(&shifts), false);
    let tw = c.storage(&limbs_bytes(&twiddles(k)), false);
    let vals = c.empty(vals_bytes);
    let out = c.empty((g * per * 32) as u64);
    let mut enc = c.encoder();
    for j in 0..ml {
        c.ntt(&mut enc, &coefb, (j * k * 8) as u64, &vals, (j * g * k * 8) as u64, &sh, &tw, log_k, g);
    }
    let (x, y, row) = grid(g * per);
    let mut words = u32s(&job.init);
    words.extend([k as u32, per as u32, g as u32, 1 << job.log_width, job.members() as u32, job.limbs_per_member as u32, spb as u32, blocks as u32, job.rate as u32, row, 0, 0]);
    let u = c.uniform(&words);
    c.pass(&mut enc, &c.ext.leaves, &[(0, c.consts.as_entire_binding()), (1, vals.as_entire_binding()), (2, out.as_entire_binding()), (3, u.as_entire_binding())], (x, y));
    let raw = c.submit_read(enc, &out, 0, (g * per * 32) as u64);
    Some(raw.chunks_exact(32).map(|d| core::array::from_fn(|i| u64::from_le_bytes(d[8 * i..8 * i + 8].try_into().expect("8")))).collect())
}

pub fn merkle_levels(c: &Ctx<'_>, leaves: &[[u64; 4]], arities: &[usize], tag: u64) -> Option<Vec<Vec<[u64; 4]>>> {
    if leaves.len() < 1 << 12 || (leaves.len() * 32) as u64 > c.max_storage || arities.iter().any(|&a| a != 2 && a != 4) {
        return None;
    }
    let mut enc = c.encoder();
    let mut src = c.storage(&limbs_bytes(leaves.as_flattened()), true);
    let mut n = leaves.len();
    let mut bufs = Vec::with_capacity(arities.len());
    for &a in arities {
        n /= a;
        let dst = c.empty((n * 32) as u64);
        let (x, y, row) = grid(n);
        let t = u32s(&[tag]);
        let u = c.uniform(&[n as u32, a as u32, row, 0, t[0], t[1], 0, 0]);
        c.pass(&mut enc, &c.ext.level, &[(0, c.consts.as_entire_binding()), (1, src.as_entire_binding()), (2, dst.as_entire_binding()), (3, u.as_entire_binding())], (x, y));
        bufs.push((dst.clone(), n));
        src = dst;
    }
    // every level into one read buffer
    let total: usize = bufs.iter().map(|b| b.1).sum();
    let all = c.empty((total * 32) as u64);
    let mut at = 0u64;
    for (b, m) in &bufs {
        enc.copy_buffer_to_buffer(b, 0, &all, at, (*m * 32) as u64);
        at += (*m * 32) as u64;
    }
    let raw = c.submit_read(enc, &all, 0, (total * 32) as u64);
    let digests: Vec<[u64; 4]> = raw.chunks_exact(32).map(|d| core::array::from_fn(|i| u64::from_le_bytes(d[8 * i..8 * i + 8].try_into().expect("8")))).collect();
    let mut out = Vec::with_capacity(bufs.len());
    let mut i = 0;
    for (_, m) in &bufs {
        out.push(digests[i..i + m].to_vec());
        i += m;
    }
    Some(out)
}

/// A line session: columns resident in two storage buffers (ping-pong).
pub struct Session<'a> {
    w: &'a crate::webgpu::WebGpu,
    prog: &'a Program,
    progb: wgpu::Buffer,
    consts_at: u32,
    cols: [wgpu::Buffer; 2],
    cur: usize,
    ncols: usize,
    stride: usize,
    len: usize,
}

pub fn session<'a>(c: &Ctx<'_>, w: &'a crate::webgpu::WebGpu, prog: &'a Program, cols: &[Limbs], len: usize) -> Option<Session<'a>> {
    let bytes_cols = (cols.len() * 24) as u64;
    if prog.regs > 160 || len < 2 || cols.len() % len != 0 || bytes_cols > c.max_storage {
        return None;
    }
    let mut pw: Vec<u32> = prog.ops.iter().flat_map(|o| [o.code as u32, o.a, o.b, o.dst]).collect();
    let consts_at = pw.len() as u32;
    pw.extend(u32s(prog.consts.as_flattened()));
    let progb = c.storage(&bytes(&pw), false);
    let a = c.storage(&limbs_bytes(cols.as_flattened()), true);
    let b = c.empty(bytes_cols);
    Some(Session { w, prog, progb, consts_at, cols: [a, b], cur: 0, ncols: cols.len() / len, stride: len, len })
}

impl LineSession for Session<'_> {
    fn sums(&mut self, weights: &[Limbs], xs: &[u64]) -> Vec<Limbs> {
        let pairs = self.len / 2;
        assert_eq!(weights.len(), pairs, "weights");
        let nx = xs.len();
        assert!(nx > 0 && nx <= 16);
        self.w.with_ctx(|c| {
            let wb = c.storage(&limbs_bytes(weights.as_flattened()), false);
            let mut xa = vec![0u64; 16];
            xa[..nx].copy_from_slice(xs);
            let per = ((1usize << 16) / nx).max(1);
            let mut sums = vec![Fp3::ZERO; nx];
            for first in (0..pairs).step_by(per) {
                let n = per.min(pairs - first);
                let threads = n * nx;
                let out = c.empty((threads * 24) as u64);
                let (x, y, row) = grid(threads);
                let mut words = vec![self.prog.ops.len() as u32, self.ncols as u32, self.stride as u32, nx as u32, threads as u32, first as u32, self.prog.output, self.consts_at, row, 0, 0, 0];
                words.extend(u32s(&xa));
                let u = c.uniform(&words);
                let mut enc = c.encoder();
                c.pass(&mut enc, &c.ext.lines, &[(1, self.progb.as_entire_binding()), (2, self.cols[self.cur].as_entire_binding()), (3, wb.as_entire_binding()), (4, out.as_entire_binding()), (5, u.as_entire_binding())], (x, y));
                let raw = c.submit_read(enc, &out, 0, (threads * 24) as u64);
                for (t, v) in raw.chunks_exact(24).enumerate() {
                    let l: [u64; 3] = core::array::from_fn(|q| u64::from_le_bytes(v[8 * q..8 * q + 8].try_into().expect("8")));
                    sums[t % nx] += to_fp3(l);
                }
            }
            sums.into_iter().map(from_fp3).collect()
        })
    }
    fn fold(&mut self, alpha: Limbs) {
        let half = self.len / 2;
        self.w.with_ctx(|c| {
            let (x, y, row) = grid(self.ncols * half);
            let mut words = u32s(&[alpha[0], alpha[1], alpha[2], 0]);
            words.extend([self.ncols as u32, self.stride as u32, half as u32, row]);
            let u = c.uniform(&words);
            let mut enc = c.encoder();
            c.pass(&mut enc, &c.ext.fold, &[(1, self.cols[self.cur].as_entire_binding()), (2, self.cols[1 - self.cur].as_entire_binding()), (3, u.as_entire_binding())], (x, y));
            c.queue.submit([enc.finish()]);
        });
        self.cur = 1 - self.cur;
        self.len = half;
    }
    fn firsts(&mut self) -> Vec<Limbs> {
        self.w.with_ctx(|c| {
            let all = c.empty((self.ncols * 24) as u64);
            let mut enc = c.encoder();
            for col in 0..self.ncols {
                enc.copy_buffer_to_buffer(&self.cols[self.cur], (col * self.stride * 24) as u64, &all, (col * 24) as u64, 24);
            }
            let raw = c.submit_read(enc, &all, 0, (self.ncols * 24) as u64);
            raw.chunks_exact(24).map(|v| core::array::from_fn(|q| u64::from_le_bytes(v[8 * q..8 * q + 8].try_into().expect("8")))).collect()
        })
    }
}
