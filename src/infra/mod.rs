//! Infrastructure layer: adapters for config, storage, and OS integrations.

pub mod config;
pub mod contracts;
pub mod error;
pub mod logging;
pub mod storage_layout;
pub mod stubs;

/// Returns the infra module name for smoke checks.
pub fn module_name() -> &'static str {
    "infra"
}
