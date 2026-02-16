//! Infrastructure layer: adapters for config, storage, and OS integrations.

/// Returns the infra module name for smoke checks.
pub fn module_name() -> &'static str {
    "infra"
}
