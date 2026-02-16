//! UI layer: rendering and interaction entry points (CLI/TUI).

mod event_source;
pub mod shell;
mod terminal;
mod view;

/// Returns the UI module name for smoke checks.
pub fn module_name() -> &'static str {
    "ui"
}
