//! Metal start-up cost: library compile, pipeline creation (serial or
//! concurrent), and the binary-archive cache — each scenario in a fresh
//! process; "cold OS cache" runs a copy of this example inside a fresh
//! `.app` bundle (Metal's shader cache is per bundle identifier), so the
//! shipped source is compiled as on a first run, without clearing any
//! system cache. `one` with a nonce instead emulates a kernel change.
//!
//! `cargo run --release -p cyber-lens-gpu --features metal --example metal_cold`
//! runs the sweep (`-- [rounds]`, default 3). One scenario by hand:
//! `metal_cold one <nonce|-> <cache dir|off> <serial|concurrent>`.
//!
//! Every line carries the 1-minute load average (the machine may be shared)
//! and a hash of a GPU permutation's output, which must be equal on all lines.

use std::process::Command;
use std::time::Instant;

use cyber_lens_gpu::metal::{Metal, Options, ShaderCache};
use cyber_lens_rspcs::backend::{Backend, State};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("one") {
        one(&args[1], &args[2], &args[3]);
        return;
    }
    let rounds: usize = args.first().and_then(|a| a.parse().ok()).unwrap_or(3);
    sweep(rounds);
}

fn one(nonce: &str, cache: &str, mode: &str) {
    let t = Instant::now();
    let m = Metal::with_options(Options {
        per_thread: cyber_lens_gpu::metal::PER_THREAD,
        cache: if cache == "off" {
            ShaderCache::Off
        } else {
            ShaderCache::Dir(cache.into())
        },
        concurrent: mode == "concurrent",
        nonce: (nonce != "-").then(|| nonce.to_string()),
    })
    .expect("metal");
    let total = t.elapsed();
    let s = m.startup();
    let n = cyber_lens_gpu::metal::MIN_BATCH;
    let mut st: Vec<State> = (0..n as u64)
        .map(|k| core::array::from_fn(|i| k * 16 + i as u64))
        .collect();
    m.permute(&mut st);
    let mut bytes = Vec::with_capacity(n * 128);
    for s in &st {
        for w in s {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
    }
    let h = cyber_hemera::hash(&bytes).to_hex();
    println!(
        "{:.1}\t{:.1}\t{:.1}\t{:.1}\t{}\t{:.1}\t{:?}\t{}\t{}",
        total.as_secs_f64() * 1e3,
        s.library.as_secs_f64() * 1e3,
        s.pipelines.as_secs_f64() * 1e3,
        s.archive_load.as_secs_f64() * 1e3,
        s.archive_hits,
        s.archive_save.as_secs_f64() * 1e3,
        s.archive,
        s.library_from,
        &h[..16],
    );
}

fn load() -> String {
    Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.split_whitespace().nth(1).map(str::to_string))
        .unwrap_or_default()
}

/// A copy of this executable inside a fresh `.app` with its own bundle
/// identifier: Metal keeps a per-bundle shader cache
/// (`$(getconf DARWIN_USER_CACHE_DIR)/<bundle id>/com.apple.metal*`), so
/// the copy starts with a cold OS cache — front end and back end — without
/// touching anyone else's cache.
struct Bundle {
    exe: std::path::PathBuf,
    id: String,
}

fn bundle(root: &std::path::Path, id: &str) -> Bundle {
    let contents = root.join(format!("{id}.app/Contents"));
    std::fs::create_dir_all(contents.join("MacOS")).expect("bundle dir");
    let exe = contents.join("MacOS/metal_cold");
    std::fs::copy(std::env::current_exe().expect("exe"), &exe).expect("copy exe");
    std::fs::write(
        contents.join("Info.plist"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\"><dict>\
             <key>CFBundleIdentifier</key><string>{id}</string>\
             <key>CFBundleExecutable</key><string>metal_cold</string>\
             <key>CFBundlePackageType</key><string>APPL</string></dict></plist>\n"
        ),
    )
    .expect("Info.plist");
    Bundle { exe, id: id.into() }
}

fn user_cache_dir() -> Option<std::path::PathBuf> {
    let o = Command::new("getconf")
        .arg("DARWIN_USER_CACHE_DIR")
        .output()
        .ok()?;
    let s = String::from_utf8(o.stdout).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.into())
}

fn sweep(rounds: usize) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("lens-metal-cold-{stamp}"));
    let archives = root.join("archives");
    let d = archives.to_str().expect("utf8").to_string();
    let mut ids = Vec::new();
    println!(
        "| round | scenario | load | Metal::new ms | library ms | pipelines ms | archive load ms | hits | archive save ms | archive | library | permute hash |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|");
    for r in 0..rounds {
        let mut fresh = |tag: &str| {
            let b = bundle(
                &root,
                &format!("to.cyberia.lens.metalcold.s{stamp}r{r}{tag}"),
            );
            ids.push(b.id.clone());
            b
        };
        let (a, b, c, e, f) = (fresh("a"), fresh("b"), fresh("c"), fresh("e"), fresh("f"));
        let scenarios: [(&str, &Bundle, &str, &str, &str); 8] = [
            (
                "cold OS cache, serial, no archive (old path)",
                &a,
                "-",
                "off",
                "serial",
            ),
            (
                "warm OS cache, serial, no archive",
                &a,
                "-",
                "off",
                "serial",
            ),
            (
                "cold OS cache, concurrent, no archive",
                &b,
                "-",
                "off",
                "concurrent",
            ),
            (
                "warm OS cache, concurrent, no archive",
                &b,
                "-",
                "off",
                "concurrent",
            ),
            (
                "cold OS cache, concurrent, archive absent (writes it)",
                &c,
                "-",
                &d,
                "concurrent",
            ),
            ("warm OS cache, archive hit", &c, "-", &d, "concurrent"),
            ("cold OS cache, archive hit", &e, "-", &d, "concurrent"),
            ("cold OS cache, archive hit, serial", &f, "-", &d, "serial"),
        ];
        for (name, bundle, nonce, cache, mode) in scenarios {
            let l = load();
            let o = Command::new(&bundle.exe)
                .args(["one", nonce, cache, mode])
                .output()
                .expect("spawn");
            let line = String::from_utf8_lossy(&o.stdout)
                .trim()
                .replace('\t', " | ");
            if !o.status.success() {
                eprintln!("{name}: {}", String::from_utf8_lossy(&o.stderr));
            }
            println!("| {r} | {name} | {l} | {line} |");
        }
        // the next round starts without an archive
        let _ = std::fs::remove_dir_all(&archives);
    }
    // remove the bundles and the OS caches they created (ours only)
    let _ = std::fs::remove_dir_all(&root);
    if let Some(c) = user_cache_dir() {
        for id in ids {
            let _ = std::fs::remove_dir_all(c.join(id));
        }
    }
}
