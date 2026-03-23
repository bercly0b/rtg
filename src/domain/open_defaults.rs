//! Default constants for message opening configuration.

/// Default command to open files on macOS (delegates to Launch Services).
#[cfg(target_os = "macos")]
pub const DEFAULT_OPEN: &str = "open {file_path}";

/// Default command to open files on Linux (XDG desktop integration).
#[cfg(target_os = "linux")]
pub const DEFAULT_OPEN: &str = "xdg-open {file_path}";

/// Fallback for other platforms.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub const DEFAULT_OPEN: &str = "open {file_path}";
