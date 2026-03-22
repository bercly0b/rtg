//! UI layer: rendering and interaction entry points (CLI/TUI).

pub(crate) mod chat_message_list;
mod event_source;
mod help_popup;
mod message_input;
mod message_rendering;
pub mod shell;
mod styles;
mod terminal;
mod view;

pub(crate) use event_source::{
    ChannelBackgroundResultSource, ChannelChatUpdatesSignalSource, ChannelConnectivityStatusSource,
    CrosstermEventSource, StubChatUpdatesSignalSource, StubConnectivityStatusSource,
};

/// Returns the UI module name for smoke checks.
pub fn module_name() -> &'static str {
    "ui"
}
