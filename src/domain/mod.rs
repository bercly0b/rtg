//! Domain layer: core entities and business rules.

pub mod chat;
pub mod chat_info_state;
pub mod chat_list_state;
pub mod chat_search_state;
pub mod chat_subtitle;
pub mod command_popup_state;
pub mod events;
pub mod help_content;
pub mod message;
pub mod message_cache;
pub mod message_info_state;
pub mod message_input_state;
pub mod open_chat_state;
pub mod open_defaults;
pub mod open_handler;
pub mod shell_state;
pub mod status;
pub mod typing_state;
pub mod voice_defaults;

/// Returns the domain module name for smoke checks.
pub fn module_name() -> &'static str {
    "domain"
}
