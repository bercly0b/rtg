//! Default constants for message opening configuration.

use super::command_template;

/// Command to open files on macOS (delegates to Launch Services).
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "used outside tests only on its own platform")
)]
pub const MACOS_OPEN: &str = "open {file_path}";

/// Command to open files on Linux (XDG desktop integration).
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "used outside tests only on its own platform")
)]
pub const LINUX_OPEN: &str = "xdg-open {file_path}";

/// Default command to open files on macOS (delegates to Launch Services).
#[cfg(target_os = "macos")]
pub const DEFAULT_OPEN: &str = MACOS_OPEN;

/// Default command to open files on Linux (XDG desktop integration).
#[cfg(target_os = "linux")]
pub const DEFAULT_OPEN: &str = LINUX_OPEN;

/// Fallback for other platforms.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub const DEFAULT_OPEN: &str = MACOS_OPEN;

/// Returns `true` when the command runs the platform default opener
/// (`open` on macOS, `xdg-open` on Linux). These commands launch an external
/// application and exit immediately, so showing a command popup is pointless.
///
/// Only the executable is compared, so extra whitespace, quoting or opener
/// flags (e.g. `open -a Preview {file_path}`) still count as the default.
pub fn is_platform_default(cmd_template: &str) -> bool {
    command_template::program(cmd_template)
        .is_some_and(|program| command_template::program(DEFAULT_OPEN) == Some(program))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::command_template::resolve_argv;

    #[test]
    fn platform_default_matches_default_open() {
        assert!(is_platform_default(DEFAULT_OPEN));
    }

    #[test]
    fn platform_default_ignores_whitespace_and_quoting() {
        let program = command_template::program(DEFAULT_OPEN).unwrap();

        assert!(is_platform_default(&format!(
            "  {program}   {{file_path}} "
        )));
        assert!(is_platform_default(&format!(
            "'{program}' \"{{file_path}}\""
        )));
    }

    #[test]
    fn platform_default_allows_opener_flags() {
        let program = command_template::program(DEFAULT_OPEN).unwrap();

        assert!(is_platform_default(&format!(
            "{program} -a Preview {{file_path}}"
        )));
    }

    #[test]
    fn custom_command_is_not_platform_default() {
        assert!(!is_platform_default("mpv {file_path}"));
    }

    #[test]
    fn command_with_opener_as_argument_is_not_platform_default() {
        let program = command_template::program(DEFAULT_OPEN).unwrap();

        assert!(!is_platform_default(&format!(
            "env {program} {{file_path}}"
        )));
    }

    #[test]
    fn empty_string_is_not_platform_default() {
        assert!(!is_platform_default(""));
    }

    #[test]
    fn macos_open_resolves_to_expected_argv() {
        let argv = resolve_argv(MACOS_OPEN, "/tmp/my photo.jpg").unwrap();

        assert_eq!(argv, ["open", "/tmp/my photo.jpg"]);
    }

    #[test]
    fn linux_open_resolves_to_expected_argv() {
        let argv = resolve_argv(LINUX_OPEN, "/tmp/my photo.jpg").unwrap();

        assert_eq!(argv, ["xdg-open", "/tmp/my photo.jpg"]);
    }
}
