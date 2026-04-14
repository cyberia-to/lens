// ---
// tags: trop, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! GPU backend for trop — tropical matrix multiplication via wgpu compute shaders.
//!
//! Provides GPU-accelerated operations:
//! - `eval_tropical_op` — run a single tropical expression on the GPU
//! - `run_matmul` — tropical matrix multiplication (tiled 16x16 compute)

pub mod shaders;

use wgpu;
use wgpu::util::DeviceExt;

// ── WGSL sources ───────────────────────────────────────────────────

const TROPICAL_WGSL: &str = include_str!("shaders/tropical.wgsl");

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

    /// Run a single tropical operation on the GPU via a generated shader.
    /// `op`: WGSL expression producing a u32. Returns the result.
    pub fn eval_tropical_op(&self, op: &str) -> u32 {
        let source = format!(
            "@group(0) @binding(0) var<storage, read_write> out: array<u32>;\n\
            \n\
            const TROP_INF: u32 = 0xFFFFFFFFu;\n\
            \n\
            fn trop_add(a: u32, b: u32) -> u32 {{ return min(a, b); }}\n\
            \n\
            fn trop_mul(a: u32, b: u32) -> u32 {{\n\
                if a == TROP_INF || b == TROP_INF {{ return TROP_INF; }}\n\
                let sum = a + b;\n\
                if sum < a || sum == TROP_INF {{ return TROP_INF; }}\n\
                return sum;\n\
            }}\n\
            \n\
            @compute @workgroup_size(1)\n\
            fn main() {{\n\
                let result = {op};\n\
                out[0] = result;\n\
            }}\n"
        );

        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("eval_tropical"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("eval_tropical"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let gpu_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 4,
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
        encoder.copy_buffer_to_buffer(&gpu_buf, 0, &staging, 0, 4);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let result = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        drop(data);
        staging.unmap();

        result
    }

    /// Run tropical matrix multiplication on the GPU.
    ///
    /// `a` and `b` are flat row-major u32 arrays of size `n*n`.
    /// Returns a flat row-major u32 array of size `n*n`.
    ///
    /// Uses the tiled 16x16 compute shader from `tropical.wgsl`.
    pub fn run_matmul(&self, a: &[u32], b: &[u32], n: usize) -> Vec<u32> {
        assert_eq!(a.len(), n * n, "matrix A size mismatch");
        assert_eq!(b.len(), n * n, "matrix B size mismatch");

        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tropical_matmul"),
            source: wgpu::ShaderSource::Wgsl(TROPICAL_WGSL.into()),
        });

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("tropical_matmul"),
            layout: None,
            module: &module,
            entry_point: Some("tropical_matmul"),
            compilation_options: Default::default(),
            cache: None,
        });

        let mat_size = (n * n * 4) as u64;

        let buf_a = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("matrix_a"),
            contents: bytemuck_cast_slice(a),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let buf_b = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("matrix_b"),
            contents: bytemuck_cast_slice(b),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let buf_c = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("matrix_c"),
            size: mat_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = [n as u32];
        let buf_params = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params"),
            contents: bytemuck_cast_slice(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: mat_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tropical_matmul"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buf_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buf_c.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buf_params.as_entire_binding(),
                },
            ],
        });

        // Dispatch enough 16x16 workgroups to cover n x n
        let wg = ((n as u32) + 15) / 16;

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(wg, wg, 1);
        }
        encoder.copy_buffer_to_buffer(&buf_c, 0, &staging, 0, mat_size);
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
}

/// Safe cast of &[u32] to &[u8] without bytemuck dependency.
fn bytemuck_cast_slice(data: &[u32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4)
    }
}
