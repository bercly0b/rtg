//! Synchronous TDLib log configuration that runs *before* any client exists.
//!
//! TDLib's C++ logger writes to stderr at verbosity 5 by default the moment
//! `td_create_client_id` is called. The async `set_log_verbosity_level`
//! exposed by `tdlib_rs` requires a `client_id`, so it cannot suppress the
//! initial burst. We bind `td_execute` directly — the canonical synchronous,
//! client-less entry point — to set global verbosity ahead of client init.

use std::ffi::{c_char, CString};

#[link(name = "tdjson")]
unsafe extern "C" {
    fn td_execute(request: *const c_char) -> *const c_char;
}

/// Sets the global TDLib log verbosity. Must be called before `create_client`
/// to suppress (or enable) TDLib's startup logging to stderr.
///
/// Levels: 0 = fatal only, 1 = errors, 2 = warnings, 3 = info, 5 = verbose.
pub(super) fn set_global_verbosity(level: u8) {
    let request = format!(r#"{{"@type":"setLogVerbosityLevel","new_verbosity_level":{level}}}"#);
    let Ok(cstring) = CString::new(request) else {
        return;
    };
    unsafe {
        td_execute(cstring.as_ptr());
    }
}
