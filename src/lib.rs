//! Procedural Layout Engine (PLE).
//!
//! Context JSON -> semantic profile -> OKLCH/APCA palette -> BSP spatial grammar ->
//! constraint solver + candidate search -> typed IR -> semantic HTML with Tailwind classes.
//!
//! Zero dependencies. Library (`rlib`), C ABI / WASM (`cdylib`) and CLI (`ple`) share this crate.

pub mod codegen;
pub mod color;
pub mod compose;
pub mod context;
pub mod engine;
pub mod ir;
pub mod json;
pub mod layout;
pub mod text;

pub use engine::{compile, Compiled, EmitMode, EngineConfig, EngineError, ENGINE_VERSION};

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// JSON-in / JSON-out entry point shared by the C ABI and WASM exports.
/// Returns `{"ok":true,"html":...,"report":...}` or `{"ok":false,"error":...}`.
pub fn compile_to_json(context_json: &str, options_json: Option<&str>) -> String {
    let mut cfg = EngineConfig::default();
    if let Some(o) = options_json {
        if !o.trim().is_empty() {
            match json::parse(o) {
                Ok(v) => {
                    if let Err(e) = cfg.apply_json(&v) {
                        return error_json(&e);
                    }
                }
                Err(e) => return error_json(&e.to_string()),
            }
        }
    }
    match compile(context_json, &cfg) {
        Ok(c) => json::obj(vec![
            ("ok", json::boolean(true)),
            ("html", json::string(c.html)),
            ("report", c.report),
        ])
        .to_json(),
        Err(e) => error_json(&e.to_string()),
    }
}

fn error_json(msg: &str) -> String {
    json::obj(vec![("ok", json::boolean(false)), ("error", json::string(msg))]).to_json()
}

fn to_c(s: String) -> *mut c_char {
    CString::new(s.replace('\0', "")).unwrap_or_default().into_raw()
}

/// C ABI. Both arguments are NUL-terminated UTF-8; `options_json` may be NULL.
/// The returned string must be released with `ple_free`.
///
/// # Safety
/// `context_json` must be a valid NUL-terminated string; `options_json` must be NULL or valid.
#[no_mangle]
pub unsafe extern "C" fn ple_compile(context_json: *const c_char, options_json: *const c_char) -> *mut c_char {
    if context_json.is_null() {
        return to_c(error_json("context_json is NULL"));
    }
    let ctx = CStr::from_ptr(context_json).to_string_lossy().into_owned();
    let opts = if options_json.is_null() {
        None
    } else {
        Some(CStr::from_ptr(options_json).to_string_lossy().into_owned())
    };
    to_c(compile_to_json(&ctx, opts.as_deref()))
}

/// Releases a string returned by `ple_compile`.
///
/// # Safety
/// `ptr` must come from `ple_compile` and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn ple_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Allocates `size` bytes in the module's linear memory (for WASM hosts that must write the input).
#[no_mangle]
pub extern "C" fn ple_alloc(size: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(size.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Frees memory obtained from `ple_alloc`.
///
/// # Safety
/// `ptr` and `size` must match a previous `ple_alloc` call.
#[no_mangle]
pub unsafe extern "C" fn ple_dealloc(ptr: *mut u8, size: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, 0, size.max(1)));
    }
}

/// NUL-terminated version string (static; do not free).
#[no_mangle]
pub extern "C" fn ple_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}
