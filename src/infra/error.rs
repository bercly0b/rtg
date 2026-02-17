use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to read config file at {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file at {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid configuration [{code}]: {details}")]
    ConfigValidation { code: &'static str, details: String },
    #[error("failed to initialize logging: {0}")]
    LoggingInit(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("failed to resolve storage paths: {details}")]
    StoragePathResolution { details: String },
    #[error("failed to create storage directory at {path}: {source}")]
    StorageDirCreate {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("session store is busy (another rtg instance may be running): {path}")]
    SessionStoreBusy { path: PathBuf },
    #[error("failed to create session lock file at {path}: {source}")]
    SessionLockCreate {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to probe session file at {path}: {source}")]
    SessionProbe {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
