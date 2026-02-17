//! UI layer: rendering and interaction entry points (CLI/TUI).

mod event_source;
pub mod shell;
mod terminal;
mod view;

pub(crate) use event_source::{ChannelConnectivityStatusSource, CrosstermEventSource};

/// Returns the UI module name for smoke checks.
pub fn module_name() -> &'static str {
    "ui"
}
