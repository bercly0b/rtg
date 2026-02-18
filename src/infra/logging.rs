use std::sync::OnceLock;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::infra::{config::LogConfig, error::AppError, storage_layout::StorageLayout};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init(config: &LogConfig) -> Result<(), AppError> {
    let layout = StorageLayout::resolve()?;
    std::fs::create_dir_all(&layout.config_dir).map_err(|source| AppError::StorageDirCreate {
        path: layout.config_dir.clone(),
        source,
    })?;

    let log_path = layout.config_dir.join("rtg.log");
    let file_appender = tracing_appender::rolling::never(&layout.config_dir, "rtg.log");
    let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level)),
        )
        .with_ansi(false)
        .with_target(true)
        .with_writer(non_blocking_writer)
        .try_init()
        .map_err(AppError::LoggingInit)?;

    tracing::info!(log_path = %log_path.display(), "file logging initialized");
    Ok(())
}
