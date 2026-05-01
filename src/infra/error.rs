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
    #[error("failed to write config file at {path}: {source}")]
    ConfigWrite {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to serialize config file at {path}: {source}")]
    ConfigSerialize {
        path: PathBuf,
        #[source]
        source: toml_edit::TomlError,
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
    #[error("another rtg instance is already running (lock held at {path})")]
    InstanceBusy { path: PathBuf },
    #[error("failed to create instance lock file at {path}: {source}")]
    InstanceLockCreate {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to remove TDLib data at {path}: {source}")]
    TdlibDataCleanup {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
