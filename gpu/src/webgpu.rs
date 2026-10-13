//! The WebGPU backend: hemera kernels in WGSL through wgpu (Metal, Vulkan,
//! DX12 or GL natively; the browser's WebGPU on wasm32). Readback waits on
//! the device, so the backend runs where a thread may block — native
//! targets and browser workers with a blocking poll; on the browser main
//! thread construction fails and the CPU backend proves.
//!
//! Batches below [`MIN_BATCH`] states stay on the CPU; the NTT runs on the
//! CPU backend (no WGSL NTT yet).

use std::sync::Mutex;

use cyber_lens_rspcs::backend::{Backend, Cpu, Inject, State};
use cyber_lens_rspcs::program::{Limbs, Program, from_fp3, to_fp3};
use nebu::Fp3;
use wgpu::util::DeviceExt;

/// Fewer states than this run on the CPU backend.
pub const MIN_BATCH: usize = 1 << 12;
/// Invocations per workgroup (the kernels' `@workgroup_size`).
const WG: u32 = 64;
/// Workgroups along x before a dispatch wraps into y.
const ROW_GROUPS: u32 = 1 << 15;
/// Nonces per grinding dispatch after the first (2^16).
const GRIND_DISPATCH: u64 = 1 << 22;

struct Inner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    consts: wgpu::Buffer,
    permute: wgpu::ComputePipeline,
    grind: wgpu::ComputePipeline,
    sponge: wgpu::ComputePipeline,
    ext: crate::webgpu2::Ext,
    /// Largest storage binding, bytes (uncapped).
    max_storage: u64,
    lines: wgpu::ComputePipeline,
    /// Largest storage binding, bytes.
    max_binding: u64,
    /// Largest states buffer the device binds, in states.
    max_states: usize,
}

/// Hemera kernels on a WebGPU adapter.
pub struct WebGpu {
    inner: Mutex<Inner>,
    adapter: String,
}

impl core::fmt::Debug for WebGpu {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WebGpu")
            .field("adapter", &self.adapter)
            .finish()
    }
}

fn layout_entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn u32s(v: &[u64]) -> Vec<u32> {
    v.iter()
        .flat_map(|&x| [x as u32, (x >> 32) as u32])
        .collect()
}

fn bytes(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// `(x, y)` workgroups and the row stride in invocations for `n` items.
fn grid(n: usize) -> (u32, u32, u32) {
    let groups = (n as u32).div_ceil(WG);
    let x = groups.min(ROW_GROUPS);
    (x, groups.div_ceil(x), x * WG)
}

impl WebGpu {
    /// The first high-performance adapter with compute shaders.
    pub fn new() -> Result<Self, String> {
        pollster::block_on(Self::new_async())
    }

    pub async fn new_async() -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("webgpu adapter: {e}"))?;
        if !adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
        {
            return Err("webgpu: adapter without compute shaders".into());
        }
        let info = adapter.get_info();
        let limits = adapter.limits();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("lens gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: limits.clone(),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("webgpu device: {e}"))?;
        let mut k = crate::consts::round_constants().to_vec();
        k.extend(crate::consts::diag());
        let consts = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hemera constants"),
            contents: &bytes(&u32s(&k)),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bgl = |second: wgpu::BufferBindingType| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    layout_entry(0, wgpu::BufferBindingType::Storage { read_only: true }),
                    layout_entry(1, second),
                    layout_entry(2, wgpu::BufferBindingType::Uniform),
                ],
            })
        };
        let rw = wgpu::BufferBindingType::Storage { read_only: false };
        let pipeline = |src: &str, entry: &str| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(entry),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bgl(rw)],
                immediate_size: 0,
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let permute = pipeline(crate::wgsl::SOURCE, "hemera_permute");
        let grind = pipeline(&crate::wgsl::grind_source(), "hemera_grind");
        let auto = |src: String, entry: &str| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(entry),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &module,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let lines = auto(crate::wgsl::lines_source(), "prog_lines");
        let sponge = {
            let src = crate::wgsl::sponge_source();
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("hemera_sponge"),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("hemera_sponge"),
                layout: None,
                module: &module,
                entry_point: Some("hemera_sponge"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let ext = crate::webgpu2::Ext::new(&device);
        let max_storage =
            (limits.max_storage_buffer_binding_size as u64).min(limits.max_buffer_size);
        let max_bytes = (limits.max_storage_buffer_binding_size as u64)
            .min(limits.max_buffer_size)
            .min(1 << 28);
        let max_states = (max_bytes / 128) as usize;
        Ok(Self {
            inner: Mutex::new(Inner {
                device,
                queue,
                consts,
                permute,
                grind,
                sponge,
                ext,
                max_storage,
                lines,
                max_binding: max_bytes,
                max_states,
            }),
            adapter: format!("{} ({:?})", info.name, info.backend),
        })
    }

    /// The adapter's name and native API.
    pub fn adapter(&self) -> &str {
        &self.adapter
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// [`Backend::coset_ntt`] without blocking (browsers): awaits the
    /// readback.
    pub async fn coset_ntt_async(&self, coeffs: &[u64], shifts: &[u64]) -> Option<Vec<Vec<u64>>> {
        let g = self.lock();
        let c = crate::webgpu2::Ctx {
            device: &g.device,
            queue: &g.queue,
            consts: &g.consts,
            ext: &g.ext,
            max_storage: g.max_storage,
        };
        crate::webgpu2::coset_ntt_async(&c, coeffs, shifts).await
    }

    /// [`Backend::merkle_levels`] without blocking (browsers).
    pub async fn merkle_levels_async(
        &self,
        leaves: &[[u64; 4]],
        arities: &[usize],
        tag: u64,
    ) -> Option<Vec<Vec<[u64; 4]>>> {
        let g = self.lock();
        let c = crate::webgpu2::Ctx {
            device: &g.device,
            queue: &g.queue,
            consts: &g.consts,
            ext: &g.ext,
            max_storage: g.max_storage,
        };
        crate::webgpu2::merkle_levels_async(&c, leaves, arities, tag).await
    }

    /// Run `f` with the device context (the backend locked).
    pub(crate) fn with_ctx<R>(&self, f: impl FnOnce(&crate::webgpu2::Ctx<'_>) -> R) -> R {
        let g = self.lock();
        let c = crate::webgpu2::Ctx {
            device: &g.device,
            queue: &g.queue,
            consts: &g.consts,
            ext: &g.ext,
            max_storage: g.max_storage,
        };
        f(&c)
    }
}

impl Inner {
    fn run(
        &self,
        pipe: &wgpu::ComputePipeline,
        io: &wgpu::Buffer,
        params: &[u8],
        n: usize,
        read: &wgpu::Buffer,
    ) -> Vec<u8> {
        let ub = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: params,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipe.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.consts.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: io.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ub.as_entire_binding(),
                },
            ],
        });
        let (x, y, _) = grid(n);
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(pipe);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(x, y, 1);
        }
        enc.copy_buffer_to_buffer(io, 0, read, 0, io.size());
        self.queue.submit([enc.finish()]);
        let slice = read.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("webgpu: map"));
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("webgpu: poll");
        let out = slice.get_mapped_range().to_vec();
        read.unmap();
        out
    }
}

impl Backend for WebGpu {
    fn name(&self) -> &'static str {
        "webgpu"
    }

    fn permute(&self, states: &mut [State]) {
        if states.len() < MIN_BATCH {
            return Cpu.permute(states);
        }
        let g = self.lock();
        for chunk in states.chunks_mut(g.max_states) {
            let data = bytes(&u32s(chunk.as_flattened()));
            let io = g
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("states"),
                    contents: &data,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                });
            let read = g.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: data.len() as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let (_, _, row) = grid(chunk.len());
            let p = bytes(&[chunk.len() as u32, row, 0, 0]);
            let out = g.run(&g.permute, &io, &p, chunk.len(), &read);
            for (s, w) in chunk.as_flattened_mut().iter_mut().zip(out.chunks_exact(8)) {
                *s = u64::from_le_bytes(w.try_into().expect("8 bytes"));
            }
        }
    }

    fn sponge(&self, init: &State, rate: usize, blocks: usize, lanes: &[u64]) -> Vec<[u64; 4]> {
        let per = rate * blocks;
        assert!((1..=16).contains(&rate) && blocks >= 1 && lanes.len() % per == 0);
        let items = lanes.len() / per;
        if items < MIN_BATCH {
            return Cpu.sponge(init, rate, blocks, lanes);
        }
        let g = self.lock();
        // items per dispatch: the lanes binding within the device limit
        let cap = (g.max_states * 128 / (per * 8)).max(1);
        let mut out = Vec::with_capacity(items);
        for chunk in lanes.chunks(cap * per) {
            let n = chunk.len() / per;
            let inb = g
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("sponge lanes"),
                    contents: &bytes(&u32s(chunk)),
                    usage: wgpu::BufferUsages::STORAGE,
                });
            let outb = g.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sponge digests"),
                size: (n * 32) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let read = g.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (n * 32) as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let (x, y, row) = grid(n);
            let mut p = u32s(init);
            p.extend([rate as u32, blocks as u32, n as u32, row]);
            let ub = g
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &bytes(&p),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let bg = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &g.sponge.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: g.consts.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: inb.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ub.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: outb.as_entire_binding(),
                    },
                ],
            });
            let mut enc = g
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&g.sponge);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(x, y, 1);
            }
            enc.copy_buffer_to_buffer(&outb, 0, &read, 0, outb.size());
            g.queue.submit([enc.finish()]);
            let slice = read.slice(..);
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("webgpu: map"));
            g.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("webgpu: poll");
            let data = slice.get_mapped_range().to_vec();
            read.unmap();
            for d in data.chunks_exact(32) {
                out.push(core::array::from_fn(|i| {
                    u64::from_le_bytes(d[8 * i..8 * i + 8].try_into().expect("8 bytes"))
                }));
            }
        }
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
        if prog.regs > 160 || xs.len() > 16 || xs.is_empty() || pairs * xs.len() < MIN_BATCH {
            return Cpu.line_sums(prog, cols, weights, xs);
        }
        let len = 2 * pairs;
        let ncols = prog.inputs as usize;
        assert_eq!(cols.len(), ncols * len, "cols");
        let g = self.lock();
        let mut progw: Vec<u32> = prog
            .ops
            .iter()
            .flat_map(|o| [o.code as u32, o.a, o.b, o.dst])
            .collect();
        let consts_at = progw.len() as u32;
        progw.extend(u32s(prog.consts.as_flattened()));
        let progb = g
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("program"),
                contents: &bytes(&progw),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let nx = xs.len();
        // pairs per dispatch: the chunk's rows within the binding limit and
        // a bounded number of threads
        let per_pair = (ncols * 2 + 1) * 24;
        let per = ((g.max_binding as usize / per_pair).min((1 << 16) / nx)).max(1);
        let mut xa = vec![0u64; 16];
        xa[..nx].copy_from_slice(xs);
        let mut sums = vec![Fp3::ZERO; nx];
        for first in (0..pairs).step_by(per) {
            let n = per.min(pairs - first);
            let mut data: Vec<u64> = Vec::with_capacity((ncols * 2 * n + n) * 3);
            for c in 0..ncols {
                data.extend(cols[c * len + 2 * first..c * len + 2 * (first + n)].as_flattened());
            }
            // in vec2 units: every limb is one vec2<u32>
            let weights_at = (ncols * 2 * n * 3) as u32;
            data.extend(weights[first..first + n].as_flattened());
            let datab = g
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("lines data"),
                    contents: &bytes(&u32s(&data)),
                    usage: wgpu::BufferUsages::STORAGE,
                });
            let threads = n * nx;
            let outb = g.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("line results"),
                size: (threads * 24) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let read = g.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (threads * 24) as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let (x, y, row) = grid(threads);
            let mut p = vec![
                prog.ops.len() as u32,
                ncols as u32,
                (2 * n) as u32,
                nx as u32,
                threads as u32,
                0,
                prog.output,
                consts_at,
                weights_at,
                row,
                0,
                0,
            ];
            p.extend(u32s(&xa));
            let ub = g
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &bytes(&p),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let bg = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &g.lines.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: progb.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: datab.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: outb.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: ub.as_entire_binding(),
                    },
                ],
            });
            let mut enc = g
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&g.lines);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(x, y, 1);
            }
            enc.copy_buffer_to_buffer(&outb, 0, &read, 0, outb.size());
            g.queue.submit([enc.finish()]);
            let slice = read.slice(..);
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("webgpu: map"));
            g.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("webgpu: poll");
            let raw = slice.get_mapped_range().to_vec();
            read.unmap();
            for (t, v) in raw.chunks_exact(24).enumerate() {
                let l: [u64; 3] = core::array::from_fn(|q| {
                    u64::from_le_bytes(v[8 * q..8 * q + 8].try_into().expect("8 bytes"))
                });
                sums[t % nx] += to_fp3(l);
            }
        }
        sums.into_iter().map(from_fp3).collect()
    }

    fn line_session<'a>(
        &'a self,
        prog: &'a Program,
        cols: &[Limbs],
        len: usize,
    ) -> Option<Box<dyn cyber_lens_rspcs::backend::LineSession + 'a>> {
        let s = self.with_ctx(|c| crate::webgpu2::session(c, self, prog, cols, len))?;
        Some(Box::new(s))
    }

    fn coset_leaves(
        &self,
        job: &cyber_lens_rspcs::backend::CosetLeaves<'_>,
        cosets: core::ops::Range<usize>,
    ) -> Option<Vec<[u64; 4]>> {
        self.with_ctx(|c| crate::webgpu2::coset_leaves(c, job, cosets))
    }

    fn merkle_levels(
        &self,
        leaves: &[[u64; 4]],
        arities: &[usize],
        tag: u64,
    ) -> Option<Vec<Vec<[u64; 4]>>> {
        self.with_ctx(|c| crate::webgpu2::merkle_levels(c, leaves, arities, tag))
    }

    fn coset_ntt(&self, coeffs: &[u64], shifts: &[u64]) -> Vec<Vec<u64>> {
        if coeffs.len() < 1 << 12 || shifts.is_empty() {
            return Cpu.coset_ntt(coeffs, shifts);
        }
        self.with_ctx(|c| crate::webgpu2::coset_ntt(c, coeffs, shifts))
            .unwrap_or_else(|| Cpu.coset_ntt(coeffs, shifts))
    }

    fn grind(&self, template: &State, inject: Inject, bits: u32) -> u64 {
        assert!((1..64).contains(&bits), "grinding bits in 1..64");
        let g = self.lock();
        let found = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("found"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let read = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mask = (1u64 << bits) - 1;
        let lo_mask = if inject.lo_bits >= 64 {
            u64::MAX
        } else {
            (1u64 << inject.lo_bits) - 1
        };
        let mut base = 0u64;
        let mut count = 1u64 << 16;
        loop {
            g.queue.write_buffer(&found, 0, &u32::MAX.to_le_bytes());
            let (_, _, row) = grid(count as usize);
            let mut p = u32s(template);
            p.extend(u32s(&[base, mask, lo_mask]));
            p.extend([
                inject.lo as u32,
                inject.lo_shift,
                inject.hi as u32,
                inject.hi_shift,
                count as u32,
                row,
            ]);
            let out = g.run(&g.grind, &found, &bytes(&p), count as usize, &read);
            let f = u32::from_le_bytes(out[..4].try_into().expect("4 bytes"));
            if f != u32::MAX {
                return base + f as u64;
            }
            base += count;
            count = GRIND_DISPATCH;
        }
    }
}
