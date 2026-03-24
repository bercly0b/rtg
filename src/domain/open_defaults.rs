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

/// Returns `true` when the resolved command is the platform default opener
/// (`open` on macOS, `xdg-open` on Linux). These commands launch an external
/// application and exit immediately, so showing a command popup is pointless.
pub fn is_platform_default(cmd_template: &str) -> bool {
    cmd_template == DEFAULT_OPEN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_default_matches_default_open() {
        assert!(is_platform_default(DEFAULT_OPEN));
    }

    #[test]
    fn custom_command_is_not_platform_default() {
        assert!(!is_platform_default("mpv {file_path}"));
    }

    #[test]
    fn empty_string_is_not_platform_default() {
        assert!(!is_platform_default(""));
    }
}
