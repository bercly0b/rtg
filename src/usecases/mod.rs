//! Use case layer: application workflows and orchestration.

pub mod bootstrap;
pub mod context;

/// Returns the usecases module name for smoke checks.
pub fn module_name() -> &'static str {
    "usecases"
}
