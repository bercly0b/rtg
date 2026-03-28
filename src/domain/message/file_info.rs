/// Download state of a media file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadStatus {
    /// File has not been downloaded and no download is in progress.
    #[default]
    NotStarted,
    /// File is currently being downloaded.
    Downloading { progress_percent: u8 },
    /// File has been fully downloaded.
    Completed,
}

/// File metadata for messages with downloadable media.
///
/// Provides the information needed to open/play a file: its local path
/// (if already downloaded) and MIME type (for handler lookup), plus
/// additional metadata for display (size, duration, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// TDLib file identifier, used for download requests.
    pub file_id: i32,
    /// Local filesystem path; `None` if the file has not been downloaded yet.
    pub local_path: Option<String>,
    /// MIME type reported by TDLib (e.g. `"audio/ogg"`, `"video/mp4"`).
    pub mime_type: String,
    /// File size in bytes (from TDLib `File.size` or `File.expected_size`).
    pub size: Option<u64>,
    /// Duration in seconds (for voice, audio, video, video note, animation).
    pub duration: Option<i32>,
    /// Original file name (for documents and audio).
    pub file_name: Option<String>,
    /// Whether a voice/video note has been listened/viewed.
    pub is_listened: bool,
    /// Current download state.
    pub download_status: DownloadStatus,
}

/// Formats a file size in bytes into a human-readable string.
///
/// Uses base-10 units (KB = 1000, MB = 1000000) to match the TG client convention.
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1_000;
    const MB: u64 = 1_000_000;
    const GB: u64 = 1_000_000_000;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Formats a duration in seconds into `M:SS` or `H:MM:SS`.
pub fn format_duration(seconds: i32) -> String {
    let seconds = seconds.max(0);
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;

    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

/// Builds a metadata display string for a file-bearing message.
///
/// Returns a formatted string like `"download=yes, size=15.5KB, duration=0:03, listened=yes"`
/// for rendering alongside the `[Media]` label.
pub fn build_file_metadata_display(media: super::MessageMedia, info: &FileInfo) -> String {
    let mut parts = Vec::new();

    match info.download_status {
        DownloadStatus::Completed => parts.push("download=yes".to_owned()),
        DownloadStatus::Downloading { progress_percent } => {
            parts.push(format!("downloading={}%", progress_percent));
        }
        DownloadStatus::NotStarted => parts.push("download=no".to_owned()),
    }

    if let Some(size) = info.size {
        parts.push(format!("size={}", format_file_size(size)));
    }

    if let Some(dur) = info.duration {
        match media {
            super::MessageMedia::Voice
            | super::MessageMedia::Audio
            | super::MessageMedia::Video
            | super::MessageMedia::VideoNote
            | super::MessageMedia::Animation => {
                parts.push(format!("duration={}", format_duration(dur)));
            }
            _ => {}
        }
    }

    if matches!(
        media,
        super::MessageMedia::Voice | super::MessageMedia::VideoNote
    ) && info.is_listened
    {
        parts.push("listened=yes".to_owned());
    }

    parts.join(", ")
}
