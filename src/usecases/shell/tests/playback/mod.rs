mod audio_playback;
mod media_open;

use super::*;

fn voice_message_downloaded(id: i64, path: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Voice,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: Some(path.to_owned()),
            mime_type: "audio/ogg".to_owned(),
            size: Some(1000),
            duration: Some(3),
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn voice_message_not_downloaded(id: i64) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Voice,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: None,
            mime_type: "audio/ogg".to_owned(),
            size: Some(1000),
            duration: Some(3),
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::NotStarted,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn audio_message_downloaded(id: i64, path: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Audio,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: Some(path.to_owned()),
            mime_type: "audio/mpeg".to_owned(),
            size: Some(5000),
            duration: Some(180),
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn photo_message_downloaded(id: i64, path: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Photo,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: Some(path.to_owned()),
            mime_type: "image/jpeg".to_owned(),
            size: Some(50_000),
            duration: None,
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn video_message_downloaded(id: i64, path: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Video,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: Some(path.to_owned()),
            mime_type: "video/mp4".to_owned(),
            size: Some(10_000),
            duration: Some(30),
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}
