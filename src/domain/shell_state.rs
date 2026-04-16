use std::time::{Duration, Instant};

use super::{
    chat::ChatSummary, chat_info_state::ChatInfoPopupState, chat_list_state::ChatListState,
    chat_search_state::ChatSearchState, command_popup_state::CommandPopupState,
    events::ConnectivityStatus, message_cache::MessageCache,
    message_info_state::MessageInfoPopupState, message_input_state::MessageInputState,
    open_chat_state::OpenChatState, reaction_picker_state::ReactionPickerState,
};

const NOTIFICATION_TTL: Duration = Duration::from_secs(3);

/// Represents which panel currently has focus for keyboard navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePane {
    /// The left panel showing the list of chats.
    #[default]
    ChatList,
    /// The right panel showing messages for the selected chat.
    Messages,
    /// The message input field for composing messages.
    MessageInput,
}

#[derive(Debug, Clone)]
pub struct ShellState {
    running: bool,
    connectivity_status: ConnectivityStatus,
    chat_list: ChatListState,
    open_chat: OpenChatState,
    message_cache: MessageCache,
    message_input: MessageInputState,
    active_pane: ActivePane,
    help_visible: bool,
    command_popup: Option<CommandPopupState>,
    notification: Option<(String, Instant)>,
    chat_info_popup: Option<ChatInfoPopupState>,
    message_info_popup: Option<MessageInfoPopupState>,
    reaction_picker: Option<ReactionPickerState>,
    chat_search: Option<ChatSearchState>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            running: true,
            connectivity_status: ConnectivityStatus::Connecting,
            chat_list: ChatListState::default(),
            open_chat: OpenChatState::default(),
            message_cache: MessageCache::default(),
            message_input: MessageInputState::default(),
            active_pane: ActivePane::default(),
            help_visible: false,
            command_popup: None,
            notification: None,
            chat_info_popup: None,
            message_info_popup: None,
            reaction_picker: None,
            chat_search: None,
        }
    }
}

impl ShellState {
    /// Creates a state pre-populated with cached chat list data.
    ///
    /// If `chats` is non-empty, `ChatListState` starts as `Ready` immediately,
    /// allowing the TUI to display cached chats on the very first frame.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_initial_chat_list(chats: Vec<ChatSummary>) -> Self {
        Self {
            chat_list: ChatListState::with_initial_chats(chats),
            ..Default::default()
        }
    }

    /// Creates a state with cached chat list data and custom cache limits.
    pub fn with_cache_limits(
        chats: Vec<ChatSummary>,
        max_cached_chats: usize,
        max_messages_per_chat: usize,
    ) -> Self {
        Self {
            chat_list: ChatListState::with_initial_chats(chats),
            message_cache: MessageCache::new(max_cached_chats, max_messages_per_chat),
            ..Default::default()
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn connectivity_status(&self) -> ConnectivityStatus {
        self.connectivity_status
    }

    pub fn set_connectivity_status(&mut self, status: ConnectivityStatus) {
        self.connectivity_status = status;
    }

    #[allow(dead_code)]
    pub fn chat_list(&self) -> &ChatListState {
        &self.chat_list
    }

    #[allow(dead_code)]
    pub fn chat_list_mut(&mut self) -> &mut ChatListState {
        &mut self.chat_list
    }

    pub fn open_chat(&self) -> &OpenChatState {
        &self.open_chat
    }

    pub fn open_chat_mut(&mut self) -> &mut OpenChatState {
        &mut self.open_chat
    }

    pub fn message_cache(&self) -> &MessageCache {
        &self.message_cache
    }

    pub fn message_cache_mut(&mut self) -> &mut MessageCache {
        &mut self.message_cache
    }

    pub fn active_pane(&self) -> ActivePane {
        self.active_pane
    }

    pub fn set_active_pane(&mut self, pane: ActivePane) {
        self.active_pane = pane;
    }

    pub fn help_visible(&self) -> bool {
        self.help_visible
    }

    pub fn show_help(&mut self) {
        self.help_visible = true;
    }

    pub fn hide_help(&mut self) {
        self.help_visible = false;
    }

    pub fn command_popup(&self) -> Option<&CommandPopupState> {
        self.command_popup.as_ref()
    }

    pub fn command_popup_mut(&mut self) -> Option<&mut CommandPopupState> {
        self.command_popup.as_mut()
    }

    pub fn open_command_popup(
        &mut self,
        title: impl Into<String>,
        kind: crate::domain::command_popup_state::CommandPopupKind,
    ) {
        self.command_popup = Some(CommandPopupState::new(title, kind));
    }

    pub fn close_command_popup(&mut self) {
        self.command_popup = None;
    }

    pub fn chat_info_popup(&self) -> Option<&ChatInfoPopupState> {
        self.chat_info_popup.as_ref()
    }

    pub fn show_chat_info_loading(&mut self, chat_id: i64, title: impl Into<String>) {
        self.chat_info_popup = Some(ChatInfoPopupState::Loading {
            chat_id,
            title: title.into(),
        });
    }

    pub fn set_chat_info_loaded(&mut self, state: ChatInfoPopupState) {
        self.chat_info_popup = Some(state);
    }

    pub fn close_chat_info_popup(&mut self) {
        self.chat_info_popup = None;
    }

    pub fn message_info_popup(&self) -> Option<&MessageInfoPopupState> {
        self.message_info_popup.as_ref()
    }

    pub fn show_message_info_loading(&mut self, chat_id: i64, message_id: i64) {
        self.message_info_popup = Some(MessageInfoPopupState::Loading {
            chat_id,
            message_id,
        });
    }

    pub fn set_message_info_loaded(&mut self, state: MessageInfoPopupState) {
        self.message_info_popup = Some(state);
    }

    pub fn close_message_info_popup(&mut self) {
        self.message_info_popup = None;
    }

    pub fn reaction_picker(&self) -> Option<&ReactionPickerState> {
        self.reaction_picker.as_ref()
    }

    pub fn reaction_picker_mut(&mut self) -> Option<&mut ReactionPickerState> {
        self.reaction_picker.as_mut()
    }

    pub fn show_reaction_picker_loading(&mut self, chat_id: i64, message_id: i64) {
        self.reaction_picker = Some(ReactionPickerState::Loading {
            chat_id,
            message_id,
        });
    }

    pub fn set_reaction_picker(&mut self, state: ReactionPickerState) {
        self.reaction_picker = Some(state);
    }

    pub fn close_reaction_picker(&mut self) {
        self.reaction_picker = None;
    }

    pub fn chat_search(&self) -> Option<&ChatSearchState> {
        self.chat_search.as_ref()
    }

    pub fn chat_search_mut(&mut self) -> Option<&mut ChatSearchState> {
        self.chat_search.as_mut()
    }

    pub fn open_chat_search(&mut self) {
        self.chat_search = Some(ChatSearchState::default());
    }

    pub fn close_chat_search(&mut self) {
        self.chat_search = None;
    }

    pub fn message_input(&self) -> &MessageInputState {
        &self.message_input
    }

    pub fn message_input_mut(&mut self) -> &mut MessageInputState {
        &mut self.message_input
    }

    pub fn set_notification(&mut self, text: impl Into<String>) {
        self.notification = Some((text.into(), Instant::now()));
    }

    /// Returns the notification text if it hasn't expired yet.
    pub fn active_notification(&self) -> Option<&str> {
        self.notification.as_ref().and_then(|(text, created_at)| {
            if created_at.elapsed() < NOTIFICATION_TTL {
                Some(text.as_str())
            } else {
                None
            }
        })
    }

    #[cfg(test)]
    pub fn set_notification_at(&mut self, text: impl Into<String>, at: Instant) {
        self.notification = Some((text.into(), at));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chat::{ChatSummary, ChatType, OutgoingReadStatus};

    fn chat(id: i64, title: &str) -> ChatSummary {
        ChatSummary {
            chat_id: id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: None,
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Private,
            last_message_sender: None,
            is_online: None,
            is_bot: false,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: None,
            unread_reaction_count: 0,
        }
    }

    #[test]
    fn default_state_is_running() {
        let state = ShellState::default();
        assert!(state.is_running());
    }

    #[test]
    fn default_connectivity_is_connecting() {
        let state = ShellState::default();
        assert_eq!(state.connectivity_status(), ConnectivityStatus::Connecting);
    }

    #[test]
    fn default_active_pane_is_chat_list() {
        let state = ShellState::default();
        assert_eq!(state.active_pane(), ActivePane::ChatList);
    }

    #[test]
    fn stop_sets_running_to_false() {
        let mut state = ShellState::default();
        state.stop();
        assert!(!state.is_running());
    }

    #[test]
    fn set_connectivity_status_updates_value() {
        let mut state = ShellState::default();
        state.set_connectivity_status(ConnectivityStatus::Connected);
        assert_eq!(state.connectivity_status(), ConnectivityStatus::Connected);
    }

    #[test]
    fn set_active_pane_switches_pane() {
        let mut state = ShellState::default();
        state.set_active_pane(ActivePane::Messages);
        assert_eq!(state.active_pane(), ActivePane::Messages);

        state.set_active_pane(ActivePane::MessageInput);
        assert_eq!(state.active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn with_initial_chat_list_empty_falls_back_to_default() {
        let state = ShellState::with_initial_chat_list(vec![]);
        assert!(state.is_running());
        assert_eq!(state.connectivity_status(), ConnectivityStatus::Connecting);
        assert_eq!(state.active_pane(), ActivePane::ChatList);
    }

    #[test]
    fn with_initial_chat_list_non_empty_populates_chats() {
        let chats = vec![chat(1, "Alice"), chat(2, "Bob")];
        let state = ShellState::with_initial_chat_list(chats);
        assert!(state.is_running());
        assert_eq!(state.chat_list().chats().len(), 2);
    }

    #[test]
    fn message_input_mut_allows_mutation() {
        let mut state = ShellState::default();
        state.message_input_mut().insert_char('a');
        assert_eq!(state.message_input().text(), "a");
    }

    #[test]
    fn help_not_visible_by_default() {
        let state = ShellState::default();
        assert!(!state.help_visible());
    }

    #[test]
    fn show_help_makes_it_visible() {
        let mut state = ShellState::default();
        state.show_help();
        assert!(state.help_visible());
    }

    #[test]
    fn hide_help_makes_it_invisible() {
        let mut state = ShellState::default();
        state.show_help();
        state.hide_help();
        assert!(!state.help_visible());
    }

    #[test]
    fn chat_info_popup_none_by_default() {
        let state = ShellState::default();
        assert!(state.chat_info_popup().is_none());
    }

    #[test]
    fn show_chat_info_loading_creates_popup() {
        let mut state = ShellState::default();
        state.show_chat_info_loading(1, "Alice");
        assert!(state.chat_info_popup().is_some());
        assert_eq!(state.chat_info_popup().unwrap().title(), "Alice");
    }

    #[test]
    fn close_chat_info_popup_clears_state() {
        let mut state = ShellState::default();
        state.show_chat_info_loading(1, "Alice");
        state.close_chat_info_popup();
        assert!(state.chat_info_popup().is_none());
    }

    #[test]
    fn set_chat_info_loaded_updates_popup() {
        use crate::domain::chat_info_state::{ChatInfo, ChatInfoPopupState};
        let mut state = ShellState::default();
        state.show_chat_info_loading(1, "Alice");
        state.set_chat_info_loaded(ChatInfoPopupState::Loaded(ChatInfo {
            title: "Alice".into(),
            chat_type: ChatType::Private,
            status_line: "online".into(),
            description: Some("Hello world".into()),
        }));
        match state.chat_info_popup().unwrap() {
            ChatInfoPopupState::Loaded(info) => {
                assert_eq!(info.status_line, "online");
                assert_eq!(info.description.as_deref(), Some("Hello world"));
            }
            _ => panic!("expected Loaded state"),
        }
    }

    #[test]
    fn command_popup_none_by_default() {
        let state = ShellState::default();
        assert!(state.command_popup().is_none());
    }

    #[test]
    fn open_command_popup_creates_state() {
        use crate::domain::command_popup_state::CommandPopupKind;
        let mut state = ShellState::default();
        state.open_command_popup("Recording", CommandPopupKind::Recording);
        assert!(state.command_popup().is_some());
        assert_eq!(state.command_popup().unwrap().title(), "Recording");
    }

    #[test]
    fn close_command_popup_clears_state() {
        use crate::domain::command_popup_state::CommandPopupKind;
        let mut state = ShellState::default();
        state.open_command_popup("Recording", CommandPopupKind::Recording);
        state.close_command_popup();
        assert!(state.command_popup().is_none());
    }

    #[test]
    fn command_popup_mut_allows_mutation() {
        use crate::domain::command_popup_state::CommandPopupKind;
        let mut state = ShellState::default();
        state.open_command_popup("Test", CommandPopupKind::Recording);
        state
            .command_popup_mut()
            .unwrap()
            .push_line("output".into());
        assert_eq!(
            state.command_popup().unwrap().visible_lines(20),
            vec!["output"]
        );
    }

    #[test]
    fn notification_none_by_default() {
        let state = ShellState::default();
        assert!(state.active_notification().is_none());
    }

    #[test]
    fn set_notification_makes_it_active() {
        let mut state = ShellState::default();
        state.set_notification("Copied to clipboard");
        assert_eq!(state.active_notification(), Some("Copied to clipboard"));
    }

    #[test]
    fn notification_expires_after_ttl() {
        let mut state = ShellState::default();
        let expired = Instant::now() - Duration::from_secs(5);
        state.set_notification_at("Old message", expired);
        assert!(state.active_notification().is_none());
    }

    #[test]
    fn fresh_notification_replaces_previous() {
        let mut state = ShellState::default();
        state.set_notification("First");
        state.set_notification("Second");
        assert_eq!(state.active_notification(), Some("Second"));
    }

    #[test]
    fn message_info_popup_none_by_default() {
        let state = ShellState::default();
        assert!(state.message_info_popup().is_none());
    }

    #[test]
    fn show_message_info_loading_creates_popup() {
        let mut state = ShellState::default();
        state.show_message_info_loading(1, 42);
        assert!(state.message_info_popup().is_some());
        assert_eq!(state.message_info_popup().unwrap().ids(), Some((1, 42)));
    }

    #[test]
    fn close_message_info_popup_clears_state() {
        let mut state = ShellState::default();
        state.show_message_info_loading(1, 42);
        state.close_message_info_popup();
        assert!(state.message_info_popup().is_none());
    }

    #[test]
    fn set_message_info_loaded_updates_popup() {
        use crate::domain::message_info_state::{MessageInfo, MessageInfoPopupState};
        let mut state = ShellState::default();
        state.show_message_info_loading(1, 42);
        state.set_message_info_loaded(MessageInfoPopupState::Loaded(MessageInfo {
            reactions: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: Some(1700000000),
        }));
        match state.message_info_popup().unwrap() {
            MessageInfoPopupState::Loaded(info) => {
                assert_eq!(info.edit_date, Some(1700000000));
            }
            _ => panic!("expected Loaded state"),
        }
    }

    #[test]
    fn reaction_picker_none_by_default() {
        let state = ShellState::default();
        assert!(state.reaction_picker().is_none());
    }

    #[test]
    fn show_reaction_picker_loading_creates_picker() {
        let mut state = ShellState::default();
        state.show_reaction_picker_loading(1, 42);
        assert!(state.reaction_picker().is_some());
        assert_eq!(state.reaction_picker().unwrap().ids(), Some((1, 42)));
    }

    #[test]
    fn close_reaction_picker_clears_state() {
        let mut state = ShellState::default();
        state.show_reaction_picker_loading(1, 42);
        state.close_reaction_picker();
        assert!(state.reaction_picker().is_none());
    }

    #[test]
    fn set_reaction_picker_updates_state() {
        use crate::domain::reaction_picker_state::{
            AvailableReaction, ReactionPickerData, ReactionPickerState,
        };
        let mut state = ShellState::default();
        state.show_reaction_picker_loading(1, 42);
        state.set_reaction_picker(ReactionPickerState::Ready(ReactionPickerData::new(
            vec![AvailableReaction {
                emoji: "👍".into(),
                needs_premium: false,
            }],
            1,
            42,
        )));
        match state.reaction_picker().unwrap() {
            ReactionPickerState::Ready(data) => {
                assert_eq!(data.items.len(), 1);
            }
            _ => panic!("expected Ready state"),
        }
    }

    #[test]
    fn reaction_picker_mut_allows_mutation() {
        use crate::domain::reaction_picker_state::{
            AvailableReaction, ReactionPickerData, ReactionPickerState,
        };
        let mut state = ShellState::default();
        state.set_reaction_picker(ReactionPickerState::Ready(ReactionPickerData::new(
            vec![
                AvailableReaction {
                    emoji: "👍".into(),
                    needs_premium: false,
                },
                AvailableReaction {
                    emoji: "❤".into(),
                    needs_premium: false,
                },
            ],
            1,
            42,
        )));
        if let Some(data) = state.reaction_picker_mut().and_then(|p| p.data_mut()) {
            data.select_next();
        }
        match state.reaction_picker().unwrap() {
            ReactionPickerState::Ready(data) => {
                assert_eq!(data.selected_index, 1);
            }
            _ => panic!("expected Ready state"),
        }
    }
}
