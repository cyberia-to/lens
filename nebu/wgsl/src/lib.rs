// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! GPU backend for nebu — Goldilocks field via wgpu compute shaders.
//!
//! Provides GPU-accelerated operations:
//! - `eval_field_op` — run a single field expression on the GPU
//! - `run_test_vectors` — run the WGSL self-test shader
//! - `run_ntt` / `run_intt` — forward and inverse NTT
//! - `run_custom` — run arbitrary WGSL compute body

use wgpu;
use wgpu::util::DeviceExt;

// ── WGSL sources ───────────────────────────────────────────────────

const FIELD_WGSL: &str = include_str!("shaders/field.wgsl");
const EXTENSION_WGSL: &str = include_str!("shaders/extension.wgsl");
const TEST_VECTORS_WGSL: &str = include_str!("shaders/test_vectors.wgsl");
const NTT_WGSL: &str = include_str!("shaders/ntt.wgsl");
const NTT_KERNELS_WGSL: &str = include_str!("shaders/ntt_kernels.wgsl");

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

    /// Run the test_vectors.wgsl compute shader and return results.
    /// Returns a Vec of u32: 1 = pass, 0 = fail for each test.
    pub fn run_test_vectors(&self) -> Vec<u32> {
        let source = format!("{FIELD_WGSL}\n{EXTENSION_WGSL}\n{TEST_VECTORS_WGSL}");
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("test_vectors"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("test_vectors"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let result_size = 65 * 4u64;
        let gpu_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("results"),
            size: result_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: result_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("test_vectors"),
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
        encoder.copy_buffer_to_buffer(&gpu_buf, 0, &staging_buf, 0, result_size);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging_buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let results: Vec<u32> = data
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        drop(data);
        staging_buf.unmap();

        results
    }

    /// Run a single field operation on the GPU via a generated shader.
    /// `op`: WGSL expression producing vec2<u32>. Returns (lo, hi).
    pub fn eval_field_op(&self, op: &str) -> (u32, u32) {
        let source = format!(
            "{FIELD_WGSL}\n{EXTENSION_WGSL}\n\
            @group(0) @binding(0) var<storage, read_write> out: array<u32>;\n\
            @compute @workgroup_size(1)\n\
            fn main() {{\n\
                let raw = {op};\n\
                let r = gl_canon(raw.x, raw.y);\n\
                out[0] = r.x;\n\
                out[1] = r.y;\n\
            }}\n"
        );

        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("eval"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("eval"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let gpu_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 8,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 8,
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
        encoder.copy_buffer_to_buffer(&gpu_buf, 0, &staging, 0, 8);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let lo = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let hi = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        drop(data);
        staging.unmap();

        (lo, hi)
    }

    /// Run a custom compute shader body. The body writes to `out: array<u32>`.
    /// Returns `n_u32s` output values.
    pub fn run_custom(&self, body: &str, n_u32s: usize) -> Vec<u32> {
        let source = format!(
            "{FIELD_WGSL}\n{EXTENSION_WGSL}\n\
            @group(0) @binding(0) var<storage, read_write> out: array<u32>;\n\
            @compute @workgroup_size(1)\n\
            fn main() {{\n\
                {body}\n\
            }}\n"
        );

        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None, layout: None, module: &module,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });

        let size = (n_u32s * 4) as u64;
        let gpu_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None, size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None, size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0, resource: gpu_buf.as_entire_binding(),
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
        let results: Vec<u32> = data
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        drop(data);
        staging.unmap();

        results
    }

    /// Run forward NTT on GPU. Input/output as (lo, hi) pairs.
    pub fn run_ntt(&self, data: &mut [(u32, u32)]) {
        self.run_ntt_impl(data, true);
    }

    /// Run inverse NTT on GPU.
    pub fn run_intt(&self, data: &mut [(u32, u32)]) {
        self.run_ntt_impl(data, false);
    }

    fn run_ntt_impl(&self, data: &mut [(u32, u32)], forward: bool) {
        let n = data.len();
        assert!(n.is_power_of_two());
        let k = n.trailing_zeros();

        let source = format!("{FIELD_WGSL}\n{NTT_WGSL}\n{NTT_KERNELS_WGSL}");
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ntt"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let mut flat: Vec<u32> = Vec::with_capacity(n * 2);
        for &(lo, hi) in data.iter() {
            flat.push(lo);
            flat.push(hi);
        }

        let data_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ntt_data"),
            contents: bytemuck_cast_slice(&flat),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: (n * 2 * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let workgroups = ((n / 2) as u32 + 255) / 256;
        let workgroups_n = (n as u32 + 255) / 256;

        if forward {
            self.dispatch_ntt_pass(&module, &data_buf, "bit_reverse_kernel",
                                   n as u32, k, 0, workgroups_n);
            for s in 0..k {
                self.dispatch_ntt_pass(&module, &data_buf, "ntt_stage_kernel",
                                       n as u32, k, s, workgroups);
            }
        } else {
            for s in (0..k).rev() {
                self.dispatch_ntt_pass(&module, &data_buf, "intt_stage_kernel",
                                       n as u32, k, s, workgroups);
            }
            self.dispatch_ntt_pass(&module, &data_buf, "bit_reverse_kernel",
                                   n as u32, k, 0, workgroups_n);
            self.dispatch_ntt_pass(&module, &data_buf, "intt_scale_kernel",
                                   n as u32, k, 0, workgroups_n);
        }

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&data_buf, 0, &staging, 0, (n * 2 * 4) as u64);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        let mapped = slice.get_mapped_range();
        let result: Vec<u32> = mapped
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        drop(mapped);
        staging.unmap();

        for i in 0..n {
            data[i] = (result[i * 2], result[i * 2 + 1]);
        }
    }

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
}

/// Safe cast of &[u32] to &[u8] without bytemuck dependency.
fn bytemuck_cast_slice(data: &[u32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4)
    }
}
