//! TDLib client wrapper with lifecycle management.
//!
//! Provides a foundational TDLib client for RTG with:
//! - Client initialization with configuration parameters
//! - Update receiver loop for processing TDLib events
//! - Proper shutdown handling

mod auth;
mod chats;
mod messages;
pub mod types;
mod update_loop;

pub use types::{AuthStateUpdate, TdLibConfig, TdLibError};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use tokio::runtime::Runtime;

use super::tdlib_updates::TdLibUpdate;
use types::TDLIB_LOG_MAX_SIZE;

/// TDLib client with managed lifecycle.
///
/// This wrapper around `tdlib_rs` manages:
/// - Client ID allocation
/// - Dedicated async runtime for TDLib operations
/// - Background update receiver loop
/// - Authorization state channel
/// - Typed update events channel
/// - Proper shutdown via `close()` function
pub struct TdLibClient {
    client_id: i32,
    config: TdLibConfig,
    rt: Arc<Runtime>,
    auth_state_rx: Mutex<mpsc::Receiver<AuthStateUpdate>>,
    /// Receiver for typed TDLib updates. Wrapped in Option to allow taking.
    update_rx: Mutex<Option<mpsc::Receiver<TdLibUpdate>>>,
    /// Update loop thread handle. Kept alive for the client's lifetime.
    _update_thread: Option<thread::JoinHandle<()>>,
    is_closed: AtomicBool,
    /// Shared cache of Chat/User objects populated by the update loop.
    cache: super::tdlib_cache::TdLibCache,
}

impl TdLibClient {
    /// Creates a new TDLib client and starts the update receiver loop.
    ///
    /// This allocates a new TDLib client ID and spawns a background thread
    /// that continuously calls `tdlib_rs::receive()` to process updates.
    pub fn new(config: TdLibConfig) -> Result<Self, TdLibError> {
        let client_id = tdlib_rs::create_client();

        tracing::info!(
            client_id,
            database_dir = %config.database_directory.display(),
            "Created TDLib client"
        );

        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| TdLibError::Init {
                    message: format!("failed to create tokio runtime: {e}"),
                })?,
        );

        let (auth_state_tx, auth_state_rx) = mpsc::channel::<AuthStateUpdate>();
        let (update_tx, update_rx) = mpsc::channel::<TdLibUpdate>();
        let cache = super::tdlib_cache::TdLibCache::new();

        // Spawn update receiver thread (fully synchronous, no async runtime needed)
        let update_thread = {
            let cache = cache.clone();
            thread::spawn(move || {
                Self::run_update_loop(client_id, auth_state_tx, update_tx, cache);
            })
        };

        // Redirect TDLib's internal C++ logger from stderr to a file FIRST,
        // before any other request. TDLib (C++ library) writes logs to stderr
        // by default, which corrupts the TUI alternate screen (ratatui).
        // This also serves as the mandatory initial request that activates
        // TDLib's update delivery — without at least one request, `receive()`
        // will never return the initial `WaitTdlibParameters` state update.
        // See: tdlib-rs examples/get_me.rs and lib.rs documentation.
        let log_path = config
            .log_file
            .to_str()
            .ok_or_else(|| TdLibError::Init {
                message: "TDLib log file path is not valid UTF-8".to_owned(),
            })?
            .to_owned();
        rt.block_on(async {
            tdlib_rs::functions::set_log_stream(
                tdlib_rs::enums::LogStream::File(tdlib_rs::types::LogStreamFile {
                    path: log_path,
                    max_file_size: TDLIB_LOG_MAX_SIZE,
                    redirect_stderr: false,
                }),
                client_id,
            )
            .await
        })
        .map_err(|e| TdLibError::Init {
            message: format!("failed to redirect TDLib log stream to file: {}", e.message),
        })?;

        // Set TDLib internal log verbosity: 2 = warnings and errors only.
        rt.block_on(async { tdlib_rs::functions::set_log_verbosity_level(2, client_id).await })
            .map_err(|e| TdLibError::Init {
                message: format!("failed to set TDLib log verbosity: {}", e.message),
            })?;

        tracing::debug!(
            client_id,
            log_file = %config.log_file.display(),
            "TDLib client initialized, logs redirected to file"
        );

        Ok(Self {
            client_id,
            config,
            rt,
            auth_state_rx: Mutex::new(auth_state_rx),
            update_rx: Mutex::new(Some(update_rx)),
            _update_thread: Some(update_thread),
            is_closed: AtomicBool::new(false),
            cache,
        })
    }

    /// Takes the typed update receiver.
    ///
    /// This can only be called once - subsequent calls return None.
    /// Used by TelegramChatUpdatesMonitor to receive typed updates.
    pub fn take_update_receiver(&self) -> Option<mpsc::Receiver<TdLibUpdate>> {
        self.update_rx.lock().ok()?.take()
    }

    /// Returns the TDLib client ID for sending requests.
    #[allow(dead_code)]
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the configuration used to create this client.
    #[allow(dead_code)]
    pub fn config(&self) -> &TdLibConfig {
        &self.config
    }

    /// Returns the async runtime for executing TDLib operations.
    #[allow(dead_code)]
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.rt
    }

    /// Returns the shared TDLib cache populated by the update loop.
    pub fn cache(&self) -> &super::tdlib_cache::TdLibCache {
        &self.cache
    }

    /// Checks if the client has been closed.
    #[allow(dead_code)]
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Acquire)
    }

    /// Graceful shutdown: sends `close()` and marks client as closed.
    ///
    /// After calling this method, the client should not be used for any
    /// further operations. TDLib will flush all data to disk and send
    /// `AuthorizationStateClosed` update.
    #[allow(dead_code)]
    pub fn close(&self) -> Result<(), TdLibError> {
        // Use compare_exchange to atomically check and set is_closed
        // This prevents race conditions when close() is called concurrently
        if self
            .is_closed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            tracing::debug!(client_id = self.client_id, "Client already closed");
            return Ok(());
        }

        tracing::info!(client_id = self.client_id, "Closing TDLib client");

        let client_id = self.client_id;
        let result = self.rt.block_on(async {
            tdlib_rs::functions::close(client_id)
                .await
                .map_err(|e| TdLibError::Shutdown { message: e.message })
        });

        // Don't reset is_closed on failure - a failed close attempt leaves the
        // client in an undefined state. The client should be considered unusable
        // after any close attempt, successful or not.
        if result.is_err() {
            tracing::error!(
                client_id = self.client_id,
                "TDLib close failed - client is in undefined state"
            );
        } else {
            tracing::info!(client_id = self.client_id, "TDLib client closed");
        }

        result
    }
}

impl Drop for TdLibClient {
    fn drop(&mut self) {
        if !self.is_closed.load(Ordering::Acquire) {
            tracing::warn!(
                client_id = self.client_id,
                "TdLibClient dropped without calling close() - resources may not be properly released"
            );
        }
    }
}

#[cfg(test)]
mod tests;
