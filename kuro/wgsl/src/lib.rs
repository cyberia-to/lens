//! kuro-wgsl — GPU backend for F2 tower arithmetic via wgpu compute shaders.
//!
//! Provides GPU-accelerated operations:
//! - `eval_tower_op` — run a single tower field expression on the GPU
//! - `run_packed_inner_product` — GPU-accelerated packed F2 inner product
//! - `run_custom` — run arbitrary WGSL compute body

pub mod shaders;

use wgpu;

// -- WGSL sources ------------------------------------------------------------

const TOWER_WGSL: &str = include_str!("shaders/tower.wgsl");

// -- GPU context -------------------------------------------------------------

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

    /// Evaluate a WGSL expression that returns an F2_128 (array<u32, 4>).
    /// The `op` string should be a WGSL expression producing an F2_128 value.
    /// Returns the result as a u128 (little-endian from 4 x u32).
    pub fn eval_tower_op(&self, op: &str) -> u128 {
        // Build a minimal compute shader: tower.wgsl + output binding + main
        // We strip the existing compute entry point and bindings from tower.wgsl
        // and provide our own single-output binding.
        let source = format!(
            "{tower}\n\
            @group(0) @binding(0) var<storage, read_write> out: array<u32>;\n\
            @compute @workgroup_size(1)\n\
            fn main() {{\n\
                let r = {op};\n\
                out[0] = r[0];\n\
                out[1] = r[1];\n\
                out[2] = r[2];\n\
                out[3] = r[3];\n\
            }}\n",
            tower = tower_lib_source(),
        );

        let results = self.run_shader(&source, 4);
        u32s_to_u128(&results)
    }

    /// Run packed inner product on GPU: popcount(a AND b) for 128-bit packed vectors.
    /// `a` and `b` are each 128 bits (u128), treated as packed F2 elements.
    /// Returns the popcount (a scalar u32).
    pub fn run_packed_inner_product(&self, a: u128, b: u128) -> u32 {
        let [a0, a1, a2, a3] = u128_to_u32s(a);
        let [b0, b1, b2, b3] = u128_to_u32s(b);
        let source = format!(
            "{tower}\n\
            @group(0) @binding(0) var<storage, read_write> out: array<u32>;\n\
            @compute @workgroup_size(1)\n\
            fn main() {{\n\
                let a = Packed128({a0}u, {a1}u, {a2}u, {a3}u);\n\
                let b = Packed128({b0}u, {b1}u, {b2}u, {b3}u);\n\
                out[0] = packed128_inner_product(a, b);\n\
            }}\n",
            tower = tower_lib_source(),
        );

        let results = self.run_shader(&source, 1);
        results[0]
    }

    /// Run a custom compute shader body. The body writes to `out: array<u32>`.
    /// Returns `n_u32s` output values.
    pub fn run_custom(&self, body: &str, n_u32s: usize) -> Vec<u32> {
        let source = format!(
            "{tower}\n\
            @group(0) @binding(0) var<storage, read_write> out: array<u32>;\n\
            @compute @workgroup_size(1)\n\
            fn main() {{\n\
                {body}\n\
            }}\n",
            tower = tower_lib_source(),
        );

        self.run_shader(&source, n_u32s)
    }

    /// Core dispatch: compile shader, create pipeline, run, read back results.
    fn run_shader(&self, source: &str, n_u32s: usize) -> Vec<u32> {
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("kuro_eval"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("kuro_eval"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
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
        let results: Vec<u32> = data
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        drop(data);
        staging.unmap();

        results
    }
}

// -- helpers -----------------------------------------------------------------

/// Return the tower WGSL source with the bulk compute entry point stripped.
/// The stock tower.wgsl has bindings (input_a, input_b, output_c, params)
/// and a `tower_mul_main` entry point. We strip everything starting from the
/// compute shader section so that callers can attach their own bindings and
/// entry points.
fn tower_lib_source() -> &'static str {
    // Find the marker: "// Compute shader: process arrays"
    // Then walk back to the preceding blank line to strip the whole block.
    if let Some(idx) = TOWER_WGSL.find("// Compute shader: process arrays") {
        let block_start = TOWER_WGSL[..idx]
            .rfind("\n\n")
            .map(|i| i + 1)
            .unwrap_or(idx);
        &TOWER_WGSL[..block_start]
    } else {
        TOWER_WGSL
    }
}

/// Convert 4 x u32 (little-endian) to u128.
fn u32s_to_u128(v: &[u32]) -> u128 {
    (v[0] as u128) | ((v[1] as u128) << 32) | ((v[2] as u128) << 64) | ((v[3] as u128) << 96)
}

/// Convert u128 to [u32; 4] (little-endian).
pub fn u128_to_u32s(v: u128) -> [u32; 4] {
    [
        v as u32,
        (v >> 32) as u32,
        (v >> 64) as u32,
        (v >> 96) as u32,
    ]
}
