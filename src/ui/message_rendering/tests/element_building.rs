use super::*;
use crate::ui::message_rendering::{
    build_message_list_elements, message_index_to_element_index, MessageListElement,
};

// ── date separator & sender grouping ──

#[test]
fn builds_date_separator_for_first_message() {
    let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];

    let elements = build_message_list_elements(&messages);

    assert_eq!(elements.len(), 2);
    assert!(matches!(&elements[0], MessageListElement::DateSeparator(_)));
}

#[test]
fn groups_consecutive_messages_from_same_sender() {
    let messages = vec![
        msg(1, "Alice", "First", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Second", FEB_14_2026_10AM + 60000, false),
    ];

    let elements = build_message_list_elements(&messages);

    // DateSeparator + Message1 (with sender) + Message2 (no sender)
    assert_eq!(elements.len(), 3);

    if let MessageListElement::Message { sender, .. } = &elements[1] {
        assert!(sender.is_some());
    } else {
        panic!("Expected Message element");
    }

    if let MessageListElement::Message { sender, .. } = &elements[2] {
        assert!(sender.is_none());
    } else {
        panic!("Expected Message element");
    }
}

#[test]
fn shows_sender_when_sender_changes() {
    let messages = vec![
        msg(1, "Alice", "Hi", FEB_14_2026_10AM, false),
        msg(2, "Bob", "Hello", FEB_14_2026_10AM + 60000, false),
    ];

    let elements = build_message_list_elements(&messages);

    // DateSeparator + Message1 (Alice) + Message2 (Bob)
    assert_eq!(elements.len(), 3);

    if let MessageListElement::Message { sender, .. } = &elements[1] {
        assert_eq!(sender.as_deref(), Some("Alice"));
    }

    if let MessageListElement::Message { sender, .. } = &elements[2] {
        assert_eq!(sender.as_deref(), Some("Bob"));
    }
}

#[test]
fn inserts_date_separator_on_date_change() {
    let messages = vec![
        msg(1, "Alice", "Day 1", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Day 2", FEB_15_2026_1PM, false),
    ];

    let elements = build_message_list_elements(&messages);

    // DateSeparator1 + Message1 + DateSeparator2 + Message2
    assert_eq!(elements.len(), 4);
    assert!(matches!(&elements[0], MessageListElement::DateSeparator(_)));
    assert!(matches!(&elements[2], MessageListElement::DateSeparator(_)));
}

#[test]
fn resets_sender_grouping_on_date_change() {
    let messages = vec![
        msg(1, "Alice", "Day 1", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Day 2", FEB_15_2026_1PM, false),
    ];

    let elements = build_message_list_elements(&messages);

    // Both messages should show sender (after date separators)
    if let MessageListElement::Message { sender, .. } = &elements[1] {
        assert!(sender.is_some(), "First message should show sender");
    }

    if let MessageListElement::Message { sender, .. } = &elements[3] {
        assert!(
            sender.is_some(),
            "Message after date change should show sender"
        );
    }
}

#[test]
fn uses_you_for_outgoing_messages() {
    let messages = vec![msg(1, "MyName", "Hello", FEB_14_2026_10AM, true)];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { sender, .. } = &elements[1] {
        assert_eq!(sender.as_deref(), Some("You"));
    }
}

#[test]
fn outgoing_message_sets_is_outgoing_flag() {
    let messages = vec![msg(1, "MyName", "Hello", FEB_14_2026_10AM, true)];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message {
        is_outgoing,
        sender,
        ..
    } = &elements[1]
    {
        assert!(is_outgoing, "Outgoing message should have is_outgoing=true");
        assert_eq!(sender.as_deref(), Some("You"));
    } else {
        panic!("Expected Message element");
    }
}

#[test]
fn incoming_message_sets_is_outgoing_false() {
    let messages = vec![msg(1, "Alice", "Hi", FEB_14_2026_10AM, false)];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { is_outgoing, .. } = &elements[1] {
        assert!(
            !is_outgoing,
            "Incoming message should have is_outgoing=false"
        );
    } else {
        panic!("Expected Message element");
    }
}

#[test]
fn media_message_shows_indicator() {
    let messages = vec![msg_with_media(
        1,
        "Alice",
        "",
        FEB_14_2026_10AM,
        MessageMedia::Photo,
    )];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { content, .. } = &elements[1] {
        assert_eq!(content, "[Photo]");
    }
}

#[test]
fn media_message_with_text_shows_both() {
    let messages = vec![msg_with_media(
        1,
        "Alice",
        "Check this out",
        FEB_14_2026_10AM,
        MessageMedia::Photo,
    )];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { content, .. } = &elements[1] {
        assert_eq!(content, "[Photo]\nCheck this out");
    }
}

// ── time grouping ──

#[test]
fn hides_duplicate_time_for_same_sender_same_minute() {
    let messages = vec![
        msg(1, "Alice", "First", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Second", FEB_14_2026_10AM + 5000, false), // +5s, same minute
    ];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { show_time, .. } = &elements[1] {
        assert!(show_time, "First message in group should show time");
    } else {
        panic!("Expected Message element");
    }

    if let MessageListElement::Message { show_time, .. } = &elements[2] {
        assert!(!show_time, "Same sender + same minute should hide time");
    } else {
        panic!("Expected Message element");
    }
}

#[test]
fn shows_time_when_minute_changes_within_same_sender_group() {
    let messages = vec![
        msg(1, "Alice", "First", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Second", FEB_14_2026_10AM + 60000, false), // +1 min
    ];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { show_time, .. } = &elements[1] {
        assert!(show_time, "First message should show time");
    }

    if let MessageListElement::Message { show_time, .. } = &elements[2] {
        assert!(show_time, "Different minute in same group should show time");
    }
}

#[test]
fn shows_time_when_sender_changes_even_if_same_minute() {
    let messages = vec![
        msg(1, "Alice", "Hi", FEB_14_2026_10AM, false),
        msg(2, "Bob", "Hello", FEB_14_2026_10AM + 5000, false), // same minute
    ];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { show_time, .. } = &elements[1] {
        assert!(show_time, "First message should show time");
    }

    if let MessageListElement::Message { show_time, .. } = &elements[2] {
        assert!(
            show_time,
            "Different sender should always show time even if same minute"
        );
    }
}

#[test]
fn resets_time_grouping_on_date_change() {
    let messages = vec![
        msg(1, "Alice", "Day 1", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Day 2", FEB_15_2026_1PM, false),
    ];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { show_time, .. } = &elements[1] {
        assert!(show_time, "First message should show time");
    }

    if let MessageListElement::Message { show_time, .. } = &elements[3] {
        assert!(show_time, "Message after date change should show time");
    }
}

#[test]
fn first_message_always_shows_time() {
    let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { show_time, .. } = &elements[1] {
        assert!(show_time, "Single message should always show time");
    }
}

#[test]
fn three_messages_same_sender_same_minute_only_first_shows_time() {
    let messages = vec![
        msg(1, "Alice", "One", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Two", FEB_14_2026_10AM + 10_000, false), // +10s
        msg(3, "Alice", "Three", FEB_14_2026_10AM + 20_000, false), // +20s
    ];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { show_time, .. } = &elements[1] {
        assert!(show_time, "First should show time");
    }
    if let MessageListElement::Message { show_time, .. } = &elements[2] {
        assert!(!show_time, "Second should hide time");
    }
    if let MessageListElement::Message { show_time, .. } = &elements[3] {
        assert!(!show_time, "Third should hide time");
    }
}

// ── message_index_to_element_index ──

#[test]
fn message_index_to_element_index_maps_first_message() {
    let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];
    let elements = build_message_list_elements(&messages);

    assert_eq!(message_index_to_element_index(&elements, 0), Some(1));
}

#[test]
fn message_index_to_element_index_accounts_for_date_separators() {
    let messages = vec![
        msg(1, "Alice", "Day 1", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Day 2", FEB_15_2026_1PM, false),
    ];
    let elements = build_message_list_elements(&messages);

    assert_eq!(message_index_to_element_index(&elements, 0), Some(1));
    assert_eq!(message_index_to_element_index(&elements, 1), Some(3));
}

#[test]
fn message_index_to_element_index_handles_multiple_messages_same_day() {
    let messages = vec![
        msg(1, "Alice", "First", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Second", FEB_14_2026_10AM + 60000, false),
        msg(3, "Bob", "Third", FEB_14_2026_10AM + 120000, false),
    ];
    let elements = build_message_list_elements(&messages);

    assert_eq!(message_index_to_element_index(&elements, 0), Some(1));
    assert_eq!(message_index_to_element_index(&elements, 1), Some(2));
    assert_eq!(message_index_to_element_index(&elements, 2), Some(3));
}

#[test]
fn message_index_to_element_index_returns_none_for_out_of_range() {
    let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];
    let elements = build_message_list_elements(&messages);

    assert_eq!(message_index_to_element_index(&elements, 5), None);
}

#[test]
fn message_index_to_element_index_returns_none_for_empty_elements() {
    let elements: Vec<MessageListElement> = vec![];

    assert_eq!(message_index_to_element_index(&elements, 0), None);
}

// ── format_date ──

#[test]
fn format_date_produces_correct_format() {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();

    let formatted = crate::ui::message_rendering::text_utils::format_date(date);

    assert_eq!(formatted, "14 Feb 2026");
}

#[test]
fn format_time_produces_hh_mm() {
    let time = crate::ui::message_rendering::text_utils::format_time(FEB_14_2026_10AM);

    assert_eq!(time.len(), 5);
    assert!(time.contains(':'));
}
