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

pub mod checked;
pub mod consts;
pub mod profile;
#[cfg(all(feature = "metal", target_os = "macos"))]
pub mod metal;
#[cfg(all(feature = "metal", target_os = "macos"))]
mod msl;
#[cfg(feature = "webgpu")]
pub mod webgpu;
#[cfg(feature = "webgpu")]
mod webgpu2;
#[cfg(feature = "webgpu")]
mod wgsl;
#[cfg(feature = "webgpu")]
mod wgsl2;

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
    /// A device backend checked against the CPU on every operation.
    Check(Inner),
    /// A device backend spot-checked against the CPU on sampled outputs.
    Spot(Inner),
    /// A device backend recording every call's input and output hashes
    /// to the file `LENS_RECORD`.
    Record(Inner),
    /// A backend whose operations' wall times are added up per kind
    /// ([`profile::report`]).
    Profile(Option<Inner>),
}

/// The device a [`Choice::Check`] runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inner {
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
            "check-metal" => Ok(Choice::Check(Inner::Metal)),
            "check-webgpu" => Ok(Choice::Check(Inner::WebGpu)),
            "spot-metal" => Ok(Choice::Spot(Inner::Metal)),
            "spot-webgpu" => Ok(Choice::Spot(Inner::WebGpu)),
            "record-metal" => Ok(Choice::Record(Inner::Metal)),
            "record-webgpu" => Ok(Choice::Record(Inner::WebGpu)),
            "prof-cpu" => Ok(Choice::Profile(None)),
            "prof-metal" => Ok(Choice::Profile(Some(Inner::Metal))),
            "prof-webgpu" => Ok(Choice::Profile(Some(Inner::WebGpu))),
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
        Choice::Check(i) => {
            let s = if i == Inner::Metal {
                metal()?
            } else {
                webgpu()?
            };
            Ok(Selected {
                device: s.device,
                backend: Arc::new(checked::Checked(s.backend)),
            })
        }
        Choice::Spot(i) => {
            let s = if i == Inner::Metal {
                metal()?
            } else {
                webgpu()?
            };
            Ok(Selected {
                device: s.device,
                backend: Arc::new(checked::Spot(s.backend)),
            })
        }
        Choice::Record(i) => {
            let s = if i == Inner::Metal {
                metal()?
            } else {
                webgpu()?
            };
            let path =
                std::env::var("LENS_RECORD").map_err(|_| "record: set LENS_RECORD".to_string())?;
            let f = std::fs::File::create(&path).map_err(|e| format!("record: {e}"))?;
            Ok(Selected {
                device: s.device,
                backend: Arc::new(checked::Record(s.backend, std::sync::Mutex::new(f))),
            })
        }
        Choice::Profile(i) => {
            let s = match i {
                None => cpu(),
                Some(Inner::Metal) => metal()?,
                Some(Inner::WebGpu) => webgpu()?,
            };
            Ok(Selected {
                device: s.device,
                backend: Arc::new(profile::Profile(s.backend)),
            })
        }
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
