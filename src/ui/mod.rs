//! UI layer: rendering and interaction entry points (CLI/TUI).

pub mod shell;

/// Returns the UI module name for smoke checks.
pub fn module_name() -> &'static str {
    "ui"
}
