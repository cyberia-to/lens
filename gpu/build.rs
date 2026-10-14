// ---
// tags: lens, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Offline Metal library, when the build machine can make one.
//!
//! With the `metal` feature on a macOS target and the offline compiler
//! present (`xcrun -f metal`, i.e. Xcode or the Metal toolchain), the
//! generated MSL (the same `msl::source` the runtime compiles, at the
//! default states-per-thread) is compiled to a `.metallib` and embedded with
//! the hemera hash of its source; the runtime uses it only when its own
//! source hashes equal. Otherwise `METALLIB` is `None` and the runtime
//! compiles the source (and caches pipelines in a binary archive).

#![allow(dead_code)]

#[path = "src/consts.rs"]
mod consts;
#[path = "src/msl.rs"]
mod msl;
#[path = "src/msl_tree.rs"]
mod msl_tree;

use std::path::PathBuf;
use std::process::Command;

/// `metal::PER_THREAD` (a mismatch only disables the embedded library: the
/// source hash would differ).
const PER_THREAD: usize = 4;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/msl.rs");
    println!("cargo:rerun-if-changed=src/msl_tree.rs");
    println!("cargo:rerun-if-changed=src/consts.rs");
    println!("cargo:rerun-if-env-changed=LENS_NO_METALLIB");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let rs = out.join("metallib.rs");
    let none = "/// No offline-compiled metallib in this build.\n\
                pub const METALLIB: Option<(&[u8], &str)> = None;\n";
    let body = match metallib(&out) {
        Ok(Some((lib, hash))) => format!(
            "/// The offline-compiled metallib and the hemera hash of its source.\n\
             pub const METALLIB: Option<(&[u8], &str)> = Some((include_bytes!({lib:?}), {hash:?}));\n"
        ),
        Ok(None) => none.to_string(),
        Err(e) => {
            println!("cargo:warning=lens-gpu: offline metallib skipped: {e}");
            none.to_string()
        }
    };
    std::fs::write(&rs, body).expect("write metallib.rs");
}

fn metallib(out: &std::path::Path) -> Result<Option<(PathBuf, String)>, String> {
    let macos = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|o| o == "macos");
    if !macos || std::env::var_os("CARGO_FEATURE_METAL").is_none() {
        return Ok(None);
    }
    if std::env::var_os("LENS_NO_METALLIB").is_some() {
        return Ok(None);
    }
    let found = Command::new("xcrun")
        .args(["-sdk", "macosx", "-f", "metal"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !found {
        return Ok(None);
    }
    let source = msl::source(PER_THREAD);
    let src = out.join("lens.metal");
    let air = out.join("lens.air");
    let lib = out.join("lens.metallib");
    std::fs::write(&src, &source).map_err(|e| e.to_string())?;
    run(Command::new("xcrun")
        .args(["-sdk", "macosx", "metal", "-c"])
        .arg(&src)
        .arg("-o")
        .arg(&air))?;
    run(Command::new("xcrun")
        .args(["-sdk", "macosx", "metallib"])
        .arg(&air)
        .arg("-o")
        .arg(&lib))?;
    let hash = cyber_hemera::hash(source.as_bytes()).to_hex();
    Ok(Some((lib, hash)))
}

fn run(c: &mut Command) -> Result<(), String> {
    let o = c.output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).into_owned())
    }
}
