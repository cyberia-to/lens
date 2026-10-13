// ---
// tags: lens, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! cyber-lens-gpu — GPU prover backends for lens's
//! [`cyber_lens_rspcs::backend::Backend`]:
//!
//! - [`metal::Metal`] (feature `metal`, macOS): MSL kernels over native
//!   64-bit integers on the Apple GPU through honeycrisp's aruminium,
//!   shared-storage buffers in unified memory;
//! - [`webgpu::WebGpu`] (feature `webgpu`): WGSL kernels over 32-bit limbs
//!   through wgpu — Vulkan (Android, Linux), DX12, Metal, GL, and the
//!   browser's WebGPU.
//!
//! Both compute exactly what the CPU backend computes (canonical limbs,
//! least grinding nonce); `tests/differential.rs` checks them on random
//! inputs. [`install`] picks one for the process.

pub mod consts;
#[cfg(all(feature = "metal", target_os = "macos"))]
pub mod metal;
#[cfg(all(feature = "metal", target_os = "macos"))]
mod msl;
#[cfg(feature = "webgpu")]
pub mod webgpu;
#[cfg(feature = "webgpu")]
mod wgsl;
#[cfg(feature = "webgpu")]
mod wgsl2;
#[cfg(feature = "webgpu")]
mod webgpu2;

use std::sync::Arc;

use cyber_lens_rspcs::backend::{self, Backend, Cpu};

/// Which backend to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Metal where available, else WebGPU if an adapter exists, else CPU.
    Auto,
    Cpu,
    Metal,
    WebGpu,
}

impl core::str::FromStr for Choice {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "auto" => Ok(Choice::Auto),
            "cpu" => Ok(Choice::Cpu),
            "metal" => Ok(Choice::Metal),
            "webgpu" => Ok(Choice::WebGpu),
            _ => Err(format!("unknown backend `{s}` (auto, cpu, metal, webgpu)")),
        }
    }
}

/// A backend and a description of the device it runs on.
pub struct Selected {
    pub backend: Arc<dyn Backend>,
    pub device: String,
}

impl core::fmt::Debug for Selected {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Selected")
            .field("backend", &self.backend.name())
            .field("device", &self.device)
            .finish()
    }
}

fn cpu() -> Selected {
    let t = std::thread::available_parallelism().map_or(1, |n| n.get());
    Selected {
        backend: Arc::new(Cpu),
        device: format!("{t} threads"),
    }
}

/// The Metal backend, or why not.
pub fn metal() -> Result<Selected, String> {
    #[cfg(all(feature = "metal", target_os = "macos"))]
    {
        let m = metal::Metal::new()?;
        let device = m.device().to_string();
        Ok(Selected {
            backend: Arc::new(m),
            device,
        })
    }
    #[cfg(not(all(feature = "metal", target_os = "macos")))]
    {
        Err("metal: not built (macOS with the `metal` feature)".into())
    }
}

/// The WebGPU backend, or why not.
pub fn webgpu() -> Result<Selected, String> {
    #[cfg(all(feature = "webgpu", not(target_arch = "wasm32")))]
    {
        let w = webgpu::WebGpu::new()?;
        let device = w.adapter().to_string();
        Ok(Selected {
            backend: Arc::new(w),
            device,
        })
    }
    #[cfg(all(feature = "webgpu", target_arch = "wasm32"))]
    {
        Err("webgpu: a browser cannot block on GPU readback; the synchronous prover needs an asynchronous backend there".into())
    }
    #[cfg(not(feature = "webgpu"))]
    {
        Err("webgpu: not built (the `webgpu` feature)".into())
    }
}

/// Resolve `choice` to a backend. `Auto` falls back silently; an explicit
/// choice that is unavailable is an error.
pub fn select(choice: Choice) -> Result<Selected, String> {
    match choice {
        Choice::Cpu => Ok(cpu()),
        Choice::Metal => metal(),
        Choice::WebGpu => webgpu(),
        Choice::Auto => Ok(metal().or_else(|_| webgpu()).unwrap_or_else(|_| cpu())),
    }
}

/// Select and install for the process; returns what was installed.
pub fn install(choice: Choice) -> Result<(&'static str, String), String> {
    let s = select(choice)?;
    let name = s.backend.name();
    backend::install(s.backend);
    Ok((name, s.device))
}
