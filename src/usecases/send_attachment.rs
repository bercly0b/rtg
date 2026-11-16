//! Use case for sending an attachment to a chat.

use std::{fs::File, io::BufReader};

use crate::domain::attachment::{AttachmentKind, OutgoingAttachment};

use super::send_message::SendMessageSourceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendAttachmentError {
    /// The file is larger than Telegram lets this account upload.
    TooLarge,
    Source(SendMessageSourceError),
}

/// Trait for sending attachments to chats.
pub trait AttachmentSender {
    /// Sends the attachment to the chat, or to a specific forum topic when
    /// `topic_id` is `Some`.
    fn send_attachment(
        &self,
        chat_id: i64,
        topic_id: Option<i32>,
        attachment: &OutgoingAttachment,
    ) -> Result<(), SendAttachmentError>;
}

impl<T: AttachmentSender + ?Sized> AttachmentSender for std::sync::Arc<T> {
    fn send_attachment(
        &self,
        chat_id: i64,
        topic_id: Option<i32>,
        attachment: &OutgoingAttachment,
    ) -> Result<(), SendAttachmentError> {
        (**self).send_attachment(chat_id, topic_id, attachment)
    }
}

/// Decides how the file at `path` is sent: a JPEG, PNG or WebP image within
/// Telegram's photo limits as a photo, anything else as a document.
pub fn attachment_kind(path: &str, size_bytes: u64) -> AttachmentKind {
    AttachmentKind::for_file(size_bytes, photo_size(path))
}

fn photo_size(path: &str) -> Option<(u32, u32)> {
    use imagesize::ImageType;

    let mut reader = BufReader::new(File::open(path).ok()?);
    let image_type = imagesize::reader_type(&mut reader).ok()?;
    if !matches!(
        image_type,
        ImageType::Jpeg | ImageType::Png | ImageType::Webp
    ) {
        return None;
    }
    let size = image_type.reader_size(&mut reader).ok()?;
    Some((
        u32::try_from(size.width).ok()?,
        u32::try_from(size.height).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The signature and IHDR chunk of a PNG: all that its size is read from.
    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend([0, 0, 0, 13]);
        png.extend(b"IHDR");
        png.extend(width.to_be_bytes());
        png.extend(height.to_be_bytes());
        png.extend([8, 6, 0, 0, 0]);
        png.extend([0, 0, 0, 0]);
        png
    }

    /// A GIF header declaring a 100x50 image.
    const GIF_HEADER: &[u8] = b"GIF89a\x64\x00\x32\x00\x00\x00\x00";

    fn kind_of(file_name: &str, content: &[u8]) -> AttachmentKind {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(file_name);
        std::fs::write(&path, content).unwrap();
        attachment_kind(path.to_str().unwrap(), content.len() as u64)
    }

    #[test]
    fn png_screenshot_is_sent_as_a_photo() {
        assert_eq!(
            kind_of("shot.png", &png_header(1920, 1080)),
            AttachmentKind::Photo {
                width: 1920,
                height: 1080
            }
        );
    }

    #[test]
    fn image_is_recognised_by_content_not_by_file_name() {
        assert_eq!(
            kind_of("shot", &png_header(800, 600)),
            AttachmentKind::Photo {
                width: 800,
                height: 600
            }
        );
    }

    #[test]
    fn very_long_screenshot_is_sent_as_a_document() {
        assert_eq!(
            kind_of("long.png", &png_header(400, 9000)),
            AttachmentKind::Document
        );
    }

    #[test]
    fn gif_is_sent_as_a_document() {
        assert_eq!(kind_of("anim.gif", GIF_HEADER), AttachmentKind::Document);
    }

    #[test]
    fn text_file_is_sent_as_a_document() {
        assert_eq!(
            kind_of("notes.txt", b"hello world, plain text"),
            AttachmentKind::Document
        );
    }

    #[test]
    fn missing_file_is_sent_as_a_document() {
        assert_eq!(
            attachment_kind("/rtg-no-such-file.png", 10),
            AttachmentKind::Document
        );
    }
}
