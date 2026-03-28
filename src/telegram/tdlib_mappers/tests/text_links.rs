use tdlib_rs::enums::MessageContent;

use crate::telegram::tdlib_mappers::text_links::{
    extract_content_links, extract_text_links, utf16_offset_to_byte_offset,
};

use super::{make_formatted_text, make_text_url_entity, make_url_entity};

#[test]
fn extract_text_links_returns_empty_for_no_entities() {
    let ft = make_formatted_text("Hello world", vec![]);
    assert!(extract_text_links(&ft).is_empty());
}

#[test]
fn extract_text_links_extracts_url_entity() {
    let text = "Visit https://example.com please";
    let ft = make_formatted_text(text, vec![make_url_entity(6, 19)]);

    let links = extract_text_links(&ft);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].offset, 6);
    assert_eq!(links[0].length, 19);
    assert_eq!(links[0].url, "https://example.com");
}

#[test]
fn extract_text_links_extracts_text_url_entity() {
    let text = "Click here for info";
    let ft = make_formatted_text(
        text,
        vec![make_text_url_entity(0, 10, "https://hidden.com")],
    );

    let links = extract_text_links(&ft);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].offset, 0);
    assert_eq!(links[0].length, 10);
    assert_eq!(links[0].url, "https://hidden.com");
}

#[test]
fn extract_text_links_ignores_non_url_entities() {
    let text = "Bold text here";
    let ft = make_formatted_text(
        text,
        vec![tdlib_rs::types::TextEntity {
            offset: 0,
            length: 4,
            r#type: tdlib_rs::enums::TextEntityType::Bold,
        }],
    );

    assert!(extract_text_links(&ft).is_empty());
}

#[test]
fn extract_text_links_handles_multiple_links() {
    let text = "See https://a.com and https://b.com";
    let ft = make_formatted_text(text, vec![make_url_entity(4, 13), make_url_entity(22, 13)]);

    let links = extract_text_links(&ft);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].url, "https://a.com");
    assert_eq!(links[1].url, "https://b.com");
}

#[test]
fn extract_content_links_from_text_message() {
    let content = MessageContent::MessageText(tdlib_rs::types::MessageText {
        text: make_formatted_text("Check https://example.com", vec![make_url_entity(6, 19)]),
        link_preview: None,
        link_preview_options: None,
    });

    let links = extract_content_links(&content);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].url, "https://example.com");
}

#[test]
fn extract_content_links_returns_empty_for_no_caption_types() {
    let content = MessageContent::MessageContact(tdlib_rs::types::MessageContact {
        contact: tdlib_rs::types::Contact {
            phone_number: "+1234567890".to_owned(),
            first_name: "John".to_owned(),
            last_name: String::new(),
            vcard: String::new(),
            user_id: 0,
        },
    });

    assert!(extract_content_links(&content).is_empty());
}

// ── UTF-16 offset conversion tests ──

#[test]
fn utf16_offset_to_byte_offset_ascii() {
    assert_eq!(utf16_offset_to_byte_offset("hello", 0), Some(0));
    assert_eq!(utf16_offset_to_byte_offset("hello", 3), Some(3));
    assert_eq!(utf16_offset_to_byte_offset("hello", 5), Some(5));
}

#[test]
fn utf16_offset_to_byte_offset_cyrillic() {
    // "Привет" — each Cyrillic char is 2 bytes in UTF-8 but 1 UTF-16 code unit
    let text = "Привет";
    assert_eq!(utf16_offset_to_byte_offset(text, 0), Some(0));
    assert_eq!(utf16_offset_to_byte_offset(text, 1), Some(2)); // after 'П'
    assert_eq!(utf16_offset_to_byte_offset(text, 6), Some(12)); // end
}

#[test]
fn utf16_offset_to_byte_offset_out_of_range() {
    assert_eq!(utf16_offset_to_byte_offset("hi", 10), None);
}

#[test]
fn extract_text_links_with_cyrillic_prefix() {
    // "Смотри тут" — "Смотри " is 7 Cyrillic chars = 7 UTF-16 code units, 14 UTF-8 bytes
    // "тут" starts at UTF-16 offset 7, length 3
    let text = "Смотри тут";
    let ft = make_formatted_text(
        text,
        vec![make_text_url_entity(7, 3, "https://example.com")],
    );

    let links = extract_text_links(&ft);
    assert_eq!(links.len(), 1);
    // Byte offset of "тут" in UTF-8: "Смотри " = 12 bytes (6 × 2) + 1 space = 13
    assert_eq!(links[0].offset, 13);
    assert_eq!(links[0].length, 6); // "тут" = 3 chars × 2 bytes
    assert_eq!(links[0].url, "https://example.com");
}

#[test]
fn extract_text_links_with_emoji_prefix() {
    // "\u{1f44d} link" — \u{1f44d} is 1 UTF-16 surrogate pair (2 code units), 4 UTF-8 bytes
    let text = "\u{1f44d} link";
    // "link" starts at UTF-16 offset 3 (2 for emoji + 1 for space), length 4
    let ft = make_formatted_text(text, vec![make_url_entity(3, 4)]);

    let links = extract_text_links(&ft);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].offset, 5); // 4 bytes for \u{1f44d} + 1 for space
    assert_eq!(links[0].length, 4);
    assert_eq!(links[0].url, "link");
}
