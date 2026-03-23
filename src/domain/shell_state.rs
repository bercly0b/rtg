use std::time::{Duration, Instant};

use super::{
    chat::ChatSummary, chat_list_state::ChatListState, command_popup_state::CommandPopupState,
    events::ConnectivityStatus, message_cache::MessageCache,
    message_input_state::MessageInputState, open_chat_state::OpenChatState,
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
}
