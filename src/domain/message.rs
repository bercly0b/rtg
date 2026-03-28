/// Type of media attached to a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageMedia {
    #[default]
    None,
    Photo,
    Voice,
    Video,
    VideoNote,
    Sticker,
    Document,
    Audio,
    Animation,
    Contact,
    Location,
    Poll,
    Call,
    VideoCall,
    Other,
}

impl MessageMedia {
    /// Returns a display label for the media type, or None if no media.
    pub fn display_label(&self) -> Option<&'static str> {
        match self {
            MessageMedia::None => None,
            MessageMedia::Photo => Some("[Photo]"),
            MessageMedia::Voice => Some("[Voice]"),
            MessageMedia::Video => Some("[Video]"),
            MessageMedia::VideoNote => Some("[Video message]"),
            MessageMedia::Sticker => Some("[Sticker]"),
            MessageMedia::Document => Some("[Document]"),
            MessageMedia::Audio => Some("[Audio]"),
            MessageMedia::Animation => Some("[GIF]"),
            MessageMedia::Contact => Some("[Contact]"),
            MessageMedia::Location => Some("[Location]"),
            MessageMedia::Poll => Some("[Poll]"),
            MessageMedia::Call => Some("[Call]"),
            MessageMedia::VideoCall => Some("[Video call]"),
            MessageMedia::Other => Some("[Media]"),
        }
    }
}

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

/// Reason why a call was ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDiscardReason {
    /// Normal hang-up or unknown reason.
    HungUp,
    /// The call was missed (incoming) or cancelled (outgoing).
    Missed,
    /// The other party declined the call.
    Declined,
    /// The users were disconnected during the call.
    Disconnected,
}

/// Metadata for a call message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallInfo {
    /// Whether this was a video call.
    pub is_video: bool,
    /// Call duration in seconds (0 if the call didn't connect).
    pub duration: i32,
    /// Why the call ended.
    pub discard_reason: CallDiscardReason,
}

/// Builds a display string for call metadata.
///
/// Uses `is_outgoing` from the message to determine direction.
/// Examples: `"Outgoing, 1:23"`, `"Missed"`, `"Declined"`, `"Cancelled"`.
pub fn build_call_metadata_display(info: &CallInfo, is_outgoing: bool) -> String {
    match info.discard_reason {
        CallDiscardReason::Missed => {
            if is_outgoing {
                "Cancelled".to_owned()
            } else {
                "Missed".to_owned()
            }
        }
        CallDiscardReason::Declined => "Declined".to_owned(),
        CallDiscardReason::Disconnected => {
            if info.duration > 0 {
                format!("Disconnected, {}", format_duration(info.duration))
            } else {
                "Disconnected".to_owned()
            }
        }
        CallDiscardReason::HungUp => {
            if info.duration > 0 {
                let direction = if is_outgoing { "Outgoing" } else { "Incoming" };
                format!("{direction}, {}", format_duration(info.duration))
            } else if is_outgoing {
                "Cancelled".to_owned()
            } else {
                "Missed".to_owned()
            }
        }
    }
}

/// Information about the message being replied to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyInfo {
    /// Display name of the original message sender.
    pub sender_name: String,
    /// Text preview of the original message.
    pub text: String,
}

/// A hyperlink embedded in message text via a text entity.
///
/// Represents both `TextEntityTypeUrl` (URL visible in text) and
/// `TextEntityTypeTextUrl` (clickable text with a hidden URL).
/// Offsets are **byte** offsets into `Message::text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLink {
    /// Byte offset of the link text start in `Message::text`.
    pub offset: usize,
    /// Byte length of the link text in `Message::text`.
    pub length: usize,
    /// The target URL to open.
    pub url: String,
}

/// Delivery status of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageStatus {
    /// Message has been delivered (normal state).
    #[default]
    Delivered,
    /// Message is being sent (optimistic display).
    Sending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: i64,
    pub sender_name: String,
    pub text: String,
    pub timestamp_ms: i64,
    pub is_outgoing: bool,
    pub media: MessageMedia,
    pub status: MessageStatus,
    /// File metadata for messages that carry downloadable media.
    /// `None` for text-only, poll, contact, location, and other non-file types.
    pub file_info: Option<FileInfo>,
    /// Call metadata for `MessageMedia::Call` messages.
    pub call_info: Option<CallInfo>,
    /// Information about the message this message replies to.
    /// `None` if the message is not a reply.
    pub reply_to: Option<ReplyInfo>,
    /// Total number of reactions on this message (summed across all reaction types).
    pub reaction_count: u32,
    /// Hyperlinks embedded in the message text via text entities.
    pub links: Vec<TextLink>,
    /// Whether the message has been edited after sending.
    pub is_edited: bool,
}

impl Message {
    /// Returns the display content: media label + text, or just text if no media.
    pub fn display_content(&self) -> String {
        match (self.media.display_label(), self.text.is_empty()) {
            (Some(label), true) => label.to_owned(),
            (Some(label), false) => format!("{}\n{}", label, self.text),
            (None, _) => self.text.clone(),
        }
    }
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
pub fn build_file_metadata_display(media: MessageMedia, info: &FileInfo) -> String {
    let mut parts = Vec::new();

    // Download status
    match info.download_status {
        DownloadStatus::Completed => parts.push("download=yes".to_owned()),
        DownloadStatus::Downloading { progress_percent } => {
            parts.push(format!("downloading={}%", progress_percent));
        }
        DownloadStatus::NotStarted => parts.push("download=no".to_owned()),
    }

    // Size
    if let Some(size) = info.size {
        parts.push(format!("size={}", format_file_size(size)));
    }

    // Duration (only for time-based media)
    if let Some(dur) = info.duration {
        match media {
            MessageMedia::Voice
            | MessageMedia::Audio
            | MessageMedia::Video
            | MessageMedia::VideoNote
            | MessageMedia::Animation => {
                parts.push(format!("duration={}", format_duration(dur)));
            }
            _ => {}
        }
    }

    // Listened/viewed (only for voice and video notes)
    if matches!(media, MessageMedia::Voice | MessageMedia::VideoNote) && info.is_listened {
        parts.push("listened=yes".to_owned());
    }

    parts.join(", ")
}

/// Extracts the first URL from message text and link entities.
///
/// Checks entity links first (they may contain URLs not visible in text),
/// then falls back to whitespace-delimited scanning of plain text.
/// URLs without a scheme get `http://` prepended so they can be opened by the OS.
pub fn extract_first_url(text: &str, links: &[TextLink]) -> Option<String> {
    if let Some(link) = links.first() {
        return Some(normalize_url(&link.url));
    }
    text.split_whitespace()
        .find(|word| word.starts_with("https://") || word.starts_with("http://"))
        .map(|s| s.to_owned())
}

/// Ensures a URL has an `http://` or `https://` scheme.
///
/// TDLib `TextEntityTypeUrl` may match bare hosts like `127.0.0.1:8080`
/// or `example.com/path` — the OS launcher needs a full scheme to work.
fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_owned()
    } else {
        format!("http://{url}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(text: &str, media: MessageMedia) -> Message {
        Message {
            id: 1,
            sender_name: "User".to_owned(),
            text: text.to_owned(),
            timestamp_ms: 1000,
            is_outgoing: false,
            media,
            status: MessageStatus::Delivered,
            file_info: None,
            call_info: None,
            reply_to: None,
            reaction_count: 0,
            links: Vec::new(),
            is_edited: false,
        }
    }

    #[test]
    fn display_label_returns_none_for_no_media() {
        assert_eq!(MessageMedia::None.display_label(), None);
    }

    #[test]
    fn display_label_returns_photo_indicator() {
        assert_eq!(MessageMedia::Photo.display_label(), Some("[Photo]"));
    }

    #[test]
    fn display_label_returns_voice_indicator() {
        assert_eq!(MessageMedia::Voice.display_label(), Some("[Voice]"));
    }

    #[test]
    fn display_label_returns_sticker_indicator() {
        assert_eq!(MessageMedia::Sticker.display_label(), Some("[Sticker]"));
    }

    #[test]
    fn display_label_returns_video_note_indicator() {
        assert_eq!(
            MessageMedia::VideoNote.display_label(),
            Some("[Video message]")
        );
    }

    #[test]
    fn display_content_returns_text_only_when_no_media() {
        let message = msg("Hello world", MessageMedia::None);

        assert_eq!(message.display_content(), "Hello world");
    }

    #[test]
    fn display_content_returns_media_label_only_when_text_empty() {
        let message = msg("", MessageMedia::Photo);

        assert_eq!(message.display_content(), "[Photo]");
    }

    #[test]
    fn display_content_combines_media_label_and_text() {
        let message = msg("Check this out", MessageMedia::Photo);

        assert_eq!(message.display_content(), "[Photo]\nCheck this out");
    }

    #[test]
    fn display_content_media_with_text_uses_newline_separator() {
        let message = msg("Caption text", MessageMedia::Video);
        let content = message.display_content();

        assert!(
            content.contains('\n'),
            "Media + text should be separated by newline"
        );
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "[Video]");
        assert_eq!(lines[1], "Caption text");
    }

    #[test]
    fn display_content_media_only_has_no_newline() {
        let message = msg("", MessageMedia::Photo);
        let content = message.display_content();

        assert!(!content.contains('\n'));
        assert_eq!(content, "[Photo]");
    }

    #[test]
    fn display_content_text_only_has_no_media_prefix() {
        let message = msg("Just text", MessageMedia::None);
        let content = message.display_content();

        assert_eq!(content, "Just text");
        assert!(!content.starts_with('['));
    }

    #[test]
    fn display_content_handles_all_media_types() {
        let types = [
            (MessageMedia::Photo, "[Photo]"),
            (MessageMedia::Voice, "[Voice]"),
            (MessageMedia::Video, "[Video]"),
            (MessageMedia::VideoNote, "[Video message]"),
            (MessageMedia::Sticker, "[Sticker]"),
            (MessageMedia::Document, "[Document]"),
            (MessageMedia::Audio, "[Audio]"),
            (MessageMedia::Animation, "[GIF]"),
            (MessageMedia::Contact, "[Contact]"),
            (MessageMedia::Location, "[Location]"),
            (MessageMedia::Poll, "[Poll]"),
            (MessageMedia::Call, "[Call]"),
            (MessageMedia::VideoCall, "[Video call]"),
            (MessageMedia::Other, "[Media]"),
        ];

        for (media, expected_label) in types {
            let message = msg("", media);
            assert_eq!(
                message.display_content(),
                expected_label,
                "Failed for {:?}",
                media
            );
        }
    }

    // ── extract_first_url tests ──

    #[test]
    fn extract_first_url_returns_none_when_no_url() {
        assert_eq!(extract_first_url("hello world", &[]), None);
    }

    #[test]
    fn extract_first_url_finds_https() {
        assert_eq!(
            extract_first_url("visit https://example.com please", &[]),
            Some("https://example.com".to_owned())
        );
    }

    #[test]
    fn extract_first_url_finds_http() {
        assert_eq!(
            extract_first_url("go to http://example.com", &[]),
            Some("http://example.com".to_owned())
        );
    }

    #[test]
    fn extract_first_url_returns_first_when_multiple() {
        assert_eq!(
            extract_first_url("see https://first.com and https://second.com", &[]),
            Some("https://first.com".to_owned())
        );
    }

    #[test]
    fn extract_first_url_handles_url_at_start() {
        assert_eq!(
            extract_first_url("https://start.com is the link", &[]),
            Some("https://start.com".to_owned())
        );
    }

    #[test]
    fn extract_first_url_handles_url_at_end() {
        assert_eq!(
            extract_first_url("link: https://end.com", &[]),
            Some("https://end.com".to_owned())
        );
    }

    #[test]
    fn extract_first_url_returns_none_for_empty_string() {
        assert_eq!(extract_first_url("", &[]), None);
    }

    #[test]
    fn extract_first_url_ignores_non_http_schemes() {
        assert_eq!(extract_first_url("check ftp://files.com out", &[]), None);
    }

    #[test]
    fn extract_first_url_prefers_entity_link() {
        let links = vec![TextLink {
            offset: 0,
            length: 9,
            url: "https://hidden.com".to_owned(),
        }];
        assert_eq!(
            extract_first_url("click here and https://visible.com", &links),
            Some("https://hidden.com".to_owned())
        );
    }

    #[test]
    fn extract_first_url_falls_back_to_text_when_no_entities() {
        assert_eq!(
            extract_first_url("go to https://example.com", &[]),
            Some("https://example.com".to_owned())
        );
    }

    #[test]
    fn extract_first_url_prepends_scheme_to_bare_host() {
        let links = vec![TextLink {
            offset: 0,
            length: 18,
            url: "127.0.0.1:18789".to_owned(),
        }];
        assert_eq!(
            extract_first_url("127.0.0.1:18789", &links),
            Some("http://127.0.0.1:18789".to_owned())
        );
    }

    #[test]
    fn extract_first_url_preserves_existing_scheme() {
        let links = vec![TextLink {
            offset: 0,
            length: 22,
            url: "https://example.com".to_owned(),
        }];
        assert_eq!(
            extract_first_url("https://example.com", &links),
            Some("https://example.com".to_owned())
        );
    }

    // ── normalize_url tests ──

    #[test]
    fn normalize_url_adds_http_to_bare_host() {
        assert_eq!(normalize_url("127.0.0.1:8080"), "http://127.0.0.1:8080");
        assert_eq!(normalize_url("example.com/path"), "http://example.com/path");
    }

    #[test]
    fn normalize_url_keeps_https() {
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
    }

    #[test]
    fn normalize_url_keeps_http() {
        assert_eq!(normalize_url("http://example.com"), "http://example.com");
    }

    // ── format_file_size tests ──

    #[test]
    fn format_file_size_bytes() {
        assert_eq!(format_file_size(0), "0B");
        assert_eq!(format_file_size(999), "999B");
    }

    #[test]
    fn format_file_size_kilobytes() {
        assert_eq!(format_file_size(1_000), "1.0KB");
        assert_eq!(format_file_size(15_500), "15.5KB");
    }

    #[test]
    fn format_file_size_megabytes() {
        assert_eq!(format_file_size(1_000_000), "1.0MB");
        assert_eq!(format_file_size(1_400_000), "1.4MB");
        assert_eq!(format_file_size(20_600_000), "20.6MB");
    }

    #[test]
    fn format_file_size_gigabytes() {
        assert_eq!(format_file_size(1_000_000_000), "1.0GB");
        assert_eq!(format_file_size(2_500_000_000), "2.5GB");
    }

    // ── format_duration tests ──

    #[test]
    fn format_duration_seconds_only() {
        assert_eq!(format_duration(3), "0:03");
        assert_eq!(format_duration(59), "0:59");
    }

    #[test]
    fn format_duration_minutes_and_seconds() {
        assert_eq!(format_duration(60), "1:00");
        assert_eq!(format_duration(85), "1:25");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(3600), "1:00:00");
        assert_eq!(format_duration(3723), "1:02:03");
    }

    #[test]
    fn format_duration_zero() {
        assert_eq!(format_duration(0), "0:00");
    }

    #[test]
    fn format_duration_negative_clamps_to_zero() {
        assert_eq!(format_duration(-5), "0:00");
    }

    // ── build_file_metadata_display tests ──

    #[test]
    fn metadata_display_voice_completed() {
        let fi = FileInfo {
            file_id: 1,
            local_path: Some("/tmp/v.ogg".to_owned()),
            mime_type: "audio/ogg".to_owned(),
            size: Some(15_500),
            duration: Some(3),
            file_name: None,
            is_listened: true,
            download_status: DownloadStatus::Completed,
        };
        assert_eq!(
            build_file_metadata_display(MessageMedia::Voice, &fi),
            "download=yes, size=15.5KB, duration=0:03, listened=yes"
        );
    }

    #[test]
    fn metadata_display_photo_not_downloaded() {
        let fi = FileInfo {
            file_id: 2,
            local_path: None,
            mime_type: "image/jpeg".to_owned(),
            size: Some(1_400_000),
            duration: None,
            file_name: None,
            is_listened: false,
            download_status: DownloadStatus::NotStarted,
        };
        assert_eq!(
            build_file_metadata_display(MessageMedia::Photo, &fi),
            "download=no, size=1.4MB"
        );
    }

    #[test]
    fn metadata_display_downloading_progress() {
        let fi = FileInfo {
            file_id: 3,
            local_path: None,
            mime_type: "video/mp4".to_owned(),
            size: Some(10_000_000),
            duration: Some(120),
            file_name: None,
            is_listened: false,
            download_status: DownloadStatus::Downloading {
                progress_percent: 42,
            },
        };
        assert_eq!(
            build_file_metadata_display(MessageMedia::Video, &fi),
            "downloading=42%, size=10.0MB, duration=2:00"
        );
    }

    #[test]
    fn metadata_display_voice_not_listened() {
        let fi = FileInfo {
            file_id: 4,
            local_path: Some("/tmp/v.ogg".to_owned()),
            mime_type: "audio/ogg".to_owned(),
            size: Some(20_600),
            duration: Some(7),
            file_name: None,
            is_listened: false,
            download_status: DownloadStatus::Completed,
        };
        let display = build_file_metadata_display(MessageMedia::Voice, &fi);
        assert!(
            !display.contains("listened"),
            "should not show listened=yes when not listened"
        );
    }

    // ── build_call_metadata_display tests ──

    #[test]
    fn call_metadata_outgoing_connected() {
        let info = CallInfo {
            is_video: false,
            duration: 83,
            discard_reason: CallDiscardReason::HungUp,
        };
        assert_eq!(build_call_metadata_display(&info, true), "Outgoing, 1:23");
    }

    #[test]
    fn call_metadata_incoming_connected() {
        let info = CallInfo {
            is_video: false,
            duration: 5,
            discard_reason: CallDiscardReason::HungUp,
        };
        assert_eq!(build_call_metadata_display(&info, false), "Incoming, 0:05");
    }

    #[test]
    fn call_metadata_missed_incoming() {
        let info = CallInfo {
            is_video: false,
            duration: 0,
            discard_reason: CallDiscardReason::Missed,
        };
        assert_eq!(build_call_metadata_display(&info, false), "Missed");
    }

    #[test]
    fn call_metadata_missed_outgoing_shows_cancelled() {
        let info = CallInfo {
            is_video: false,
            duration: 0,
            discard_reason: CallDiscardReason::Missed,
        };
        assert_eq!(build_call_metadata_display(&info, true), "Cancelled");
    }

    #[test]
    fn call_metadata_declined() {
        let info = CallInfo {
            is_video: false,
            duration: 0,
            discard_reason: CallDiscardReason::Declined,
        };
        assert_eq!(build_call_metadata_display(&info, false), "Declined");
    }

    #[test]
    fn call_metadata_video_outgoing_connected() {
        let info = CallInfo {
            is_video: true,
            duration: 60,
            discard_reason: CallDiscardReason::HungUp,
        };
        assert_eq!(build_call_metadata_display(&info, true), "Outgoing, 1:00");
    }

    #[test]
    fn call_metadata_video_missed() {
        let info = CallInfo {
            is_video: true,
            duration: 0,
            discard_reason: CallDiscardReason::Missed,
        };
        assert_eq!(build_call_metadata_display(&info, false), "Missed");
    }

    #[test]
    fn call_metadata_disconnected_with_duration() {
        let info = CallInfo {
            is_video: false,
            duration: 30,
            discard_reason: CallDiscardReason::Disconnected,
        };
        assert_eq!(
            build_call_metadata_display(&info, true),
            "Disconnected, 0:30"
        );
    }

    #[test]
    fn call_metadata_hungup_zero_duration_incoming_shows_missed() {
        let info = CallInfo {
            is_video: false,
            duration: 0,
            discard_reason: CallDiscardReason::HungUp,
        };
        assert_eq!(build_call_metadata_display(&info, false), "Missed");
    }

    #[test]
    fn call_metadata_hungup_zero_duration_outgoing_shows_cancelled() {
        let info = CallInfo {
            is_video: false,
            duration: 0,
            discard_reason: CallDiscardReason::HungUp,
        };
        assert_eq!(build_call_metadata_display(&info, true), "Cancelled");
    }
}
