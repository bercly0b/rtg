use std::path::PathBuf;
use std::time::Duration;

use tdlib_rs::enums::AuthorizationState;

/// Polling interval when no TDLib updates are available.
/// 10ms provides responsive update handling without excessive CPU usage.
pub(super) const UPDATE_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Maximum size of TDLib's internal log file before automatic rotation.
/// TDLib creates a `.old` backup when this limit is reached.
pub(super) const TDLIB_LOG_MAX_SIZE: i64 = 10 * 1024 * 1024; // 10 MB

/// TDLib error code returned by `loadChats` when all chats are already cached.
///
/// This is a normal "nothing more to load" signal, not a real failure.
/// See TDLib docs: "Returns a 404 error if all chats have been loaded."
pub(super) const TDLIB_ERROR_ALL_CHATS_LOADED: i32 = 404;

/// Configuration for TDLib client initialization.
#[derive(Clone)]
pub struct TdLibConfig {
    /// Telegram API ID from <https://my.telegram.org>
    pub api_id: i32,
    /// Telegram API hash from <https://my.telegram.org>
    pub api_hash: String,
    /// Directory for TDLib's SQLite database
    pub database_directory: PathBuf,
    /// Directory for downloaded files
    pub files_directory: PathBuf,
    /// Path for TDLib's internal log file.
    ///
    /// TDLib (C++ library) has its own logger that writes to stderr by default,
    /// which corrupts the TUI alternate screen. This redirects it to a file.
    pub log_file: PathBuf,
    /// When true, raise TDLib's internal log verbosity (info/debug visible
    /// on stderr and in the log file). When false, suppress all but fatal
    /// startup logs and keep the log file at warning level. Mapped from the
    /// app's logging level via `LogConfig::is_verbose()`.
    pub verbose: bool,
}

// Custom Debug implementation to redact sensitive api_hash field.
impl std::fmt::Debug for TdLibConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TdLibConfig")
            .field("api_id", &self.api_id)
            .field("api_hash", &"[REDACTED]")
            .field("database_directory", &self.database_directory)
            .field("files_directory", &self.files_directory)
            .field("log_file", &self.log_file)
            .field("verbose", &self.verbose)
            .finish()
    }
}

/// Error types for TDLib operations.
#[derive(Debug, thiserror::Error)]
pub enum TdLibError {
    /// TDLib initialization error
    #[error("TDLib initialization error: {message}")]
    Init { message: String },

    /// TDLib request error (carries TDLib error code for discrimination)
    #[error("TDLib request error {code}: {message}")]
    Request { code: i32, message: String },

    /// TDLib shutdown error
    #[error("TDLib shutdown error: {message}")]
    Shutdown { message: String },

    /// TDLib timeout error
    #[error("TDLib operation timed out: {message}")]
    Timeout { message: String },
}

/// Authorization state change event.
#[derive(Debug, Clone)]
pub struct AuthStateUpdate {
    pub state: AuthorizationState,
}
