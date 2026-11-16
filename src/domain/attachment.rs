//! Attachments: local files the user sends into a chat.

use super::{message::MessageMedia, message_input_state::ReplyContext};

/// TDLib limits for `inputMessagePhoto`: at most 10 MB, width and height at
/// most 10000 in total, and a side ratio of at most 20.
const MAX_PHOTO_BYTES: u64 = 10_000_000;
const MAX_PHOTO_SIDES_SUM: u32 = 10_000;
const MAX_PHOTO_SIDE_RATIO: u32 = 20;

/// How Telegram receives an attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    /// A compressed photo.
    Photo { width: u32, height: u32 },
    /// A general file, sent as is.
    Document,
}

impl AttachmentKind {
    /// Picks the kind for a file of `size_bytes`. `image_size` is the width
    /// and height of an image Telegram shows as a photo, `None` for any other
    /// file. An image outside the photo limits is sent as a document.
    pub fn for_file(size_bytes: u64, image_size: Option<(u32, u32)>) -> Self {
        let Some((width, height)) = image_size else {
            return Self::Document;
        };
        let (short, long) = (width.min(height), width.max(height));
        let fits = size_bytes <= MAX_PHOTO_BYTES
            && short > 0
            && width.saturating_add(height) <= MAX_PHOTO_SIDES_SUM
            && long <= short.saturating_mul(MAX_PHOTO_SIDE_RATIO);
        if fits {
            Self::Photo { width, height }
        } else {
            Self::Document
        }
    }

    pub fn media(self) -> MessageMedia {
        match self {
            Self::Photo { .. } => MessageMedia::Photo,
            Self::Document => MessageMedia::Document,
        }
    }
}

/// An attachment on its way to Telegram: shown as the pending message
/// `pending_id` until Telegram accepts it; on failure the caption is put
/// back into the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingAttachment {
    pub pending_id: i64,
    pub path: String,
    pub size_bytes: u64,
    pub kind: AttachmentKind,
    pub caption: String,
    pub reply_to: Option<ReplyContext>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_that_is_not_an_image_is_a_document() {
        assert_eq!(
            AttachmentKind::for_file(1_000, None),
            AttachmentKind::Document
        );
    }

    #[test]
    fn an_ordinary_image_is_a_photo() {
        assert_eq!(
            AttachmentKind::for_file(2_000_000, Some((1920, 1080))),
            AttachmentKind::Photo {
                width: 1920,
                height: 1080
            }
        );
    }

    #[test]
    fn an_image_over_ten_megabytes_is_a_document() {
        assert_eq!(
            AttachmentKind::for_file(10_000_001, Some((1920, 1080))),
            AttachmentKind::Document
        );
    }

    #[test]
    fn an_image_with_sides_over_ten_thousand_in_total_is_a_document() {
        assert_eq!(
            AttachmentKind::for_file(1_000, Some((5000, 5000))),
            AttachmentKind::Photo {
                width: 5000,
                height: 5000
            }
        );
        assert_eq!(
            AttachmentKind::for_file(1_000, Some((5001, 5000))),
            AttachmentKind::Document
        );
    }

    #[test]
    fn a_long_screenshot_past_the_side_ratio_is_a_document() {
        assert_eq!(
            AttachmentKind::for_file(1_000, Some((400, 8000))),
            AttachmentKind::Photo {
                width: 400,
                height: 8000
            }
        );
        assert_eq!(
            AttachmentKind::for_file(1_000, Some((400, 8001))),
            AttachmentKind::Document
        );
    }

    #[test]
    fn an_image_with_a_zero_side_is_a_document() {
        assert_eq!(
            AttachmentKind::for_file(1_000, Some((0, 100))),
            AttachmentKind::Document
        );
    }
}
