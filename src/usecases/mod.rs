//! Use case layer: application workflows and orchestration.

pub mod background;
pub mod bootstrap;
pub mod chat_lifecycle;
pub mod chat_subtitle;
pub mod context;
pub mod contracts;
pub mod guided_auth;
pub mod list_chats;
pub mod load_messages;
pub mod logout;
pub mod message_info;
pub mod pty;
pub mod send_message;
pub mod send_voice;
pub mod shell;
pub mod startup;
pub mod voice_recording;

/// Returns the usecases module name for smoke checks.
pub fn module_name() -> &'static str {
    "usecases"
}
