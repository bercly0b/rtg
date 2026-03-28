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
