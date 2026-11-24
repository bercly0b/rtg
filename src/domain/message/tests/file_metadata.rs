use super::*;

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
        upload_progress_percent: None,
    };
    assert_eq!(
        build_file_metadata_display(MessageMedia::Voice, MessageStatus::Delivered, &fi),
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
        upload_progress_percent: None,
    };
    assert_eq!(
        build_file_metadata_display(MessageMedia::Photo, MessageStatus::Delivered, &fi),
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
        upload_progress_percent: None,
    };
    assert_eq!(
        build_file_metadata_display(MessageMedia::Video, MessageStatus::Delivered, &fi),
        "downloading=42%, size=10.0MB, duration=2:00"
    );
}

#[test]
fn metadata_display_document_with_extension() {
    let fi = FileInfo {
        file_id: 5,
        local_path: Some("/tmp/report.pdf".to_owned()),
        mime_type: "application/pdf".to_owned(),
        size: Some(2_500_000),
        duration: None,
        file_name: Some("report.pdf".to_owned()),
        is_listened: false,
        download_status: DownloadStatus::Completed,
        upload_progress_percent: None,
    };
    assert_eq!(
        build_file_metadata_display(MessageMedia::Document, MessageStatus::Delivered, &fi),
        "type=pdf, download=yes, size=2.5MB, name=report.pdf"
    );
}

#[test]
fn metadata_display_document_without_extension() {
    let fi = FileInfo {
        file_id: 6,
        local_path: None,
        mime_type: "application/octet-stream".to_owned(),
        size: Some(500),
        duration: None,
        file_name: Some("notes".to_owned()),
        is_listened: false,
        download_status: DownloadStatus::NotStarted,
        upload_progress_percent: None,
    };
    let display =
        build_file_metadata_display(MessageMedia::Document, MessageStatus::Delivered, &fi);
    assert!(
        display.contains("name=notes"),
        "should contain file name, got: '{}'",
        display
    );
    assert!(
        !display.contains("type="),
        "should not contain type for extension-less name, got: '{}'",
        display
    );
    assert!(display.contains("download=no"), "got: '{}'", display);
    assert!(display.contains("size=500B"), "got: '{}'", display);
}

#[test]
fn metadata_display_audio_with_name_and_duration() {
    let fi = FileInfo {
        file_id: 7,
        local_path: Some("/tmp/song.mp3".to_owned()),
        mime_type: "audio/mpeg".to_owned(),
        size: Some(4_200_000),
        duration: Some(185),
        file_name: Some("song.mp3".to_owned()),
        is_listened: false,
        download_status: DownloadStatus::Completed,
        upload_progress_percent: None,
    };
    assert_eq!(
        build_file_metadata_display(MessageMedia::Audio, MessageStatus::Delivered, &fi),
        "type=mp3, download=yes, size=4.2MB, duration=3:05, name=song.mp3"
    );
}

#[test]
fn metadata_display_multi_dot_name_uses_last_segment() {
    let fi = FileInfo {
        file_id: 8,
        local_path: None,
        mime_type: "application/gzip".to_owned(),
        size: None,
        duration: None,
        file_name: Some("archive.tar.gz".to_owned()),
        is_listened: false,
        download_status: DownloadStatus::NotStarted,
        upload_progress_percent: None,
    };
    let display =
        build_file_metadata_display(MessageMedia::Document, MessageStatus::Delivered, &fi);
    assert!(
        display.contains("name=archive.tar.gz"),
        "got: '{}'",
        display
    );
    assert!(display.contains("type=gz"), "got: '{}'", display);
}

// ── file_extension tests ──

#[test]
fn file_extension_simple() {
    assert_eq!(file_extension("report.pdf"), Some("pdf"));
    assert_eq!(file_extension("song.MP3"), Some("MP3"));
}

#[test]
fn file_extension_multi_dot_returns_last() {
    assert_eq!(file_extension("archive.tar.gz"), Some("gz"));
}

#[test]
fn file_extension_no_dot() {
    assert_eq!(file_extension("notes"), None);
}

#[test]
fn file_extension_empty_string() {
    assert_eq!(file_extension(""), None);
}

#[test]
fn file_extension_trailing_dot() {
    assert_eq!(file_extension("a."), None);
}

#[test]
fn file_extension_hidden_file() {
    assert_eq!(file_extension(".env"), None);
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
        upload_progress_percent: None,
    };
    let display = build_file_metadata_display(MessageMedia::Voice, MessageStatus::Delivered, &fi);
    assert!(
        !display.contains("listened"),
        "should not show listened=yes when not listened"
    );
}

fn document_being_sent(upload_progress_percent: Option<u8>) -> FileInfo {
    FileInfo {
        file_id: 1,
        local_path: Some("/d/123.mp4".to_owned()),
        mime_type: "video/mp4".to_owned(),
        size: Some(166_000_000),
        duration: None,
        file_name: Some("123.mp4".to_owned()),
        is_listened: false,
        download_status: DownloadStatus::Completed,
        upload_progress_percent,
    }
}

#[test]
fn metadata_display_of_a_file_being_sent_shows_upload_progress_instead_of_download() {
    assert_eq!(
        build_file_metadata_display(
            MessageMedia::Document,
            MessageStatus::Sending,
            &document_being_sent(Some(5))
        ),
        "type=mp4, sending=5%, size=166.0MB, name=123.mp4"
    );
}

#[test]
fn metadata_display_of_a_file_being_sent_has_no_download_state_once_uploaded() {
    assert_eq!(
        build_file_metadata_display(
            MessageMedia::Document,
            MessageStatus::Sending,
            &document_being_sent(None)
        ),
        "type=mp4, size=166.0MB, name=123.mp4"
    );
}
