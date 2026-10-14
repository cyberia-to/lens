//! Objective-C plumbing for the Metal shader cache (`metal_cache`): owned
//! objects, the compute pipeline handle, typed `objc_msgSend` shims and
//! Foundation strings/URLs — the calls aruminium does not wrap.

use std::ffi::{CStr, c_void};
use std::path::Path;

use aruminium::ffi::{
    CFRelease, CFTypeRef, ObjcId, ObjcSel, nserror_string, nsstring, objc_getClass, objc_msgSend,
    release, sel_registerName,
};

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
pub(crate) struct Pipe(pub(crate) Obj);

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

pub(crate) fn sel(name: &CStr) -> ObjcSel {
    // SAFETY: a NUL-terminated selector name.
    unsafe { sel_registerName(name.as_ptr()) }
}

pub(crate) fn class(name: &CStr) -> ObjcId {
    // SAFETY: a NUL-terminated class name; nil if absent.
    unsafe { objc_getClass(name.as_ptr()) as ObjcId }
}

pub(crate) unsafe fn send0(o: ObjcId, s: ObjcSel) -> ObjcId {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel) -> ObjcId;
    // SAFETY: objc_msgSend called with the selector's true signature.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s) }
}

pub(crate) unsafe fn send_usize(o: ObjcId, s: ObjcSel) -> usize {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel) -> usize;
    // SAFETY: as in `send0`.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s) }
}

pub(crate) unsafe fn send1(o: ObjcId, s: ObjcSel, a: ObjcId) -> ObjcId {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel, ObjcId) -> ObjcId;
    // SAFETY: as in `send0`.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s, a) }
}

/// `-(id/BOOL)x:(id)a error:(NSError**)e` — returns the raw result word.
pub(crate) unsafe fn send1_err(o: ObjcId, s: ObjcSel, a: ObjcId, err: *mut ObjcId) -> ObjcId {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel, ObjcId, *mut ObjcId) -> ObjcId;
    // SAFETY: as in `send0`.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s, a, err) }
}

pub(crate) unsafe fn send1_bool_err(o: ObjcId, s: ObjcSel, a: ObjcId, err: *mut ObjcId) -> bool {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel, ObjcId, *mut ObjcId) -> bool;
    // SAFETY: as in `send0`.
    unsafe { core::mem::transmute::<*const c_void, F>(objc_msgSend as *const c_void)(o, s, a, err) }
}

pub(crate) fn error_text(err: ObjcId) -> String {
    nserror_string(err).unwrap_or_else(|| "unknown error".into())
}

/// An `NSString`, released on drop.
pub(crate) struct NsStr(pub(crate) ObjcId);
impl NsStr {
    pub(crate) fn new(s: &str) -> Self {
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
pub(crate) fn file_url(path: &Path) -> Option<ObjcId> {
    let p = NsStr::new(path.to_str()?);
    // SAFETY: +[NSURL fileURLWithPath:] with an NSString.
    let u = unsafe { send1(class(c"NSURL"), sel(c"fileURLWithPath:"), p.0) };
    (!u.is_null()).then_some(u)
}
