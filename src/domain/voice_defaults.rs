//! Default constants for voice recording configuration.

/// Recording command for macOS (AVFoundation).
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "used outside tests only on its own platform")
)]
pub const MACOS_RECORD_CMD: &str =
    "ffmpeg -f avfoundation -i ':0' -c:a libopus -b:a 32k {file_path}";

/// Recording command for Linux (ALSA).
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "used outside tests only on its own platform")
)]
pub const LINUX_RECORD_CMD: &str = "ffmpeg -f alsa -i hw:0 -c:a libopus -b:a 32k {file_path}";

/// Default recording command for macOS (AVFoundation).
#[cfg(target_os = "macos")]
pub const DEFAULT_RECORD_CMD: &str = MACOS_RECORD_CMD;

/// Default recording command for Linux (ALSA).
#[cfg(target_os = "linux")]
pub const DEFAULT_RECORD_CMD: &str = LINUX_RECORD_CMD;

/// Fallback for other platforms.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub const DEFAULT_RECORD_CMD: &str = MACOS_RECORD_CMD;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::command_template::resolve_argv;

    #[test]
    fn macos_record_cmd_passes_unquoted_device_to_ffmpeg() {
        let argv = resolve_argv(MACOS_RECORD_CMD, "/tmp/voice.oga").unwrap();

        assert_eq!(
            argv,
            [
                "ffmpeg",
                "-f",
                "avfoundation",
                "-i",
                ":0",
                "-c:a",
                "libopus",
                "-b:a",
                "32k",
                "/tmp/voice.oga"
            ]
        );
    }

    #[test]
    fn linux_record_cmd_resolves_to_expected_argv() {
        let argv = resolve_argv(LINUX_RECORD_CMD, "/tmp/voice.oga").unwrap();

        assert_eq!(
            argv,
            [
                "ffmpeg",
                "-f",
                "alsa",
                "-i",
                "hw:0",
                "-c:a",
                "libopus",
                "-b:a",
                "32k",
                "/tmp/voice.oga"
            ]
        );
    }
}
