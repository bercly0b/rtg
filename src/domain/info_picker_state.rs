use super::{
    chat_info_state::ChatInfoSubject,
    message::{LinkTarget, Message, MessageSender},
    selectable_list::SelectableList,
};

/// One row of the info picker: whose chat info popup it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoPickerEntry {
    /// The sender's name or `@username`; the popup is titled with it while
    /// it loads.
    pub label: String,
    pub subject: ChatInfoSubject,
}

/// Lists whose chat info popup a message can show: its sender first, then its
/// mentions in text order, each one once. A mention of the sender stays, as
/// nothing tells that it is the same account before the lookup.
///
/// Empty for a message whose sender is unknown.
pub fn info_entries(message: &Message) -> Vec<InfoPickerEntry> {
    let Some(sender) = message.sender else {
        return Vec::new();
    };
    let mut entries = vec![InfoPickerEntry {
        label: message.sender_name.clone(),
        subject: match sender {
            MessageSender::User(user_id) => ChatInfoSubject::User(user_id),
            MessageSender::Chat(chat_id) => ChatInfoSubject::Chat(chat_id),
        },
    }];
    for link in &message.links {
        let LinkTarget::Mention(username) = &link.target else {
            continue;
        };
        let entry = InfoPickerEntry {
            label: link.target.label(),
            subject: ChatInfoSubject::Mention(username.clone()),
        };
        if !entries.contains(&entry) {
            entries.push(entry);
        }
    }
    entries
}

/// The sender of one message and those it mentions, offered when it holds a
/// mention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoPickerState {
    entries: SelectableList<InfoPickerEntry>,
}

impl InfoPickerState {
    pub fn new(entries: Vec<InfoPickerEntry>) -> Self {
        let mut list = SelectableList::default();
        list.replace(entries, None);
        Self { entries: list }
    }

    pub fn entries(&self) -> &[InfoPickerEntry] {
        self.entries.items()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.entries.selected_index()
    }

    pub fn selected(&self) -> Option<&InfoPickerEntry> {
        self.entries.selected()
    }

    pub fn select_next(&mut self) {
        self.entries.select_next();
    }

    pub fn select_previous(&mut self) {
        self.entries.select_previous();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::{MessageMedia, MessageStatus, TextLink};

    fn message(sender: Option<MessageSender>, targets: Vec<LinkTarget>) -> Message {
        Message {
            id: 1,
            sender,
            sender_name: "Alice".to_owned(),
            text: String::new(),
            timestamp_ms: 0,
            is_outgoing: false,
            media: MessageMedia::None,
            status: MessageStatus::Delivered,
            file_info: None,
            call_info: None,
            reply_to: None,
            forward_info: None,
            reaction_count: 0,
            links: targets
                .into_iter()
                .map(|target| TextLink {
                    offset: 0,
                    length: 0,
                    target,
                })
                .collect(),
            is_edited: false,
            is_service: false,
        }
    }

    fn mention(username: &str) -> LinkTarget {
        LinkTarget::Mention(username.to_owned())
    }

    fn mention_entry(username: &str) -> InfoPickerEntry {
        InfoPickerEntry {
            label: format!("@{username}"),
            subject: ChatInfoSubject::Mention(username.to_owned()),
        }
    }

    #[test]
    fn message_without_mentions_offers_only_its_sender() {
        let entries = info_entries(&message(Some(MessageSender::User(7)), vec![]));

        assert_eq!(
            entries,
            [InfoPickerEntry {
                label: "Alice".to_owned(),
                subject: ChatInfoSubject::User(7),
            }]
        );
    }

    #[test]
    fn chat_sender_is_offered_as_a_chat() {
        let entries = info_entries(&message(Some(MessageSender::Chat(-100)), vec![]));

        assert_eq!(entries[0].subject, ChatInfoSubject::Chat(-100));
    }

    #[test]
    fn mentions_follow_the_sender_in_text_order_without_repeats_or_links() {
        let entries = info_entries(&message(
            Some(MessageSender::User(7)),
            vec![
                mention("bob"),
                LinkTarget::Url("https://example.com".to_owned()),
                mention("carol"),
                mention("bob"),
            ],
        ));

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].subject, ChatInfoSubject::User(7));
        assert_eq!(entries[1..], [mention_entry("bob"), mention_entry("carol")]);
    }

    #[test]
    fn message_with_unknown_sender_offers_nobody() {
        assert!(info_entries(&message(None, vec![mention("bob")])).is_empty());
    }

    #[test]
    fn picker_opens_on_the_sender_and_stops_at_both_ends() {
        let mut picker = InfoPickerState::new(info_entries(&message(
            Some(MessageSender::User(7)),
            vec![mention("bob")],
        )));
        assert_eq!(picker.selected_index(), Some(0));

        picker.select_previous();
        assert_eq!(picker.selected_index(), Some(0));

        picker.select_next();
        picker.select_next();
        assert_eq!(picker.selected(), Some(&mention_entry("bob")));
    }
}
