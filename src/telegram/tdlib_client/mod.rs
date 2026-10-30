//! TDLib client wrapper with lifecycle management.
//!
//! Provides a foundational TDLib client for RTG with:
//! - Client initialization with configuration parameters
//! - Update receiver loop for processing TDLib events
//! - Proper shutdown handling

mod auth;
mod chats;
mod log_setup;
mod messages;
mod request_guard;
mod shutdown;
mod types;
mod update_loop;

pub use types::{AuthStateUpdate, TdLibConfig, TdLibError};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use tokio::runtime::Runtime;

use super::tdlib_updates::TdLibUpdate;
use crate::domain::events::ConnectivityStatus;
use types::{CLOSE_TIMEOUT, TDLIB_LOG_MAX_SIZE};

/// TDLib client with managed lifecycle.
///
/// This wrapper around `tdlib_rs` manages:
/// - Client ID allocation
/// - Dedicated async runtime for TDLib operations
/// - Background update receiver loop
/// - Authorization state channel
/// - Typed update events channel
/// - Proper shutdown via `close()` function
///
/// # Thread safety
///
/// The client is `Send + Sync` and is shared between the UI thread and
/// background task threads (through `Arc<TelegramAdapter>`):
///
/// - Request methods take `&self` and block the calling thread on the client's
///   own runtime until TDLib answers. Concurrent requests from different
///   threads are independent: `tdlib-rs` matches every response to its request
///   by `@extra`. They must not be called from an async context — see
///   `block_on_request`.
/// - Only the update loop thread calls `tdlib_rs::receive()`; it writes to the
///   shared cache and the update channels, and delivers request responses.
/// - Authorization states are read under a mutex, so one thread at a time
///   consumes them. The update and connectivity receivers can each be taken
///   once.
/// - `close()` may be called from any thread and more than once: only the
///   first call shuts TDLib down.
pub struct TdLibClient {
    client_id: i32,
    config: TdLibConfig,
    rt: Arc<Runtime>,
    auth_state_rx: Mutex<mpsc::Receiver<AuthStateUpdate>>,
    /// Receiver for typed TDLib updates. Wrapped in Option to allow taking.
    update_rx: Mutex<Option<mpsc::Receiver<TdLibUpdate>>>,
    /// Receiver for TDLib connection state updates mapped to domain status.
    /// Wrapped in Option to allow taking once.
    connectivity_rx: Mutex<Option<mpsc::Receiver<ConnectivityStatus>>>,
    /// Update loop thread handle, joined by `close()` once TDLib reports `Closed`.
    update_thread: Mutex<Option<thread::JoinHandle<()>>>,
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
        // Suppress (or expose) TDLib's stderr noise BEFORE creating the
        // client. `tdlib_rs::functions::set_log_verbosity_level` requires a
        // client_id and is async — it cannot mute the startup burst that
        // happens during `create_client`. `td_execute` is the synchronous,
        // client-less entry point for log configuration.
        let initial_verbosity = if config.verbose { 5 } else { 0 };
        log_setup::set_global_verbosity(initial_verbosity);

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
        let (connectivity_tx, connectivity_rx) = mpsc::channel::<ConnectivityStatus>();
        let cache = super::tdlib_cache::TdLibCache::new();

        // Spawn update receiver thread (fully synchronous, no async runtime needed)
        let update_thread = {
            let cache = cache.clone();
            thread::spawn(move || {
                Self::run_update_loop(client_id, auth_state_tx, update_tx, connectivity_tx, cache);
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

        // After the file log stream is in place, set the file logger's
        // verbosity. Quiet mode keeps it at 2 (warnings + errors); verbose
        // mode keeps the global level at 5 so the file captures everything.
        let file_verbosity = if config.verbose { 5 } else { 2 };
        rt.block_on(async {
            tdlib_rs::functions::set_log_verbosity_level(file_verbosity, client_id).await
        })
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
            connectivity_rx: Mutex::new(Some(connectivity_rx)),
            update_thread: Mutex::new(Some(update_thread)),
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

    /// Takes the connectivity status receiver.
    ///
    /// Carries domain `ConnectivityStatus` events derived from TDLib's
    /// `Update::ConnectionState`. Can only be called once.
    pub fn take_connectivity_receiver(&self) -> Option<mpsc::Receiver<ConnectivityStatus>> {
        self.connectivity_rx.lock().ok()?.take()
    }

    /// Returns the TDLib client ID for sending requests.
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the async runtime for executing TDLib operations.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.rt
    }

    /// Returns the shared TDLib cache populated by the update loop.
    pub fn cache(&self) -> &super::tdlib_cache::TdLibCache {
        &self.cache
    }

    /// Graceful shutdown: sends `close`, waits for `AuthorizationState::Closed`
    /// and joins the update loop thread.
    ///
    /// After calling this method, the client should not be used for any
    /// further operations. Repeated calls are no-ops.
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

        let result = self.shut_down();

        // Don't reset is_closed on failure - a failed close attempt leaves the
        // client in an undefined state. The client should be considered unusable
        // after any close attempt, successful or not.
        match &result {
            Ok(()) => tracing::info!(client_id = self.client_id, "TDLib client closed"),
            Err(error) => tracing::error!(
                client_id = self.client_id,
                error = %error,
                "TDLib close failed - client is in undefined state"
            ),
        }

        result
    }

    fn shut_down(&self) -> Result<(), TdLibError> {
        let client_id = self.client_id;
        // The `ok` reply is delivered by the update loop, which is already gone
        // if TDLib closed on its own (e.g. the session was terminated remotely).
        // `Closed` is the completion signal, so the reply is not awaited.
        self.rt.spawn(async move {
            if let Err(error) = tdlib_rs::functions::close(client_id).await {
                tracing::error!(client_id, error = %error.message, "TDLib rejected close request");
            }
        });

        let auth_state_rx = self
            .auth_state_rx
            .lock()
            .map_err(|_| TdLibError::Shutdown {
                message: "auth state receiver lock poisoned".to_owned(),
            })?;
        let update_thread = self
            .update_thread
            .lock()
            .map_err(|_| TdLibError::Shutdown {
                message: "update thread lock poisoned".to_owned(),
            })?
            .take();

        shutdown::await_closed_and_join(&auth_state_rx, update_thread, CLOSE_TIMEOUT)
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
