//! Infrastructure layer: adapters for config, storage, and OS integrations.

pub mod config;
pub mod error;
pub mod logging;

/// Returns the infra module name for smoke checks.
pub fn module_name() -> &'static str {
    "infra"
}
