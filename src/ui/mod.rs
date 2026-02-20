//! UI layer: rendering and interaction entry points (CLI/TUI).

mod event_source;
mod message_input;
mod message_rendering;
pub mod shell;
mod styles;
mod terminal;
mod view;

pub(crate) use event_source::{
    ChannelChatUpdatesSignalSource, ChannelConnectivityStatusSource, CrosstermEventSource,
};

/// Returns the UI module name for smoke checks.
pub fn module_name() -> &'static str {
    "ui"
}
