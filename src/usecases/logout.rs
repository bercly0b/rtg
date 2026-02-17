use std::{fs, io::ErrorKind, path::Path};

use crate::{
    infra::error::AppError, infra::storage_layout::StorageLayout, telegram::TelegramAdapter,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogoutOutcome {
    pub session_removed: bool,
    pub policy_marker_removed: bool,
}

pub fn logout_and_reset(telegram: &mut TelegramAdapter) -> Result<LogoutOutcome, AppError> {
    let layout = StorageLayout::resolve()?;
    layout.ensure_dirs()?;

    let session_removed = remove_if_exists(&layout.session_file())?;
    let policy_marker_removed = remove_if_exists(&layout.session_policy_invalid_file())?;

    telegram.disconnect_and_reset();

    Ok(LogoutOutcome {
        session_removed,
        policy_marker_removed,
    })
}

fn remove_if_exists(path: &Path) -> Result<bool, AppError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(false),
        Err(source) => Err(AppError::SessionProbe {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    #[test]
    fn logout_removes_session_and_policy_marker() {
        let root = env::temp_dir().join(format!(
            "rtg-logout-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        ));
        let xdg = root.join("xdg");
        fs::create_dir_all(&xdg).expect("xdg dir should be creatable");

        let old_xdg = env::var_os("XDG_CONFIG_HOME");
        // SAFETY: unit test serially sets test-local env and restores it before exit.
        unsafe { env::set_var("XDG_CONFIG_HOME", &xdg) };

        let layout = StorageLayout::resolve().expect("layout should resolve");
        layout.ensure_dirs().expect("dirs should be created");
        fs::write(layout.session_file(), b"session").expect("session should be written");
        fs::write(
            layout.session_policy_invalid_file(),
            b"SESSION_POLICY_INVALID",
        )
        .expect("marker should be written");

        let mut adapter = TelegramAdapter::stub();
        let outcome = logout_and_reset(&mut adapter).expect("logout should succeed");

        assert!(outcome.session_removed);
        assert!(outcome.policy_marker_removed);
        assert!(!layout.session_file().exists());
        assert!(!layout.session_policy_invalid_file().exists());

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

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn logout_is_idempotent_when_no_files_exist() {
        let mut adapter = TelegramAdapter::stub();
        adapter.record_connectivity_status(crate::domain::events::ConnectivityStatus::Connected);

        let root = env::temp_dir().join(format!(
            "rtg-logout-test-empty-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        ));
        let xdg = root.join("xdg");
        fs::create_dir_all(&xdg).expect("xdg dir should be creatable");

        let old_xdg = env::var_os("XDG_CONFIG_HOME");
        // SAFETY: unit test serially sets test-local env and restores it before exit.
        unsafe { env::set_var("XDG_CONFIG_HOME", &xdg) };

        let outcome = logout_and_reset(&mut adapter).expect("logout should succeed");
        assert!(!outcome.session_removed);
        assert!(!outcome.policy_marker_removed);

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

        let _ = fs::remove_dir_all(root);
    }
}
