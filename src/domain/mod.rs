//! Domain layer: core entities and business rules.

pub mod chat;
pub mod chat_list_state;
pub mod events;
pub mod message;
pub mod message_input_state;
pub mod open_chat_state;
pub mod shell_state;
pub mod status;

/// Returns the domain module name for smoke checks.
pub fn module_name() -> &'static str {
    "domain"
}
