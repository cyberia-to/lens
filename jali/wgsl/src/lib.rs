// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! jali-wgsl — GPU backend for polynomial ring NTT via wgpu compute shaders.
//!
//! Provides GPU-accelerated ring operations for R_q = F_p[x]/(x^n+1):
//! - `run_ring_add`          — coefficient-wise addition
//! - `run_ring_pointwise_mul`— pointwise multiply (NTT domain)
//! - `run_ring_mul`          — full ring multiply: twist → NTT → pointwise → INTT → untwist

pub mod shaders;

use wgpu;
use wgpu::util::DeviceExt;

// ── WGSL sources ───────────────────────────────────────────────────
// Nebu field + NTT shaders composed with jali ring shaders at runtime.

const NEBU_FIELD: &str = include_str!("../../../nebu/wgsl/src/shaders/field.wgsl");
const NEBU_NTT: &str = include_str!("../../../nebu/wgsl/src/shaders/ntt.wgsl");
const NEBU_NTT_KERNELS: &str = include_str!("../../../nebu/wgsl/src/shaders/ntt_kernels.wgsl");
const RING_WGSL: &str = include_str!("shaders/ring.wgsl");

/// Shim that bridges nebu's gl_* functions (vec2<u32>) to the Fp struct
/// type expected by ring.wgsl.
const FP_SHIM: &str = r#"
// ── Fp struct shim ──────────────────────────────────────────────────
// Wraps vec2<u32> Goldilocks representation into a named struct for
// typed storage buffers used by ring.wgsl.

struct Fp {
    lo: u32,
    hi: u32,
}

fn fp_add(a: Fp, b: Fp) -> Fp {
    let r = gl_add(a.lo, a.hi, b.lo, b.hi);
    return Fp(r.x, r.y);
}

fn fp_sub(a: Fp, b: Fp) -> Fp {
    let r = gl_sub(a.lo, a.hi, b.lo, b.hi);
    return Fp(r.x, r.y);
}

fn fp_mul(a: Fp, b: Fp) -> Fp {
    let r = gl_mul(a.lo, a.hi, b.lo, b.hi);
    return Fp(r.x, r.y);
}

fn fp_neg(a: Fp) -> Fp {
    let r = gl_neg(a.lo, a.hi);
    return Fp(r.x, r.y);
}
"#;

// ── GPU context ────────────────────────────────────────────────────

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    pub fn new() -> Option<Self> {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Option<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .ok()?;
        Some(Self { device, queue })
    }

    // ── Ring addition on GPU ───────────────────────────────────────

    /// GPU ring addition: out[i] = a[i] + b[i] for coefficient-wise add.
    /// Input/output as (lo, hi) Goldilocks limb pairs.
    pub fn run_ring_add(&self, a: &[(u32, u32)], b: &[(u32, u32)], n: usize) -> Vec<(u32, u32)> {
        assert_eq!(a.len(), n);
        assert_eq!(b.len(), n);

        let source = format!("{NEBU_FIELD}\n{FP_SHIM}\n{RING_WGSL}");
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ring_add"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let lhs_buf = self.create_fp_buffer("lhs", a);
        let rhs_buf = self.create_fp_buffer("rhs", b);
        let out_buf = self.create_empty_fp_buffer("out", n);
        let params_buf = self.create_ring_params(n as u32);

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ring_add"),
            layout: None,
            module: &module,
            entry_point: Some("ring_add"),
            compilation_options: Default::default(),
            cache: None,
        });

        let bind_group = self.create_ring_bind_group(&pipeline, &lhs_buf, &rhs_buf, &out_buf, &params_buf);
        let workgroups = (n as u32 + 255) / 256;

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);

        self.read_fp_buffer(&out_buf, n)
    }

    // ── Ring pointwise multiplication on GPU ───────────────────────

    /// GPU pointwise multiply in NTT domain: out[i] = a[i] * b[i].
    pub fn run_ring_pointwise_mul(&self, a: &[(u32, u32)], b: &[(u32, u32)], n: usize) -> Vec<(u32, u32)> {
        assert_eq!(a.len(), n);
        assert_eq!(b.len(), n);

        let source = format!("{NEBU_FIELD}\n{FP_SHIM}\n{RING_WGSL}");
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ring_pointwise_mul"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let lhs_buf = self.create_fp_buffer("lhs", a);
        let rhs_buf = self.create_fp_buffer("rhs", b);
        let out_buf = self.create_empty_fp_buffer("out", n);
        let params_buf = self.create_ring_params(n as u32);

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ring_pointwise_mul"),
            layout: None,
            module: &module,
            entry_point: Some("ring_pointwise_mul"),
            compilation_options: Default::default(),
            cache: None,
        });

        let bind_group = self.create_ring_bind_group(&pipeline, &lhs_buf, &rhs_buf, &out_buf, &params_buf);
        let workgroups = (n as u32 + 255) / 256;

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);

        self.read_fp_buffer(&out_buf, n)
    }

    // ── Full ring multiplication on GPU ────────────────────────────
    //
    // Multi-stage pipeline:
    //   1. twist(a, psi_table) → ta
    //   2. twist(b, psi_table) → tb
    //   3. NTT(ta) — log2(n) butterfly dispatches
    //   4. NTT(tb) — log2(n) butterfly dispatches
    //   5. pointwise_mul(ntt_a, ntt_b) → prod
    //   6. INTT(prod)
    //   7. untwist(prod, psi_inv_table) → result

    /// GPU ring multiplication: full negacyclic convolution.
    /// Both inputs must be coefficient-form polynomials of length n.
    pub fn run_ring_mul(&self, a: &[(u32, u32)], b: &[(u32, u32)], n: usize) -> Vec<(u32, u32)> {
        assert_eq!(a.len(), n);
        assert_eq!(b.len(), n);
        assert!(n.is_power_of_two());
        let k = n.trailing_zeros();

        // ── Precompute twist tables on host ────────────────────────
        let psi_table = compute_psi_table(n);
        let psi_inv_table = compute_psi_inv_table(n);

        // ── Compile shader modules ─────────────────────────────────
        let ring_source = format!("{NEBU_FIELD}\n{FP_SHIM}\n{RING_WGSL}");
        let ring_module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ring_ops"),
            source: wgpu::ShaderSource::Wgsl(ring_source.into()),
        });

        let ntt_source = format!("{NEBU_FIELD}\n{NEBU_NTT}\n{NEBU_NTT_KERNELS}");
        let ntt_module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ntt_ops"),
            source: wgpu::ShaderSource::Wgsl(ntt_source.into()),
        });

        // ── Create GPU buffers ─────────────────────────────────────
        let buf_a = self.create_fp_buffer("a", a);
        let buf_b = self.create_fp_buffer("b", b);
        let buf_psi = self.create_fp_buffer("psi", &psi_table);
        let buf_psi_inv = self.create_fp_buffer("psi_inv", &psi_inv_table);
        let buf_ta = self.create_rw_fp_buffer("ta", n);
        let buf_tb = self.create_rw_fp_buffer("tb", n);
        let buf_prod = self.create_rw_fp_buffer("prod", n);
        let params_buf = self.create_ring_params(n as u32);

        let workgroups_n = (n as u32 + 255) / 256;
        let workgroups_half = ((n / 2) as u32 + 255) / 256;

        // ── Stage 1: twist(a, psi) → ta ────────────────────────────
        self.dispatch_ring_op(&ring_module, "ring_twist",
                              &buf_a, &buf_psi, &buf_ta, &params_buf, workgroups_n);

        // ── Stage 2: twist(b, psi) → tb ────────────────────────────
        self.dispatch_ring_op(&ring_module, "ring_twist",
                              &buf_b, &buf_psi, &buf_tb, &params_buf, workgroups_n);

        // ── Stage 3: NTT(ta) in-place ──────────────────────────────
        self.dispatch_ntt_forward(&ntt_module, &buf_ta, n as u32, k, workgroups_n, workgroups_half);

        // ── Stage 4: NTT(tb) in-place ──────────────────────────────
        self.dispatch_ntt_forward(&ntt_module, &buf_tb, n as u32, k, workgroups_n, workgroups_half);

        // ── Stage 5: pointwise_mul(ntt_a, ntt_b) → prod ────────────
        self.dispatch_ring_op(&ring_module, "ring_pointwise_mul",
                              &buf_ta, &buf_tb, &buf_prod, &params_buf, workgroups_n);

        // ── Stage 6: INTT(prod) in-place ────────────────────────────
        self.dispatch_ntt_inverse(&ntt_module, &buf_prod, n as u32, k, workgroups_n, workgroups_half);

        // ── Stage 7: untwist(prod, psi_inv) → result ────────────────
        // We reuse buf_ta as the output buffer for the final result.
        self.dispatch_ring_op(&ring_module, "ring_untwist",
                              &buf_prod, &buf_psi_inv, &buf_ta, &params_buf, workgroups_n);

        // ── Read back result ────────────────────────────────────────
        self.read_fp_buffer(&buf_ta, n)
    }

    // ── Internal helpers ───────────────────────────────────────────

    /// Dispatch a ring shader entry point with the standard (lhs, rhs, out, params) layout.
    fn dispatch_ring_op(
        &self,
        module: &wgpu::ShaderModule,
        entry_point: &str,
        lhs: &wgpu::Buffer,
        rhs: &wgpu::Buffer,
        out: &wgpu::Buffer,
        params: &wgpu::Buffer,
        workgroups: u32,
    ) {
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(entry_point),
            layout: None,
            module,
            entry_point: Some(entry_point),
            compilation_options: Default::default(),
            cache: None,
        });

        let bind_group = self.create_ring_bind_group(&pipeline, lhs, rhs, out, params);

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);
    }

    /// Dispatch a single NTT pass (bit_reverse, ntt_stage, intt_stage, or intt_scale).
    fn dispatch_ntt_pass(
        &self,
        module: &wgpu::ShaderModule,
        data_buf: &wgpu::Buffer,
        entry_point: &str,
        n: u32, k: u32, stage: u32,
        workgroups: u32,
    ) {
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(entry_point),
            layout: None,
            module,
            entry_point: Some(entry_point),
            compilation_options: Default::default(),
            cache: None,
        });

        let params = [n, k, stage, 0u32];
        let params_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ntt_params"),
            contents: bytemuck_cast_slice(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: data_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);
    }

    /// Run forward NTT on a buffer (in-place). The buffer must be read_write STORAGE.
    fn dispatch_ntt_forward(
        &self,
        module: &wgpu::ShaderModule,
        buf: &wgpu::Buffer,
        n: u32, k: u32,
        workgroups_n: u32,
        workgroups_half: u32,
    ) {
        self.dispatch_ntt_pass(module, buf, "bit_reverse_kernel", n, k, 0, workgroups_n);
        for s in 0..k {
            self.dispatch_ntt_pass(module, buf, "ntt_stage_kernel", n, k, s, workgroups_half);
        }
    }

    /// Run inverse NTT on a buffer (in-place).
    fn dispatch_ntt_inverse(
        &self,
        module: &wgpu::ShaderModule,
        buf: &wgpu::Buffer,
        n: u32, k: u32,
        workgroups_n: u32,
        workgroups_half: u32,
    ) {
        for s in (0..k).rev() {
            self.dispatch_ntt_pass(module, buf, "intt_stage_kernel", n, k, s, workgroups_half);
        }
        self.dispatch_ntt_pass(module, buf, "bit_reverse_kernel", n, k, 0, workgroups_n);
        self.dispatch_ntt_pass(module, buf, "intt_scale_kernel", n, k, 0, workgroups_n);
    }

    // ── Buffer creation helpers ────────────────────────────────────

    /// Create a read-only STORAGE buffer initialized with Fp data.
    fn create_fp_buffer(&self, label: &str, data: &[(u32, u32)]) -> wgpu::Buffer {
        let flat = flatten_fp(data);
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck_cast_slice(&flat),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        })
    }

    /// Create a read-write STORAGE buffer initialized with Fp data.
    fn create_rw_fp_buffer(&self, label: &str, n: usize) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (n * 2 * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Create an empty read-write output buffer for n Fp elements.
    fn create_empty_fp_buffer(&self, label: &str, n: usize) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (n * 2 * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Create the uniform buffer for RingParams { n, _pad0, _pad1, _pad2 }.
    fn create_ring_params(&self, n: u32) -> wgpu::Buffer {
        let params = [n, 0u32, 0u32, 0u32];
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ring_params"),
            contents: bytemuck_cast_slice(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    }

    /// Create a bind group for ring shaders: (lhs @0, rhs @1, out @2, params @3).
    fn create_ring_bind_group(
        &self,
        pipeline: &wgpu::ComputePipeline,
        lhs: &wgpu::Buffer,
        rhs: &wgpu::Buffer,
        out: &wgpu::Buffer,
        params: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        let layout = pipeline.get_bind_group_layout(0);
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: lhs.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: rhs.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: out.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params.as_entire_binding() },
            ],
        })
    }

    /// Read n Fp elements back from a GPU buffer.
    fn read_fp_buffer(&self, buf: &wgpu::Buffer, n: usize) -> Vec<(u32, u32)> {
        let size = (n * 2 * 4) as u64;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(buf, 0, &staging, 0, size);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let raw: Vec<u32> = data
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        drop(data);
        staging.unmap();

        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            result.push((raw[i * 2], raw[i * 2 + 1]));
        }
        result
    }
}

// ── Host-side twist table computation ──────────────────────────────
//
// Compute psi^i for i in [0, n) where psi = g^((p-1)/(2n)).
// All arithmetic in Goldilocks field (p = 2^64 - 2^32 + 1, g = 7).

/// Goldilocks modulus.
const P: u64 = 0xFFFFFFFF00000001;

/// Goldilocks field multiply on host (for precomputation).
/// Uses 128-bit arithmetic with Goldilocks reduction:
/// p = 2^64 - 2^32 + 1, so 2^64 ≡ 2^32 - 1 (mod p).
fn host_mul(a: u64, b: u64) -> u64 {
    let prod = (a as u128) * (b as u128);
    let lo = prod as u64;
    let hi = (prod >> 64) as u64;

    // Reduce: lo + hi * (2^32 - 1) mod p
    // hi * (2^32 - 1) can be up to ~96 bits, compute in 128-bit
    let epsilon = 0xFFFFFFFFu64;
    let he = (hi as u128) * (epsilon as u128);
    let sum = (lo as u128) + he;

    // sum is at most ~97 bits. Reduce again.
    let sum_lo = sum as u64;
    let sum_hi = (sum >> 64) as u64;

    // Second round: sum_lo + sum_hi * epsilon
    let (r, carry) = sum_lo.overflowing_add(sum_hi.wrapping_mul(epsilon));
    let mut result = r;
    if carry {
        result = result.wrapping_add(epsilon);
    }

    // Final reduction (at most 2 subtractions)
    if result >= P {
        result -= P;
    }
    if result >= P {
        result -= P;
    }
    result
}

/// Goldilocks exponentiation on host.
fn host_exp(mut base: u64, mut exp: u64) -> u64 {
    let mut result = 1u64;
    base %= P;
    while exp > 0 {
        if exp & 1 == 1 {
            result = host_mul(result, base);
        }
        exp >>= 1;
        base = host_mul(base, base);
    }
    result
}

/// Compute psi^i table for negacyclic twist.
/// psi = g^((p-1)/(2n)) where g=7.
fn compute_psi_table(n: usize) -> Vec<(u32, u32)> {
    let two_n = (2 * n) as u64;
    let psi = host_exp(7, (P - 1) / two_n);

    let mut table = Vec::with_capacity(n);
    let mut psi_pow = 1u64;
    for _ in 0..n {
        table.push((psi_pow as u32, (psi_pow >> 32) as u32));
        psi_pow = host_mul(psi_pow, psi);
    }
    table
}

/// Compute psi^(-i) table for negacyclic untwist.
fn compute_psi_inv_table(n: usize) -> Vec<(u32, u32)> {
    let two_n = (2 * n) as u64;
    let psi = host_exp(7, (P - 1) / two_n);
    let psi_inv = host_exp(psi, P - 2); // modular inverse

    let mut table = Vec::with_capacity(n);
    let mut psi_inv_pow = 1u64;
    for _ in 0..n {
        table.push((psi_inv_pow as u32, (psi_inv_pow >> 32) as u32));
        psi_inv_pow = host_mul(psi_inv_pow, psi_inv);
    }
    table
}

// ── Utility functions ──────────────────────────────────────────────

/// Flatten (lo, hi) pairs into a flat u32 array.
fn flatten_fp(data: &[(u32, u32)]) -> Vec<u32> {
    let mut flat = Vec::with_capacity(data.len() * 2);
    for &(lo, hi) in data {
        flat.push(lo);
        flat.push(hi);
    }
    flat
}

/// Safe cast of &[u32] to &[u8] without bytemuck dependency.
fn bytemuck_cast_slice(data: &[u32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4)
    }
}
