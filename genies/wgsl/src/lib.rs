// ---
// tags: genies, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! GPU backend for genies — batch F_q arithmetic via wgpu compute shaders.
//!
//! Provides GPU-accelerated operations:
//! - `eval_fq_op` — run a single F_q expression on the GPU
//! - `run_batch_mul` — batch N independent F_q multiplications
//! - `run_batch_add` — batch N independent F_q additions

use wgpu;
use wgpu::util::DeviceExt;

mod shaders;

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

    /// Run a single F_q expression on the GPU via a generated shader.
    /// `op`: WGSL expression producing an `Fq` value.
    /// Returns the result as [u64; 8] (little-endian limbs).
    pub fn eval_fq_op(&self, op: &str) -> [u64; 8] {
        let source = format!(
            "{}\n\
            @group(0) @binding(0) var<storage, read_write> out: array<u32>;\n\
            @compute @workgroup_size(1)\n\
            fn main() {{\n\
                let r = {op};\n\
                for (var i = 0u; i < 16u; i++) {{\n\
                    out[i] = r.limbs[i];\n\
                }}\n\
            }}\n",
            shaders::FQ,
        );

        let results = self.run_shader_readback(&source, 16);
        u32x16_to_u64x8(&results)
    }

    /// Batch multiply N pairs of F_q elements on the GPU.
    /// Each pair (a, b) -> a * b mod q.
    pub fn run_batch_mul(&self, pairs: &[([u64; 8], [u64; 8])]) -> Vec<[u64; 8]> {
        self.run_batch_op("batch_fq_mul", pairs)
    }

    /// Batch add N pairs of F_q elements on the GPU.
    pub fn run_batch_add(&self, pairs: &[([u64; 8], [u64; 8])]) -> Vec<[u64; 8]> {
        self.run_batch_op("batch_fq_add", pairs)
    }

    fn run_batch_op(
        &self,
        entry_point: &str,
        pairs: &[([u64; 8], [u64; 8])],
    ) -> Vec<[u64; 8]> {
        let n = pairs.len();
        if n == 0 {
            return vec![];
        }

        let source = format!("{}\n{}", shaders::FQ, shaders::BATCH);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(entry_point),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

        let pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(entry_point),
                    layout: None,
                    module: &module,
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    cache: None,
                });

        // Flatten inputs to u32 arrays
        let mut a_data: Vec<u32> = Vec::with_capacity(n * 16);
        let mut b_data: Vec<u32> = Vec::with_capacity(n * 16);
        for &(a, b) in pairs {
            a_data.extend_from_slice(&u64x8_to_u32x16(&a));
            b_data.extend_from_slice(&u64x8_to_u32x16(&b));
        }

        let a_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("input_a"),
                contents: bytemuck_cast_slice(&a_data),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let b_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("input_b"),
                contents: bytemuck_cast_slice(&b_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let out_size = (n * 16 * 4) as u64;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output"),
            size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = [n as u32, 0u32, 0u32, 0u32];
        let params_buf =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("params"),
                    contents: bytemuck_cast_slice(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: out_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: out_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        let workgroups = ((n as u32) + 63) / 64;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&out_buf, 0, &staging, 0, out_size);
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

        // Convert each 16 x u32 result back to [u64; 8]
        raw.chunks_exact(16)
            .map(|chunk| {
                let mut arr = [0u32; 16];
                arr.copy_from_slice(chunk);
                u32x16_to_u64x8(&arr)
            })
            .collect()
    }

    /// Helper: compile a shader, dispatch 1 workgroup, read back `n_u32s` values.
    fn run_shader_readback(&self, source: &str, n_u32s: usize) -> [u32; 16] {
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("eval"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

        let pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("eval"),
                    layout: None,
                    module: &module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                });

        let size = (n_u32s * 4) as u64;
        let gpu_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: gpu_buf.as_entire_binding(),
            }],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&gpu_buf, 0, &staging, 0, size);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let mut result = [0u32; 16];
        for (i, chunk) in data.chunks_exact(4).enumerate().take(n_u32s) {
            result[i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        drop(data);
        staging.unmap();

        result
    }
}

// ── Conversion helpers ─────────────────────────────────────────────

/// Convert [u64; 8] (little-endian) to [u32; 16] (little-endian).
/// Each u64 splits into two u32: low half first.
fn u64x8_to_u32x16(v: &[u64; 8]) -> [u32; 16] {
    let mut out = [0u32; 16];
    for i in 0..8 {
        out[i * 2] = v[i] as u32;
        out[i * 2 + 1] = (v[i] >> 32) as u32;
    }
    out
}

/// Convert [u32; 16] (little-endian) back to [u64; 8].
fn u32x16_to_u64x8(v: &[u32; 16]) -> [u64; 8] {
    let mut out = [0u64; 8];
    for i in 0..8 {
        out[i] = (v[i * 2] as u64) | ((v[i * 2 + 1] as u64) << 32);
    }
    out
}

/// Safe cast of &[u32] to &[u8] without bytemuck dependency.
fn bytemuck_cast_slice(data: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}
