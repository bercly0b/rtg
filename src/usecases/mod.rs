//! Use case layer: application workflows and orchestration.

pub mod bootstrap;
pub mod context;
pub mod contracts;
pub mod guided_auth;
pub mod shell;
pub mod startup;

/// Returns the usecases module name for smoke checks.
pub fn module_name() -> &'static str {
    "usecases"
}
