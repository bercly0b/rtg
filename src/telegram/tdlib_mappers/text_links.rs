use tdlib_rs::enums::MessageContent;

use crate::domain::message::TextLink;

/// Converts a UTF-16 code-unit offset to a UTF-8 byte offset within `text`.
///
/// TDLib reports entity offsets/lengths in UTF-16 code units, while Rust
/// strings are UTF-8. This function walks the string and maps between the two.
/// Returns `None` if the UTF-16 offset exceeds the string.
pub(super) fn utf16_offset_to_byte_offset(text: &str, utf16_offset: usize) -> Option<usize> {
    let mut utf16_pos = 0;
    for (byte_pos, ch) in text.char_indices() {
        if utf16_pos == utf16_offset {
            return Some(byte_pos);
        }
        utf16_pos += ch.len_utf16();
    }
    // Offset pointing exactly past the last character
    if utf16_pos == utf16_offset {
        return Some(text.len());
    }
    None
}

/// Extracts URL-bearing text entities from a `FormattedText` into domain `TextLink`s.
///
/// Handles `TextEntityTypeUrl` (URL visible in text) and `TextEntityTypeTextUrl`
/// (clickable text with a hidden URL). Converts TDLib's UTF-16 offsets to byte offsets.
pub(super) fn extract_text_links(formatted: &tdlib_rs::types::FormattedText) -> Vec<TextLink> {
    use tdlib_rs::enums::TextEntityType;

    formatted
        .entities
        .iter()
        .filter_map(|entity| {
            let utf16_offset = entity.offset as usize;
            let utf16_length = entity.length as usize;

            let byte_offset = utf16_offset_to_byte_offset(&formatted.text, utf16_offset)?;
            let byte_end =
                utf16_offset_to_byte_offset(&formatted.text, utf16_offset + utf16_length)?;
            let byte_length = byte_end - byte_offset;

            match &entity.r#type {
                TextEntityType::Url => {
                    let url = formatted.text[byte_offset..byte_end].to_owned();
                    Some(TextLink {
                        offset: byte_offset,
                        length: byte_length,
                        url,
                    })
                }
                TextEntityType::TextUrl(tu) => Some(TextLink {
                    offset: byte_offset,
                    length: byte_length,
                    url: tu.url.clone(),
                }),
                _ => None,
            }
        })
        .collect()
}

/// Extracts `TextLink`s from a `MessageContent`'s formatted text.
pub(super) fn extract_content_links(content: &MessageContent) -> Vec<TextLink> {
    match content {
        MessageContent::MessageText(t) => extract_text_links(&t.text),
        MessageContent::MessagePhoto(p) => extract_text_links(&p.caption),
        MessageContent::MessageVideo(v) => extract_text_links(&v.caption),
        MessageContent::MessageVoiceNote(v) => extract_text_links(&v.caption),
        MessageContent::MessageDocument(d) => extract_text_links(&d.caption),
        MessageContent::MessageAudio(a) => extract_text_links(&a.caption),
        MessageContent::MessageAnimation(a) => extract_text_links(&a.caption),
        _ => Vec::new(),
    }
}
