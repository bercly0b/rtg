use tracing_subscriber::EnvFilter;

use crate::infra::{config::LogConfig, error::AppError};

pub fn init(config: &LogConfig) -> Result<(), AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level)),
        )
        .with_target(true)
        .try_init()
        .map_err(AppError::LoggingInit)
}
