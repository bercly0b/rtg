use tdlib_rs::enums::MessageContent;

use crate::domain::message::DownloadStatus;
use crate::telegram::tdlib_mappers::extract_file_info;

use super::make_test_file;

#[test]
fn extract_file_info_returns_none_for_text_message() {
    let content = MessageContent::MessageText(tdlib_rs::types::MessageText {
        text: tdlib_rs::types::FormattedText {
            text: "hello".to_owned(),
            entities: vec![],
        },
        link_preview: None,
        link_preview_options: None,
    });
    assert!(extract_file_info(&content).is_none());
}

#[test]
fn extract_file_info_for_downloaded_voice_note() {
    let content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
        voice_note: tdlib_rs::types::VoiceNote {
            duration: 5,
            waveform: String::new(),
            mime_type: "audio/ogg".to_owned(),
            speech_recognition_result: None,
            voice: make_test_file(42, "/tmp/voice.ogg", true),
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
        is_listened: false,
    });

    let fi = extract_file_info(&content).expect("should have file info");
    assert_eq!(fi.file_id, 42);
    assert_eq!(fi.local_path, Some("/tmp/voice.ogg".to_owned()));
    assert_eq!(fi.mime_type, "audio/ogg");
}

#[test]
fn extract_file_info_for_not_downloaded_voice_note() {
    let content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
        voice_note: tdlib_rs::types::VoiceNote {
            duration: 5,
            waveform: String::new(),
            mime_type: "audio/ogg".to_owned(),
            speech_recognition_result: None,
            voice: make_test_file(42, "", false),
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
        is_listened: false,
    });

    let fi = extract_file_info(&content).expect("should have file info");
    assert_eq!(fi.file_id, 42);
    assert!(fi.local_path.is_none());
    assert_eq!(fi.mime_type, "audio/ogg");
}

#[test]
fn extract_file_info_for_downloaded_audio() {
    let content = MessageContent::MessageAudio(tdlib_rs::types::MessageAudio {
        audio: tdlib_rs::types::Audio {
            duration: 180,
            title: "Song".to_owned(),
            performer: "Artist".to_owned(),
            file_name: "song.mp3".to_owned(),
            mime_type: "audio/mpeg".to_owned(),
            album_cover_minithumbnail: None,
            album_cover_thumbnail: None,
            external_album_covers: vec![],
            audio: make_test_file(99, "/tmp/song.mp3", true),
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
    });

    let fi = extract_file_info(&content).expect("should have file info");
    assert_eq!(fi.file_id, 99);
    assert_eq!(fi.local_path, Some("/tmp/song.mp3".to_owned()));
    assert_eq!(fi.mime_type, "audio/mpeg");
}

#[test]
fn extract_file_info_returns_none_for_photo() {
    let content = MessageContent::MessagePhoto(tdlib_rs::types::MessagePhoto {
        photo: tdlib_rs::types::Photo {
            minithumbnail: None,
            sizes: vec![],
            has_stickers: false,
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
        show_caption_above_media: false,
        has_spoiler: false,
        is_secret: false,
    });
    assert!(extract_file_info(&content).is_none());
}

#[test]
fn extract_file_info_for_photo_with_sizes() {
    let content = MessageContent::MessagePhoto(tdlib_rs::types::MessagePhoto {
        photo: tdlib_rs::types::Photo {
            minithumbnail: None,
            sizes: vec![
                tdlib_rs::types::PhotoSize {
                    r#type: "s".to_owned(),
                    photo: make_test_file(10, "", false),
                    width: 100,
                    height: 100,
                    progressive_sizes: vec![],
                },
                tdlib_rs::types::PhotoSize {
                    r#type: "m".to_owned(),
                    photo: make_test_file(20, "/tmp/photo.jpg", true),
                    width: 800,
                    height: 600,
                    progressive_sizes: vec![],
                },
            ],
            has_stickers: false,
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
        show_caption_above_media: false,
        has_spoiler: false,
        is_secret: false,
    });

    let fi = extract_file_info(&content).expect("photo with sizes should have file_info");
    assert_eq!(fi.file_id, 20, "should select the largest photo size");
    assert_eq!(fi.local_path, Some("/tmp/photo.jpg".to_owned()));
    assert_eq!(fi.mime_type, "image/jpeg");
    assert_eq!(fi.download_status, DownloadStatus::Completed);
}

#[test]
fn extract_file_info_includes_duration_for_voice() {
    let content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
        voice_note: tdlib_rs::types::VoiceNote {
            duration: 42,
            waveform: String::new(),
            mime_type: "audio/ogg".to_owned(),
            speech_recognition_result: None,
            voice: make_test_file(1, "/tmp/v.ogg", true),
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
        is_listened: true,
    });

    let fi = extract_file_info(&content).expect("should have file info");
    assert_eq!(fi.duration, Some(42));
    assert!(fi.is_listened);
    assert_eq!(fi.size, Some(1000));
    assert_eq!(fi.download_status, DownloadStatus::Completed);
}

#[test]
fn extract_file_info_includes_file_name_for_document() {
    let content = MessageContent::MessageDocument(tdlib_rs::types::MessageDocument {
        document: tdlib_rs::types::Document {
            file_name: "report.pdf".to_owned(),
            mime_type: "application/pdf".to_owned(),
            minithumbnail: None,
            thumbnail: None,
            document: make_test_file(5, "", false),
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
    });

    let fi = extract_file_info(&content).expect("should have file info");
    assert_eq!(fi.file_name, Some("report.pdf".to_owned()));
    assert_eq!(fi.duration, None);
    assert_eq!(fi.download_status, DownloadStatus::NotStarted);
}
