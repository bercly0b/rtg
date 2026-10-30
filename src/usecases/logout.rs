use std::{fs, io::ErrorKind, path::Path};

use crate::{infra::error::AppError, infra::storage_layout::StorageLayout};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogoutOutcome {
    pub tdlib_data_removed: bool,
}

/// Telegram session that must be shut down before its local data is wiped.
pub trait LogoutSession {
    /// Closes TDLib, blocking until it reports `AuthorizationState::Closed`,
    /// and resets the reported auth/connectivity status.
    fn disconnect_and_reset(&mut self) -> Result<(), AppError>;
}

/// Closes the Telegram session and only then removes TDLib data, so TDLib
/// never writes into (or holds open) files that are being deleted.
pub fn logout_and_reset(telegram: &mut dyn LogoutSession) -> Result<LogoutOutcome, AppError> {
    telegram.disconnect_and_reset()?;

    let layout = StorageLayout::resolve()?;
    layout.ensure_dirs()?;

    let tdlib_data_removed = remove_tdlib_data(&layout)?;

    Ok(LogoutOutcome { tdlib_data_removed })
}

/// Removes TDLib database and files directories.
///
/// Returns `true` if any TDLib data was actually removed.
fn remove_tdlib_data(layout: &StorageLayout) -> Result<bool, AppError> {
    let db_removed = remove_dir_if_exists(&layout.tdlib_database_dir())?;
    let files_removed = remove_dir_if_exists(&layout.tdlib_files_dir())?;
    Ok(db_removed || files_removed)
}

fn remove_dir_if_exists(path: &Path) -> Result<bool, AppError> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(true),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(false),
        Err(source) => Err(AppError::TdlibDataCleanup {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;
    use crate::{telegram::TelegramAdapter, test_support::env_lock};

    #[test]
    fn logout_removes_tdlib_data() {
        let _guard = env_lock();

        let root = tempfile::tempdir().expect("temp dir should be creatable");
        let xdg = root.path().join("xdg");
        fs::create_dir_all(&xdg).expect("xdg dir should be creatable");

        let old_xdg = env::var_os("XDG_CONFIG_HOME");
        // SAFETY: unit test serially sets test-local env and restores it before exit.
        unsafe { env::set_var("XDG_CONFIG_HOME", &xdg) };

        let layout = StorageLayout::resolve().expect("layout should resolve");
        layout.ensure_dirs().expect("dirs should be created");

        // Create fake TDLib data
        let db_dir = layout.tdlib_database_dir();
        let files_dir = layout.tdlib_files_dir();
        fs::create_dir_all(&db_dir).expect("db dir should be creatable");
        fs::create_dir_all(&files_dir).expect("files dir should be creatable");
        fs::write(db_dir.join("td.binlog"), b"data").expect("fake data should be writable");
        fs::write(files_dir.join("photo.jpg"), b"photo").expect("fake file should be writable");

        let mut adapter = TelegramAdapter::stub();
        let outcome = logout_and_reset(&mut adapter).expect("logout should succeed");

        assert!(outcome.tdlib_data_removed);
        assert!(!db_dir.exists());
        assert!(!files_dir.exists());

        let snapshot = adapter.status_snapshot();
        assert_eq!(snapshot.auth.as_label(), "AUTH_NOT_STARTED");
        assert_eq!(snapshot.connectivity.as_label(), "CONNECTIVITY_UNAVAILABLE");

        match old_xdg {
            Some(value) => {
                // SAFETY: restoring process env in test teardown.
                unsafe { env::set_var("XDG_CONFIG_HOME", value) }
            }
            None => {
                // SAFETY: restoring process env in test teardown.
                unsafe { env::remove_var("XDG_CONFIG_HOME") }
            }
        }
    }

    #[test]
    fn logout_is_idempotent_when_no_data_exists() {
        let _guard = env_lock();

        let mut adapter = TelegramAdapter::stub();
        adapter.record_connectivity_status(crate::domain::events::ConnectivityStatus::Connected);

        let root = tempfile::tempdir().expect("temp dir should be creatable");
        let xdg = root.path().join("xdg");
        fs::create_dir_all(&xdg).expect("xdg dir should be creatable");

        let old_xdg = env::var_os("XDG_CONFIG_HOME");
        // SAFETY: unit test serially sets test-local env and restores it before exit.
        unsafe { env::set_var("XDG_CONFIG_HOME", &xdg) };

        let outcome = logout_and_reset(&mut adapter).expect("logout should succeed");
        assert!(!outcome.tdlib_data_removed);

        let snapshot = adapter.status_snapshot();
        assert_eq!(snapshot.auth.as_label(), "AUTH_NOT_STARTED");
        assert_eq!(snapshot.connectivity.as_label(), "CONNECTIVITY_UNAVAILABLE");

        match old_xdg {
            Some(value) => {
                // SAFETY: restoring process env in test teardown.
                unsafe { env::set_var("XDG_CONFIG_HOME", value) }
            }
            None => {
                // SAFETY: restoring process env in test teardown.
                unsafe { env::remove_var("XDG_CONFIG_HOME") }
            }
        }
    }

    /// Runs `test` with `XDG_CONFIG_HOME` pointed at a fresh temp dir that
    /// already holds a fake TDLib database.
    fn with_tdlib_data(test: impl FnOnce(&StorageLayout)) {
        let _guard = env_lock();
        let xdg = tempfile::tempdir().expect("temp dir");

        let old_xdg = env::var_os("XDG_CONFIG_HOME");
        // SAFETY: env is guarded by process-wide test mutex.
        unsafe { env::set_var("XDG_CONFIG_HOME", xdg.path()) };

        let layout = StorageLayout::resolve().expect("layout should resolve");
        layout.ensure_dirs().expect("dirs should be created");
        let db_dir = layout.tdlib_database_dir();
        fs::create_dir_all(&db_dir).expect("db dir should be creatable");
        fs::write(db_dir.join("td.binlog"), b"data").expect("fake data should be writable");

        test(&layout);

        match old_xdg {
            Some(value) => {
                // SAFETY: restoring env while guard is held.
                unsafe { env::set_var("XDG_CONFIG_HOME", value) }
            }
            None => {
                // SAFETY: restoring env while guard is held.
                unsafe { env::remove_var("XDG_CONFIG_HOME") }
            }
        }
    }

    /// Records whether TDLib data was still on disk when TDLib was closed.
    struct RecordingSession {
        tdlib_db_dir: std::path::PathBuf,
        data_present_at_close: Option<bool>,
    }

    impl LogoutSession for RecordingSession {
        fn disconnect_and_reset(&mut self) -> Result<(), AppError> {
            self.data_present_at_close = Some(self.tdlib_db_dir.exists());
            Ok(())
        }
    }

    struct FailingSession;

    impl LogoutSession for FailingSession {
        fn disconnect_and_reset(&mut self) -> Result<(), AppError> {
            Err(AppError::TdlibClose {
                details: "timed out waiting for Closed".to_owned(),
            })
        }
    }

    #[test]
    fn logout_closes_tdlib_before_removing_its_data() {
        with_tdlib_data(|layout| {
            let mut session = RecordingSession {
                tdlib_db_dir: layout.tdlib_database_dir(),
                data_present_at_close: None,
            };

            let outcome = logout_and_reset(&mut session).expect("logout should succeed");

            assert_eq!(session.data_present_at_close, Some(true));
            assert!(outcome.tdlib_data_removed);
            assert!(!layout.tdlib_database_dir().exists());
        });
    }

    #[test]
    fn logout_keeps_tdlib_data_when_tdlib_does_not_close() {
        with_tdlib_data(|layout| {
            let error = logout_and_reset(&mut FailingSession)
                .expect_err("logout must fail when TDLib does not close");

            assert!(matches!(error, AppError::TdlibClose { .. }), "got: {error}");
            assert!(layout.tdlib_database_dir().join("td.binlog").exists());
        });
    }
}
