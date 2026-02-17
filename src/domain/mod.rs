//! Domain layer: core entities and business rules.

pub mod chat;
pub mod events;
pub mod shell_state;
pub mod status;

/// Returns the domain module name for smoke checks.
pub fn module_name() -> &'static str {
    "domain"
}
