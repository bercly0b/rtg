//! Use case layer: application workflows and orchestration.

pub mod bootstrap;
pub mod context;
pub mod contracts;
pub mod guided_auth;
pub mod list_chats;
pub mod load_messages;
pub mod logout;
pub mod send_message;
pub mod shell;
pub mod startup;

/// Returns the usecases module name for smoke checks.
pub fn module_name() -> &'static str {
    "usecases"
}
