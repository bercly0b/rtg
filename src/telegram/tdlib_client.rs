//! TDLib client wrapper with lifecycle management.
//!
//! Provides a foundational TDLib client for RTG with:
//! - Client initialization with configuration parameters
//! - Proper shutdown handling
//! - Basic update receiver loop structure

use std::path::PathBuf;

// Note: These types are prepared for Phase 3+ of TDLib migration.
// They are currently unused but will be used when auth flow is implemented.

/// Configuration for TDLib client initialization.
#[allow(dead_code)]
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
}

// Custom Debug implementation to redact sensitive api_hash field.
impl std::fmt::Debug for TdLibConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TdLibConfig")
            .field("api_id", &self.api_id)
            .field("api_hash", &"[REDACTED]")
            .field("database_directory", &self.database_directory)
            .field("files_directory", &self.files_directory)
            .finish()
    }
}

/// Error types for TDLib operations.
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum TdLibError {
    /// TDLib initialization error
    #[error("TDLib initialization error: {message}")]
    Init { message: String },

    /// TDLib request error
    #[error("TDLib request error: {message}")]
    Request { message: String },

    /// TDLib shutdown error
    #[error("TDLib shutdown error: {message}")]
    Shutdown { message: String },
}

/// TDLib client with managed lifecycle.
///
/// This is a foundational wrapper around `tdlib_rs` that manages:
/// - Client ID allocation
/// - Proper shutdown via `close()` function
///
/// # Example
///
/// ```ignore
/// let config = TdLibConfig {
///     api_id: 12345,
///     api_hash: "your_api_hash".into(),
///     database_directory: PathBuf::from("/tmp/tdlib"),
///     files_directory: PathBuf::from("/tmp/tdlib_files"),
/// };
///
/// let client = TdLibClient::new(config);
/// // ... use client ...
/// client.close().await?;
/// ```
#[allow(dead_code)]
pub struct TdLibClient {
    client_id: i32,
    #[allow(dead_code)]
    config: TdLibConfig,
    is_closed: bool,
}

#[allow(dead_code)]
impl TdLibClient {
    /// Creates a new TDLib client.
    ///
    /// This allocates a new TDLib client ID. To start receiving updates,
    /// you need to send at least one request (e.g., `set_tdlib_parameters`).
    ///
    /// Note: Full initialization with parameters will be implemented in
    /// the authentication phase (Phase 3).
    pub fn new(config: TdLibConfig) -> Self {
        let client_id = tdlib_rs::create_client();

        tracing::info!(
            client_id,
            database_dir = %config.database_directory.display(),
            "Created TDLib client"
        );

        Self {
            client_id,
            config,
            is_closed: false,
        }
    }

    /// Returns the TDLib client ID for sending requests.
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the configuration used to create this client.
    pub fn config(&self) -> &TdLibConfig {
        &self.config
    }

    /// Checks if the client has been closed.
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    /// Graceful shutdown: sends `close()` and marks client as closed.
    ///
    /// After calling this method, the client should not be used for any
    /// further operations. TDLib will flush all data to disk and send
    /// `AuthorizationStateClosed` update.
    pub async fn close(&mut self) -> Result<(), TdLibError> {
        if self.is_closed {
            tracing::debug!(client_id = self.client_id, "Client already closed");
            return Ok(());
        }

        tracing::info!(client_id = self.client_id, "Closing TDLib client");

        tdlib_rs::functions::close(self.client_id)
            .await
            .map_err(|e| TdLibError::Shutdown { message: e.message })?;

        self.is_closed = true;
        tracing::info!(client_id = self.client_id, "TDLib client closed");

        Ok(())
    }
}

impl Drop for TdLibClient {
    fn drop(&mut self) {
        if !self.is_closed {
            tracing::warn!(
                client_id = self.client_id,
                "TdLibClient dropped without calling close() - resources may not be properly released"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_stores_all_fields() {
        let config = TdLibConfig {
            api_id: 12345,
            api_hash: "test_hash".into(),
            database_directory: PathBuf::from("/tmp/test_db"),
            files_directory: PathBuf::from("/tmp/test_files"),
        };

        assert_eq!(config.api_id, 12345);
        assert_eq!(config.api_hash, "test_hash");
        assert_eq!(config.database_directory, PathBuf::from("/tmp/test_db"));
        assert_eq!(config.files_directory, PathBuf::from("/tmp/test_files"));
    }

    #[test]
    fn new_client_is_not_closed() {
        let config = TdLibConfig {
            api_id: 12345,
            api_hash: "test_hash".into(),
            database_directory: PathBuf::from("/tmp/test_db"),
            files_directory: PathBuf::from("/tmp/test_files"),
        };

        let client = TdLibClient::new(config);
        assert!(!client.is_closed());
        assert!(client.client_id() > 0);
    }

    #[test]
    fn client_exposes_config() {
        let config = TdLibConfig {
            api_id: 99999,
            api_hash: "my_hash".into(),
            database_directory: PathBuf::from("/data/db"),
            files_directory: PathBuf::from("/data/files"),
        };

        let client = TdLibClient::new(config);
        assert_eq!(client.config().api_id, 99999);
        assert_eq!(client.config().api_hash, "my_hash");
    }

    // Note: Full async tests for close() require a running TDLib event loop
    // to process responses. These will be added in Phase 3 when the update
    // receiver loop is implemented. For now, we test the synchronous parts
    // of the close() logic.

    #[test]
    fn close_on_already_closed_returns_ok_immediately() {
        let config = TdLibConfig {
            api_id: 12345,
            api_hash: "test_hash".into(),
            database_directory: PathBuf::from("/tmp/test_idempotent_db"),
            files_directory: PathBuf::from("/tmp/test_idempotent_files"),
        };

        let mut client = TdLibClient::new(config);

        // Simulate already closed state
        client.is_closed = true;

        // Use a minimal runtime just to call the async function
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        let result = rt.block_on(client.close());

        // Should return Ok immediately without sending any TDLib request
        assert!(result.is_ok());
        assert!(client.is_closed());
    }
}
