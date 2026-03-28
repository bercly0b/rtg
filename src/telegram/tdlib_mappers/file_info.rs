use tdlib_rs::enums::MessageContent;

use crate::domain::message::{CallInfo, DownloadStatus, FileInfo};

/// Extra metadata extracted alongside the TDLib `File` reference.
struct FileMetadata {
    mime: String,
    duration: Option<i32>,
    file_name: Option<String>,
    is_listened: bool,
}

/// Extracts file metadata from a TDLib MessageContent, if it carries a downloadable file.
///
/// Returns `Some(FileInfo)` for media types that have a file (voice, audio, video, document,
/// photo, etc.) and `None` for text, polls, contacts, locations, and service messages.
pub fn extract_file_info(content: &MessageContent) -> Option<FileInfo> {
    match content {
        MessageContent::MessagePhoto(p) => extract_photo_file_info(p),
        _ => extract_single_file_info(content),
    }
}

/// Extracts file info for media types that carry a single `File`.
fn extract_single_file_info(content: &MessageContent) -> Option<FileInfo> {
    let (file, meta) = match content {
        MessageContent::MessageVoiceNote(v) => (
            &v.voice_note.voice,
            FileMetadata {
                mime: v.voice_note.mime_type.clone(),
                duration: Some(v.voice_note.duration),
                file_name: None,
                is_listened: v.is_listened,
            },
        ),
        MessageContent::MessageAudio(a) => (
            &a.audio.audio,
            FileMetadata {
                mime: a.audio.mime_type.clone(),
                duration: Some(a.audio.duration),
                file_name: Some(a.audio.file_name.clone()).filter(|s| !s.is_empty()),
                is_listened: false,
            },
        ),
        MessageContent::MessageDocument(d) => (
            &d.document.document,
            FileMetadata {
                mime: d.document.mime_type.clone(),
                duration: None,
                file_name: Some(d.document.file_name.clone()).filter(|s| !s.is_empty()),
                is_listened: false,
            },
        ),
        MessageContent::MessageVideo(v) => (
            &v.video.video,
            FileMetadata {
                mime: v.video.mime_type.clone(),
                duration: Some(v.video.duration),
                file_name: None,
                is_listened: false,
            },
        ),
        MessageContent::MessageVideoNote(v) => (
            &v.video_note.video,
            FileMetadata {
                mime: "video/mp4".to_owned(),
                duration: Some(v.video_note.duration),
                file_name: None,
                is_listened: v.is_viewed,
            },
        ),
        MessageContent::MessageAnimation(a) => (
            &a.animation.animation,
            FileMetadata {
                mime: a.animation.mime_type.clone(),
                duration: Some(a.animation.duration),
                file_name: None,
                is_listened: false,
            },
        ),
        _ => return None,
    };

    Some(build_file_info(file, meta))
}

/// Extracts file info from a photo message by selecting the largest PhotoSize.
fn extract_photo_file_info(p: &tdlib_rs::types::MessagePhoto) -> Option<FileInfo> {
    let largest = p.photo.sizes.iter().max_by_key(|s| s.width * s.height)?;
    let file = &largest.photo;
    let meta = FileMetadata {
        // TDLib PhotoSize doesn't expose MIME type; JPEG is the most common format.
        mime: "image/jpeg".to_owned(),
        duration: None,
        file_name: None,
        is_listened: false,
    };
    Some(build_file_info(file, meta))
}

/// Builds a `FileInfo` from a TDLib `File` and extracted metadata.
fn build_file_info(file: &tdlib_rs::types::File, meta: FileMetadata) -> FileInfo {
    let is_completed = file.local.is_downloading_completed && !file.local.path.is_empty();
    let local_path = if is_completed {
        Some(file.local.path.clone())
    } else {
        None
    };

    let download_status = if is_completed {
        DownloadStatus::Completed
    } else if file.local.is_downloading_active {
        let total = effective_file_size(file);
        let percent = if total > 0 {
            ((file.local.downloaded_size as u64) * 100 / total).min(99) as u8
        } else {
            0
        };
        DownloadStatus::Downloading {
            progress_percent: percent,
        }
    } else {
        DownloadStatus::NotStarted
    };

    let size = {
        let s = effective_file_size(file);
        if s > 0 {
            Some(s)
        } else {
            None
        }
    };

    FileInfo {
        file_id: file.id,
        local_path,
        mime_type: meta.mime,
        size,
        duration: meta.duration,
        file_name: meta.file_name,
        is_listened: meta.is_listened,
        download_status,
    }
}

/// Extracts call metadata from a `MessageCall` content.
pub(super) fn extract_call_info(content: &MessageContent) -> Option<CallInfo> {
    let MessageContent::MessageCall(c) = content else {
        return None;
    };

    use tdlib_rs::enums::CallDiscardReason as TdReason;

    let discard_reason = match &c.discard_reason {
        TdReason::Missed => crate::domain::message::CallDiscardReason::Missed,
        TdReason::Declined => crate::domain::message::CallDiscardReason::Declined,
        TdReason::Disconnected => crate::domain::message::CallDiscardReason::Disconnected,
        TdReason::HungUp | TdReason::Empty | TdReason::UpgradeToGroupCall(_) => {
            crate::domain::message::CallDiscardReason::HungUp
        }
    };

    Some(CallInfo {
        is_video: c.is_video,
        duration: c.duration,
        discard_reason,
    })
}

/// Returns the best known file size from TDLib's `File` struct.
///
/// Guards against negative sentinel values from TDLib by clamping to 0.
fn effective_file_size(file: &tdlib_rs::types::File) -> u64 {
    let size = file.size.max(0) as u64;
    if size > 0 {
        size
    } else {
        file.expected_size.max(0) as u64
    }
}
