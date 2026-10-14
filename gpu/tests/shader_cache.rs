//! The Metal binary-archive shader cache never changes results: a written
//! archive, an archive hit, a corrupt or truncated archive, an unwritable
//! cache directory and a nonce'd (cold) source all give the same bytes as
//! the CPU backend for permutations, grinding and byte trees.

#![cfg(all(feature = "metal", target_os = "macos"))]

use std::path::PathBuf;

use cyber_lens_gpu::metal::{ArchiveUse, KERNEL_COUNT, Metal, Options, PER_THREAD, ShaderCache};
use cyber_lens_rspcs::backend::{Backend, ByteTree, Cpu, Inject, State, hash40_template};

const P: u64 = 0xFFFF_FFFF_0000_0001;

fn dir(tag: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "lens-shader-cache-test-{tag}-{}-{stamp}",
        std::process::id()
    ))
}

fn open(cache: ShaderCache, nonce: Option<&str>) -> Option<Metal> {
    match Metal::with_options(Options {
        per_thread: PER_THREAD,
        cache,
        concurrent: true,
        nonce: nonce.map(str::to_string),
    }) {
        Ok(m) => Some(m),
        Err(e) => {
            let required = std::env::var("LENS_GPU_REQUIRE").unwrap_or_default();
            assert!(
                !required.split(',').any(|r| r == "metal"),
                "metal required: {e}"
            );
            eprintln!("metal: skipped ({e})");
            None
        }
    }
}

/// Permutation, grinding and byte-tree outputs as bytes.
fn outputs(b: &dyn Backend) -> Vec<u8> {
    let mut seed = 0x1234_5678_9abc_def1u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % P
    };
    let mut states: Vec<State> = (0..1 << 13)
        .map(|_| core::array::from_fn(|_| next()))
        .collect();
    b.permute(&mut states);
    let mut out = Vec::new();
    for s in &states {
        for w in s {
            out.extend_from_slice(&w.to_le_bytes());
        }
    }
    let t = hash40_template(&[7u8; 32]);
    out.extend_from_slice(&b.grind(&t, Inject::hash40(), 12).to_le_bytes());
    let (limbs, width, leaves) = (3usize, 16usize, 1usize << 12);
    let symbols: Vec<u64> = (0..limbs * width * leaves).map(|_| next()).collect();
    let job = ByteTree {
        symbols: &symbols,
        limbs,
        width,
        leaves,
    };
    let root: [u8; 32] = match b.byte_tree(&job) {
        Some(levels) => {
            let r = levels.last().expect("root")[0];
            core::array::from_fn(|i| r[i / 8].to_le_bytes()[i % 8])
        }
        None => *cyber_lens_rspcs::merkle::Tree::build_on_cpu(leaves, |j| {
            let mut v = Vec::with_capacity(width * limbs * 8);
            for t in 0..width {
                for l in 0..limbs {
                    v.extend_from_slice(&symbols[(j + t * leaves) * limbs + l].to_le_bytes());
                }
            }
            v
        })
        .root()
        .as_bytes(),
    };
    out.extend_from_slice(&root);
    out
}

fn archive_file(m: &Metal) -> PathBuf {
    m.startup().archive_path.clone().expect("archive path")
}

#[test]
fn an_archive_hit_gives_identical_bytes() {
    let d = dir("hit");
    let Some(first) = open(ShaderCache::Dir(d.clone()), None) else {
        return;
    };
    assert_eq!(
        first.startup().archive,
        ArchiveUse::Written,
        "{:?}",
        first.startup()
    );
    assert!(archive_file(&first).is_file());
    let want = outputs(&first);
    assert_eq!(want, outputs(&Cpu), "metal (archive written) vs cpu");
    let second = open(ShaderCache::Dir(d.clone()), None).expect("metal");
    assert_eq!(
        second.startup().archive,
        ArchiveUse::Hit,
        "{:?}",
        second.startup()
    );
    assert_eq!(second.startup().archive_hits, KERNEL_COUNT);
    assert_eq!(outputs(&second), want, "archive hit vs first build");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_corrupt_or_truncated_archive_falls_back() {
    let d = dir("corrupt");
    let Some(first) = open(ShaderCache::Dir(d.clone()), None) else {
        return;
    };
    let path = archive_file(&first);
    let want = outputs(&Cpu);
    let good = std::fs::read(&path).expect("archive");
    for bad in [
        b"not a metal binary archive".repeat(100),
        good[..good.len() / 2].to_vec(),
        {
            let mut v = good.clone();
            for b in v.iter_mut().skip(4096).step_by(7) {
                *b ^= 0x5a;
            }
            v
        },
    ] {
        std::fs::write(&path, &bad).expect("write corrupt archive");
        let m = open(ShaderCache::Dir(d.clone()), None).expect("metal");
        eprintln!(
            "corrupt archive: {:?}, {} hits",
            m.startup().archive,
            m.startup().archive_hits
        );
        assert_eq!(
            outputs(&m),
            want,
            "after a corrupt archive ({:?})",
            m.startup()
        );
    }
    // the fallback rewrote a usable archive
    let m = open(ShaderCache::Dir(d.clone()), None).expect("metal");
    assert_eq!(m.startup().archive, ArchiveUse::Hit, "{:?}", m.startup());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_unwritable_cache_dir_falls_back() {
    let Some(m) = open(ShaderCache::Dir("/dev/null/lens".into()), None) else {
        return;
    };
    assert!(
        matches!(m.startup().archive, ArchiveUse::WriteFailed(_)),
        "{:?}",
        m.startup()
    );
    assert_eq!(outputs(&m), outputs(&Cpu));
}

#[test]
fn a_nonce_cold_build_gives_identical_bytes() {
    let stamp = format!(
        "t{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    );
    let Some(m) = open(ShaderCache::Off, Some(&stamp)) else {
        return;
    };
    assert_eq!(m.startup().archive, ArchiveUse::Off);
    assert_eq!(outputs(&m), outputs(&Cpu));
}
