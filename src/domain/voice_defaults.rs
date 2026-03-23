//! Default constants for voice recording configuration.

/// Default recording command for macOS (AVFoundation).
#[cfg(target_os = "macos")]
pub const DEFAULT_RECORD_CMD: &str =
    "ffmpeg -f avfoundation -i ':0' -c:a libopus -b:a 32k {file_path}";

/// Default recording command for Linux (ALSA).
#[cfg(target_os = "linux")]
pub const DEFAULT_RECORD_CMD: &str = "ffmpeg -f alsa -i hw:0 -c:a libopus -b:a 32k {file_path}";

/// Fallback for other platforms.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub const DEFAULT_RECORD_CMD: &str =
    "ffmpeg -f avfoundation -i ':0' -c:a libopus -b:a 32k {file_path}";
