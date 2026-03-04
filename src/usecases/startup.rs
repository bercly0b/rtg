use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
};

use fs2::FileExt;

use crate::{
    infra::{error::AppError, storage_layout::StorageLayout},
    telegram::TelegramAdapter,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupFlowState {
    LaunchTui,
    GuidedAuth { reason: GuidedAuthReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuidedAuthReason {
    Missing,
}

impl GuidedAuthReason {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Missing => "AUTH_SESSION_MISSING",
        }
    }

    pub fn user_message(&self) -> &'static str {
        match self {
            Self::Missing => "no TDLib session found — authentication required",
        }
    }
}

/// Holds an OS-level advisory lock (`flock`) on the instance lock file.
///
/// The lock is automatically released by the OS when the file handle is
/// dropped — even if the process is killed with SIGKILL. This eliminates
/// stale-lock problems that occur with file-existence-based locking.
///
/// **Caveat:** Advisory locks may not be enforced across hosts on network
/// filesystems (NFS, SMB). The data directory should reside on a local
/// filesystem for reliable mutual exclusion.
#[derive(Debug)]
pub struct InstanceLockGuard {
    _file: File,
}

pub struct StartupPlan {
    pub state: StartupFlowState,
    _layout: StorageLayout,
    _lock_guard: InstanceLockGuard,
}

pub fn plan_startup(_telegram: &TelegramAdapter) -> Result<StartupPlan, AppError> {
    let layout = StorageLayout::resolve()?;
    plan_startup_with_layout(&layout)
}

fn plan_startup_with_layout(layout: &StorageLayout) -> Result<StartupPlan, AppError> {
    layout.ensure_dirs()?;

    let lock_guard = acquire_instance_lock(layout.instance_lock_file())?;

    let state = evaluate_session_state(layout);

    Ok(StartupPlan {
        state,
        _layout: layout.clone(),
        _lock_guard: lock_guard,
    })
}

/// Determines startup flow based on TDLib session presence.
///
/// If a TDLib database directory exists with files, we assume a previous
/// session was established and launch the TUI. TDLib will validate the
/// session internally and handle re-authentication if needed.
///
/// If no database exists, we route to guided auth.
fn evaluate_session_state(layout: &StorageLayout) -> StartupFlowState {
    if layout.tdlib_session_exists() {
        StartupFlowState::LaunchTui
    } else {
        StartupFlowState::GuidedAuth {
            reason: GuidedAuthReason::Missing,
        }
    }
}

fn acquire_instance_lock(path: PathBuf) -> Result<InstanceLockGuard, AppError> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|source| AppError::InstanceLockCreate {
            path: path.clone(),
            source,
        })?;

    file.try_lock_exclusive().map_err(|source| {
        if source.kind() == fs2::lock_contended_error().kind() {
            AppError::InstanceBusy { path }
        } else {
            AppError::InstanceLockCreate { path, source }
        }
    })?;

    Ok(InstanceLockGuard { _file: file })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_support::env_lock, usecases::logout::logout_and_reset};
    use std::{
        env,
        fs::{self},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn make_layout() -> StorageLayout {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        let root = env::temp_dir().join(format!("rtg-startup-test-{suffix}"));

        StorageLayout {
            config_dir: root.clone(),
            cache_dir: root.join("cache"),
        }
    }

    /// Creates a fake TDLib session by populating the database directory.
    fn write_tdlib_session(layout: &StorageLayout) {
        let db_dir = layout.tdlib_database_dir();
        fs::create_dir_all(&db_dir).expect("tdlib db dir should be creatable");
        fs::write(db_dir.join("td.binlog"), b"fake-session-data")
            .expect("fake tdlib session should be writable");
    }

    #[test]
    fn valid_tdlib_session_launches_tui() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");
        write_tdlib_session(&layout);

        let plan = plan_startup_with_layout(&layout).expect("startup plan");

        assert_eq!(plan.state, StartupFlowState::LaunchTui);
    }

    #[test]
    fn e2e_restart_reconnect_reuses_persisted_session() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");
        write_tdlib_session(&layout);

        let first_plan = plan_startup_with_layout(&layout).expect("first startup");
        assert_eq!(first_plan.state, StartupFlowState::LaunchTui);

        drop(first_plan);

        let second_plan = plan_startup_with_layout(&layout).expect("second startup");
        assert_eq!(second_plan.state, StartupFlowState::LaunchTui);
    }

    #[test]
    fn missing_session_goes_to_guided_auth() {
        let layout = make_layout();

        let plan = plan_startup_with_layout(&layout).expect("startup plan");

        assert_eq!(
            plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Missing
            }
        );
    }

    #[test]
    fn empty_tdlib_dir_goes_to_guided_auth() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");
        // Create database dir but leave it empty
        fs::create_dir_all(layout.tdlib_database_dir()).expect("tdlib db dir should be creatable");

        let plan = plan_startup_with_layout(&layout).expect("startup plan");

        assert_eq!(
            plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Missing
            }
        );
    }

    #[test]
    fn guided_auth_then_next_startup_launches_tui() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");

        let auth_plan = plan_startup_with_layout(&layout).expect("startup plan");

        assert_eq!(
            auth_plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Missing
            }
        );

        drop(auth_plan);

        // Simulate successful auth by creating TDLib session
        write_tdlib_session(&layout);

        let next_plan = plan_startup_with_layout(&layout).expect("startup");
        assert_eq!(next_plan.state, StartupFlowState::LaunchTui);
    }

    #[test]
    fn logout_reset_results_in_disconnected_state_and_clean_relogin_path() {
        let _guard = env_lock();

        let root = env::temp_dir().join(format!(
            "rtg-logout-relogin-startup-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
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
        write_tdlib_session(&layout);

        let mut adapter = TelegramAdapter::stub();
        let outcome = logout_and_reset(&mut adapter).expect("logout should succeed");
        assert!(outcome.tdlib_data_removed);
        assert!(!layout.tdlib_session_exists());

        let snapshot = adapter.status_snapshot();
        assert_eq!(snapshot.auth, crate::domain::status::AuthStatus::NotStarted);
        assert_eq!(
            snapshot.connectivity,
            crate::domain::status::ConnectivityHealth::Unavailable
        );

        let plan = plan_startup_with_layout(&layout).expect("startup plan");
        assert_eq!(
            plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Missing
            }
        );

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
    fn stale_lock_file_on_disk_does_not_block_startup() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");

        // Simulate a stale lock file left behind by a crashed process.
        // With advisory locking, the file exists but no flock is held.
        fs::write(layout.instance_lock_file(), b"").expect("stale lock file should be writable");
        assert!(layout.instance_lock_file().exists());

        let plan = plan_startup_with_layout(&layout)
            .expect("startup should succeed despite stale lock file on disk");

        assert_eq!(
            plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Missing
            }
        );
    }

    #[test]
    fn held_lock_blocks_second_acquisition() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");

        let first_guard = acquire_instance_lock(layout.instance_lock_file())
            .expect("first lock should be acquired");

        let second_result = acquire_instance_lock(layout.instance_lock_file());
        match second_result {
            Err(AppError::InstanceBusy { .. }) => {} // expected
            other => panic!("expected InstanceBusy, got: {other:?}"),
        }

        drop(first_guard);

        let third_guard = acquire_instance_lock(layout.instance_lock_file());
        assert!(
            third_guard.is_ok(),
            "lock should be acquirable after first guard is dropped"
        );
    }
}
