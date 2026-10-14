//! Shader start-up for the Metal backend: the MSL library and its compute
//! pipelines, built so that a process pays as little compilation as it can.
//!
//! Three layers, each optional, each falling back to the one below:
//!
//! 1. **embedded metallib** — when the build machine has the offline Metal
//!    compiler (`xcrun -f metal`), `build.rs` compiles the generated MSL to a
//!    `.metallib` and embeds it; it is used only if the source it was built
//!    from hashes equal to the source this process would compile.
//! 2. **binary archive** — a `MTLBinaryArchive` on disk (the cache directory,
//!    [`ShaderCache`]) holding the GPU binaries of the twelve pipelines. On a
//!    hit, pipeline creation skips the backend compiler entirely
//!    (`MTLPipelineOptionFailOnBinaryArchiveMiss` reports a miss instead of
//!    compiling); on a miss or a rejected file the pipelines compile as
//!    usual and a fresh archive is written (temp file + rename, so
//!    concurrent processes never see a torn file).
//! 3. **source compile** — `newLibraryWithSource` + the pipelines, created
//!    concurrently (one thread each).
//!
//! None of this changes what runs: a pipeline is the compilation of the same
//! function whichever path produced it, and Metal keys archive entries by the
//! function itself, so a foreign or stale archive can only miss.

use std::ffi::{CStr, c_void};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use aruminium::Gpu;
use aruminium::ffi::{
    CFRelease, CFTypeRef, NSUInteger, ObjcId, ObjcSel, nserror_string, nsstring, nsstring_to_rust,
    objc_getClass, objc_msgSend, release, sel_registerName,
};

/// The kernels, in the order [`build`] returns their pipelines.
pub(crate) const KERNELS: [&str; 12] = [
    "hemera_permute",
    "hemera_grind",
    "ntt_scatter",
    "ntt_stage",
    "ntt_block",
    "hemera_sponge",
    "prog_lines",
    "coset_leaves",
    "fold_cols",
    "merkle_level",
    "byte_leaves",
    "tree_nodes",
];

const FAIL_ON_ARCHIVE_MISS: NSUInteger = 1 << 2;

/// Where the binary-archive cache lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShaderCache {
    /// No archive: compile every process (the OS shader cache still applies).
    Off,
    /// Archives in this directory (created on demand).
    Dir(PathBuf),
}

impl ShaderCache {
    /// `$LENS_SHADER_CACHE` (`off`, `0` or empty disable it; anything else
    /// is the directory), else `~/Library/Caches/cyber-lens`.
    pub fn from_env() -> Self {
        match std::env::var("LENS_SHADER_CACHE") {
            Ok(v) if v.is_empty() || v == "off" || v == "0" => ShaderCache::Off,
            Ok(v) => ShaderCache::Dir(PathBuf::from(v)),
            Err(_) => match std::env::var_os("HOME") {
                Some(h) => ShaderCache::Dir(PathBuf::from(h).join("Library/Caches/cyber-lens")),
                None => ShaderCache::Off,
            },
        }
    }
}

/// What the binary archive did for this process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArchiveUse {
    /// No cache directory.
    Off,
    /// Every pipeline came from the archive.
    Hit,
    /// No usable archive (absent, rejected, or some pipelines missed); the
    /// pipelines compiled and a new archive was written.
    Written,
    /// As `Written`, but writing failed (the reason); nothing changed on disk.
    WriteFailed(String),
}

/// How [`build`] went, for diagnostics and `examples/metal_cold`.
#[derive(Clone, Debug)]
pub struct Startup {
    /// `"metallib"` (embedded, offline-compiled) or `"source"`.
    pub library_from: &'static str,
    /// Library creation (MSL front end, or metallib load).
    pub library: Duration,
    /// Opening the archive file (zero when absent or off).
    pub archive_load: Duration,
    /// All twelve pipelines.
    pub pipelines: Duration,
    /// Pipelines that came from the archive.
    pub archive_hits: usize,
    /// Building and writing a new archive (zero on a hit).
    pub archive_save: Duration,
    pub archive: ArchiveUse,
    /// The archive file (whether or not it existed).
    pub archive_path: Option<PathBuf>,
}

/// An owned Objective-C object (released on drop).
pub(crate) struct Obj(pub(crate) ObjcId);

impl Drop for Obj {
    fn drop(&mut self) {
        // SAFETY: we hold one reference (a `new…`/`alloc` result or retained).
        unsafe { release(self.0) }
    }
}

// SAFETY: Metal pipeline states, functions, libraries, devices and archives
// are documented thread-safe; descriptors are only touched by one thread.
unsafe impl Send for Obj {}
// SAFETY: as above.
unsafe impl Sync for Obj {}

/// A compute pipeline state.
pub(crate) struct Pipe(Obj);

impl Pipe {
    pub(crate) fn raw(&self) -> ObjcId {
        self.0.0
    }
    /// Set this pipeline on a compute encoder.
    pub(crate) fn bind(&self, enc: &aruminium::Encoder) {
        // SAFETY: a live compute encoder and pipeline state.
        unsafe {
            aruminium::ffi::msg1_void(
                enc.as_raw(),
                aruminium::ffi::SEL_setComputePipelineState(),
                self.raw(),
            )
        }
    }
    pub(crate) fn thread_execution_width(&self) -> usize {
        // SAFETY: a live MTLComputePipelineState.
        unsafe { send_usize(self.raw(), sel(c"threadExecutionWidth")) }
    }
    pub(crate) fn max_total_threads_per_threadgroup(&self) -> usize {
        // SAFETY: as above.
        unsafe { send_usize(self.raw(), sel(c"maxTotalThreadsPerThreadgroup")) }
    }
}

fn sel(name: &CStr) -> ObjcSel {
    // SAFETY: a NUL-terminated selector name.
    unsafe { sel_registerName(name.as_ptr()) }
}

fn class(name: &CStr) -> ObjcId {
    // SAFETY: a NUL-terminated class name; nil if absent.
    unsafe { objc_getClass(name.as_ptr()) as ObjcId }
}

unsafe fn send0(o: ObjcId, s: ObjcSel) -> ObjcId {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel) -> ObjcId;
    // SAFETY: objc_msgSend called with the selector's true signature.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s) }
}

unsafe fn send_usize(o: ObjcId, s: ObjcSel) -> usize {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel) -> usize;
    // SAFETY: as in `send0`.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s) }
}

unsafe fn send1(o: ObjcId, s: ObjcSel, a: ObjcId) -> ObjcId {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel, ObjcId) -> ObjcId;
    // SAFETY: as in `send0`.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s, a) }
}

/// `-(id/BOOL)x:(id)a error:(NSError**)e` — returns the raw result word.
unsafe fn send1_err(o: ObjcId, s: ObjcSel, a: ObjcId, err: *mut ObjcId) -> ObjcId {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel, ObjcId, *mut ObjcId) -> ObjcId;
    // SAFETY: as in `send0`.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s, a, err) }
}

unsafe fn send1_bool_err(o: ObjcId, s: ObjcSel, a: ObjcId, err: *mut ObjcId) -> bool {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel, ObjcId, *mut ObjcId) -> bool;
    // SAFETY: as in `send0`.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s, a, err) }
}

fn error_text(err: ObjcId) -> String {
    nserror_string(err).unwrap_or_else(|| "unknown error".into())
}

/// An `NSString`, released on drop.
struct NsStr(ObjcId);
impl NsStr {
    fn new(s: &str) -> Self {
        NsStr(nsstring(s))
    }
}
impl Drop for NsStr {
    fn drop(&mut self) {
        // SAFETY: created by CFStringCreateWithBytes (+1).
        unsafe { CFRelease(self.0 as CFTypeRef) }
    }
}

/// An autoreleased `NSURL` for a file path (call inside a pool).
fn file_url(path: &Path) -> Option<ObjcId> {
    let p = NsStr::new(path.to_str()?);
    // SAFETY: +[NSURL fileURLWithPath:] with an NSString.
    let u = unsafe { send1(class(c"NSURL"), sel(c"fileURLWithPath:"), p.0) };
    (!u.is_null()).then_some(u)
}

unsafe extern "C" {
    fn dispatch_data_create(
        buffer: *const c_void,
        size: usize,
        queue: *mut c_void,
        destructor: *const c_void,
    ) -> ObjcId;
}

/// The OS version string (`NSProcessInfo`), part of the archive key.
fn os_version() -> String {
    aruminium::autorelease_pool(|| {
        // SAFETY: +[NSProcessInfo processInfo], -operatingSystemVersionString.
        unsafe {
            let pi = send0(class(c"NSProcessInfo"), sel(c"processInfo"));
            if pi.is_null() {
                return String::new();
            }
            nsstring_to_rust(send0(pi, sel(c"operatingSystemVersionString"))).unwrap_or_default()
        }
    })
}

/// The archive file for `source` on `gpu`: keyed by the source, the device,
/// the OS build and this crate's version.
pub(crate) fn archive_path(dir: &Path, source: &str, device: &str) -> PathBuf {
    let mut key = Vec::new();
    for part in [
        source,
        device,
        &os_version(),
        env!("CARGO_PKG_VERSION"),
        "lens-metal-archive-v1",
    ] {
        key.extend_from_slice(&(part.len() as u64).to_le_bytes());
        key.extend_from_slice(part.as_bytes());
    }
    let h = cyber_hemera::hash(&key).to_hex();
    dir.join(format!("metal-{}.binarchive", &h[..32]))
}

/// The MTLLibrary: the embedded metallib if it was built from exactly
/// `source`, else the source compiled.
fn library(gpu: &Gpu, source: &str) -> Result<(Obj, &'static str), String> {
    if let Some(lib) = embedded_library(gpu, source) {
        return Ok((lib, "metallib"));
    }
    let lib = gpu
        .compile(source)
        .map_err(|e| format!("metal compile: {e:?}"))?;
    // keep our own reference; `lib` releases its own on drop
    // SAFETY: a live MTLLibrary.
    let raw = unsafe { aruminium::ffi::retain(lib.as_raw()) };
    Ok((Obj(raw), "source"))
}

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/metallib.rs"));
}

fn embedded_library(gpu: &Gpu, source: &str) -> Option<Obj> {
    let (bytes, hash) = embedded::METALLIB?;
    if cyber_hemera::hash(source.as_bytes()).to_hex() != hash {
        return None;
    }
    aruminium::autorelease_pool(|| {
        // SAFETY: dispatch_data_create copies `bytes` (default destructor);
        // newLibraryWithData:error: takes the dispatch_data_t.
        unsafe {
            let data = dispatch_data_create(
                bytes.as_ptr().cast(),
                bytes.len(),
                core::ptr::null_mut(),
                core::ptr::null(),
            );
            if data.is_null() {
                return None;
            }
            let mut err: ObjcId = core::ptr::null_mut();
            let lib = send1_err(
                gpu.as_raw(),
                sel(c"newLibraryWithData:error:"),
                data,
                &mut err,
            );
            release(data);
            (!lib.is_null()).then_some(Obj(lib))
        }
    })
}

fn function(lib: &Obj, name: &str) -> Result<Obj, String> {
    let n = NsStr::new(name);
    // SAFETY: -[MTLLibrary newFunctionWithName:] (+1 or nil).
    let f = unsafe { send1(lib.0, sel(c"newFunctionWithName:"), n.0) };
    if f.is_null() {
        return Err(format!("metal {name}: function not found"));
    }
    Ok(Obj(f))
}

/// A compute pipeline descriptor for `f`, with `archive` attached if any.
fn descriptor(f: &Obj, archive: Option<&Obj>) -> Option<Obj> {
    // SAFETY: +new, -setComputeFunction:, -setBinaryArchives: with an
    // autoreleased NSArray (the caller holds a pool).
    unsafe {
        let d = send0(class(c"MTLComputePipelineDescriptor"), sel(c"new"));
        if d.is_null() {
            return None;
        }
        let d = Obj(d);
        send1(d.0, sel(c"setComputeFunction:"), f.0);
        if let Some(a) = archive {
            let arr = send1(class(c"NSArray"), sel(c"arrayWithObject:"), a.0);
            if arr.is_null() {
                return None;
            }
            send1(d.0, sel(c"setBinaryArchives:"), arr);
        }
        Some(d)
    }
}

fn pipeline_from(gpu: Dev, d: &Obj, options: NSUInteger) -> Result<Obj, String> {
    type F = unsafe extern "C" fn(
        ObjcId,
        ObjcSel,
        ObjcId,
        NSUInteger,
        *mut ObjcId,
        *mut ObjcId,
    ) -> ObjcId;
    let mut err: ObjcId = core::ptr::null_mut();
    // SAFETY: -newComputePipelineStateWithDescriptor:options:reflection:error:
    // with a nil reflection out-pointer.
    let p = unsafe {
        let f: F = core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void);
        f(
            gpu.0,
            sel(c"newComputePipelineStateWithDescriptor:options:reflection:error:"),
            d.0,
            options,
            core::ptr::null_mut(),
            &mut err,
        )
    };
    if p.is_null() {
        return Err(error_text(err));
    }
    Ok(Obj(p))
}

/// The device, for the pipeline threads.
#[derive(Clone, Copy)]
struct Dev(ObjcId);
// SAFETY: MTLDevice is thread-safe; `build` keeps the `Gpu` alive.
unsafe impl Send for Dev {}
// SAFETY: as above.
unsafe impl Sync for Dev {}

/// One pipeline: from the archive if it has it, else compiled. Returns
/// whether it was an archive hit.
fn make_pipe(gpu: Dev, f: &Obj, archive: Option<&Obj>, name: &str) -> Result<(Pipe, bool), String> {
    aruminium::autorelease_pool(|| {
        if let Some(a) = archive
            && let Some(d) = descriptor(f, Some(a))
            && let Ok(p) = pipeline_from(gpu, &d, FAIL_ON_ARCHIVE_MISS)
        {
            return Ok((Pipe(p), true));
        }
        let d = descriptor(f, None).ok_or_else(|| format!("metal pipeline {name}: descriptor"))?;
        let p = pipeline_from(gpu, &d, 0).map_err(|e| format!("metal pipeline {name}: {e}"))?;
        Ok((Pipe(p), false))
    })
}

/// Open the archive at `path` (an error if the file is absent or Metal
/// rejects it).
fn load_archive(gpu: &Gpu, path: &Path) -> Result<Obj, String> {
    if !path.is_file() {
        return Err("absent".into());
    }
    new_archive(gpu, Some(path))
}

fn new_archive(gpu: &Gpu, path: Option<&Path>) -> Result<Obj, String> {
    aruminium::autorelease_pool(|| {
        // SAFETY: +new descriptor, -setUrl:, -newBinaryArchiveWithDescriptor:error:.
        unsafe {
            let d = send0(class(c"MTLBinaryArchiveDescriptor"), sel(c"new"));
            if d.is_null() {
                return Err("no MTLBinaryArchiveDescriptor".to_string());
            }
            let d = Obj(d);
            if let Some(p) = path {
                let u = file_url(p).ok_or("archive path")?;
                send1(d.0, sel(c"setUrl:"), u);
            }
            let mut err: ObjcId = core::ptr::null_mut();
            let a = send1_err(
                gpu.as_raw(),
                sel(c"newBinaryArchiveWithDescriptor:error:"),
                d.0,
                &mut err,
            );
            if a.is_null() {
                return Err(error_text(err));
            }
            Ok(Obj(a))
        }
    })
}

/// Write a new archive holding every function's pipeline to `path`,
/// atomically (temp file in the same directory, then rename).
fn save_archive(gpu: &Gpu, funcs: &[Obj], path: &Path) -> Result<(), String> {
    let dir = path.parent().ok_or("archive dir")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let archive = new_archive(gpu, None)?;
    for f in funcs {
        aruminium::autorelease_pool(|| {
            let d = descriptor(f, None).ok_or("descriptor")?;
            let mut err: ObjcId = core::ptr::null_mut();
            // SAFETY: -addComputePipelineFunctionsWithDescriptor:error:.
            let ok = unsafe {
                send1_bool_err(
                    archive.0,
                    sel(c"addComputePipelineFunctionsWithDescriptor:error:"),
                    d.0,
                    &mut err,
                )
            };
            if ok { Ok(()) } else { Err(error_text(err)) }
        })?;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let tmp = dir.join(format!(
        ".{}.{}.{nanos}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("archive"),
        std::process::id()
    ));
    let written = aruminium::autorelease_pool(|| {
        let u = file_url(&tmp).ok_or("temp path")?;
        let mut err: ObjcId = core::ptr::null_mut();
        // SAFETY: -serializeToURL:error:.
        let ok = unsafe { send1_bool_err(archive.0, sel(c"serializeToURL:error:"), u, &mut err) };
        if ok { Ok(()) } else { Err(error_text(err)) }
    });
    let r = written.and_then(|()| std::fs::rename(&tmp, path).map_err(|e| e.to_string()));
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    if r.is_ok() {
        prune(dir, path);
    }
    r
}

/// Archives kept per directory (one per source × device × OS × version;
/// older ones are from superseded kernels or OS builds).
const KEEP: usize = 8;

/// Drop all but the [`KEEP`] most recent archives, and temp files an hour
/// old (left by a process that died mid-write). Best effort.
fn prune(dir: &Path, keep: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let now = std::time::SystemTime::now();
    let mut archives = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name();
        let Some(name) = name.to_str() else { continue };
        let modified = e.metadata().and_then(|m| m.modified()).unwrap_or(now);
        if name.starts_with(".metal-") && name.ends_with(".tmp") {
            if now
                .duration_since(modified)
                .is_ok_and(|a| a.as_secs() > 3600)
            {
                let _ = std::fs::remove_file(&p);
            }
        } else if name.starts_with("metal-") && name.ends_with(".binarchive") && p != keep {
            archives.push((modified, p));
        }
    }
    archives.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (_, p) in archives.into_iter().skip(KEEP - 1) {
        let _ = std::fs::remove_file(p);
    }
}

/// Build the library and the [`KERNELS`] pipelines (in order).
///
pub(crate) fn build(
    gpu: &Gpu,
    source: &str,
    cache: &ShaderCache,
    concurrent: bool,
) -> Result<(Vec<Pipe>, Startup), String> {
    let t = Instant::now();
    let (lib, library_from) = library(gpu, source)?;
    let library_time = t.elapsed();
    let funcs = KERNELS
        .iter()
        .map(|n| function(&lib, n))
        .collect::<Result<Vec<_>, _>>()?;

    let path = match cache {
        ShaderCache::Off => None,
        ShaderCache::Dir(d) => Some(archive_path(d, source, &gpu.name())),
    };
    let t = Instant::now();
    let archive = path.as_deref().and_then(|p| load_archive(gpu, p).ok());
    let archive_load = t.elapsed();

    let dev = Dev(gpu.as_raw());
    let t = Instant::now();
    let made: Vec<Result<(Pipe, bool), String>> = if concurrent {
        std::thread::scope(|s| {
            let hs: Vec<_> = funcs
                .iter()
                .zip(KERNELS)
                .map(|(f, n)| {
                    let a = archive.as_ref();
                    s.spawn(move || make_pipe(dev, f, a, n))
                })
                .collect();
            hs.into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|_| Err("metal pipeline: thread panicked".into()))
                })
                .collect()
        })
    } else {
        funcs
            .iter()
            .zip(KERNELS)
            .map(|(f, n)| make_pipe(dev, f, archive.as_ref(), n))
            .collect()
    };
    let pipelines = t.elapsed();
    let mut pipes = Vec::with_capacity(KERNELS.len());
    let mut hits = 0;
    for m in made {
        let (p, hit) = m?;
        hits += usize::from(hit);
        pipes.push(p);
    }

    let t = Instant::now();
    let archive_use = match &path {
        None => ArchiveUse::Off,
        Some(_) if hits == KERNELS.len() => ArchiveUse::Hit,
        Some(p) => match save_archive(gpu, &funcs, p) {
            Ok(()) => ArchiveUse::Written,
            Err(e) => ArchiveUse::WriteFailed(e),
        },
    };
    let archive_save = if archive_use == ArchiveUse::Hit {
        Duration::ZERO
    } else {
        t.elapsed()
    };
    Ok((
        pipes,
        Startup {
            library_from,
            library: library_time,
            archive_load,
            pipelines,
            archive_hits: hits,
            archive_save,
            archive: archive_use,
            archive_path: path,
        },
    ))
}
