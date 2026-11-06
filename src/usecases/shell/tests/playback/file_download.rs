use std::time::{Duration, Instant};

use super::*;
use crate::domain::download_speed::MIN_TRACKED_FILE_SIZE;
use crate::domain::message::{DownloadStatus, FileInfo};

fn orchestrator_downloading_voice(file_id: i32) -> TestOrchestrator {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_not_downloaded(file_id.into())],
    );
    o.handle_event(AppEvent::InputKey(KeyInput::new("D", false)))
        .unwrap();
    o
}

fn file_updated(
    file_id: i32,
    local_path: Option<&str>,
    download_status: DownloadStatus,
) -> AppEvent {
    AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::FileUpdated {
            file_id,
            size: 2000,
            downloaded_size: 0,
            local_path: local_path.map(str::to_owned),
            download_status,
        }],
    }
}

fn large_file_progress(file_id: i32, downloaded_size: u64) -> AppEvent {
    AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::FileUpdated {
            file_id,
            size: MIN_TRACKED_FILE_SIZE,
            downloaded_size,
            local_path: None,
            download_status: DownloadStatus::Downloading {
                progress_percent: 0,
            },
        }],
    }
}

/// The progress events in these tests arrive back to back; a minute later
/// the stalled download has a measurable (decayed) speed.
fn download_speed_a_minute_later(o: &TestOrchestrator) -> Option<u64> {
    o.state()
        .download_speed()
        .bytes_per_second(Instant::now() + Duration::from_secs(60))
}

fn selected_file_info(o: &TestOrchestrator) -> FileInfo {
    o.state()
        .open_chat()
        .selected_message()
        .and_then(|m| m.file_info.clone())
        .expect("selected message should carry file info")
}

#[test]
fn file_update_in_progress_shows_download_progress() {
    let mut o = orchestrator_downloading_voice(10);

    o.handle_event(file_updated(
        10,
        None,
        DownloadStatus::Downloading {
            progress_percent: 40,
        },
    ))
    .unwrap();

    let fi = selected_file_info(&o);
    assert_eq!(
        fi.download_status,
        DownloadStatus::Downloading {
            progress_percent: 40
        }
    );
    assert_eq!(fi.local_path, None);
    assert_eq!(fi.size, Some(2000));
}

#[test]
fn file_update_completed_sets_local_path_and_ends_download() {
    let mut o = orchestrator_downloading_voice(10);

    o.handle_event(file_updated(
        10,
        Some("/tmp/voice.ogg"),
        DownloadStatus::Completed,
    ))
    .unwrap();
    o.handle_event(file_updated(10, None, DownloadStatus::NotStarted))
        .unwrap();

    let fi = selected_file_info(&o);
    assert_eq!(fi.download_status, DownloadStatus::Completed);
    assert_eq!(fi.local_path.as_deref(), Some("/tmp/voice.ogg"));
}

#[test]
fn file_update_not_started_ends_cancelled_download() {
    let mut o = orchestrator_downloading_voice(10);

    o.handle_event(file_updated(10, None, DownloadStatus::NotStarted))
        .unwrap();
    o.handle_event(file_updated(
        10,
        None,
        DownloadStatus::Downloading {
            progress_percent: 10,
        },
    ))
    .unwrap();

    assert_eq!(
        selected_file_info(&o).download_status,
        DownloadStatus::NotStarted
    );
}

#[test]
fn file_update_for_untracked_file_is_ignored() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_not_downloaded(10)],
    );

    o.handle_event(file_updated(
        10,
        Some("/tmp/voice.ogg"),
        DownloadStatus::Completed,
    ))
    .unwrap();

    assert_eq!(
        selected_file_info(&o).download_status,
        DownloadStatus::NotStarted
    );
}

#[test]
fn large_file_progress_feeds_download_speed() {
    let mut o = orchestrator_downloading_voice(10);

    o.handle_event(large_file_progress(10, 0)).unwrap();
    o.handle_event(large_file_progress(10, 3_000_000)).unwrap();

    assert!(download_speed_a_minute_later(&o).is_some_and(|bps| bps > 0));
}

#[test]
fn completed_download_clears_download_speed() {
    let mut o = orchestrator_downloading_voice(10);
    o.handle_event(large_file_progress(10, 0)).unwrap();
    o.handle_event(large_file_progress(10, 3_000_000)).unwrap();

    o.handle_event(file_updated(
        10,
        Some("/tmp/voice.ogg"),
        DownloadStatus::Completed,
    ))
    .unwrap();

    assert_eq!(download_speed_a_minute_later(&o), None);
}

#[test]
fn cancelled_download_clears_download_speed() {
    let mut o = orchestrator_downloading_voice(10);
    o.handle_event(large_file_progress(10, 0)).unwrap();
    o.handle_event(large_file_progress(10, 3_000_000)).unwrap();

    o.handle_event(file_updated(10, None, DownloadStatus::NotStarted))
        .unwrap();

    assert_eq!(download_speed_a_minute_later(&o), None);
}

#[test]
fn untracked_file_progress_does_not_feed_download_speed() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_not_downloaded(10)],
    );

    o.handle_event(large_file_progress(10, 0)).unwrap();
    o.handle_event(large_file_progress(10, 3_000_000)).unwrap();

    assert_eq!(download_speed_a_minute_later(&o), None);
}
