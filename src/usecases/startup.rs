use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use grammers_session::Session;

use crate::{
    infra::{error::AppError, storage_layout::StorageLayout},
    telegram::TelegramAdapter,
};

const POLICY_INVALID_MARKER: &str = "SESSION_POLICY_INVALID";
const DEFAULT_PROBE_TIMEOUT_MS: u64 = 1_500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupFlowState {
    LaunchTui,
    GuidedAuth { reason: GuidedAuthReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuidedAuthReason {
    Missing,
    Broken,
    Revoked,
    PolicyInvalid,
}

impl GuidedAuthReason {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Missing => "AUTH_SESSION_MISSING",
            Self::Broken => "AUTH_SESSION_BROKEN",
            Self::Revoked => "AUTH_SESSION_REVOKED",
            Self::PolicyInvalid => "AUTH_SESSION_POLICY_INVALID",
        }
    }

    pub fn user_message(&self) -> &'static str {
        match self {
            Self::Missing => "no saved session found",
            Self::Broken => "saved session is unreadable or corrupted",
            Self::Revoked => "saved session is no longer valid on Telegram",
            Self::PolicyInvalid => {
                "saved session is marked invalid locally and requires re-authorization"
            }
        }
    }
}

#[derive(Debug)]
pub struct SessionLockGuard {
    path: PathBuf,
}

impl Drop for SessionLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub struct StartupPlan {
    pub state: StartupFlowState,
    pub probe_warning: Option<&'static str>,
    _layout: StorageLayout,
    _lock_guard: SessionLockGuard,
}

impl StartupPlan {
    pub fn session_file(&self) -> PathBuf {
        self._layout.session_file()
    }
}

pub trait SessionProtocolProber {
    fn probe_session_protocol(
        &self,
        _session_file: &Path,
        _timeout: Duration,
    ) -> Result<ProtocolSessionValidity, ProtocolProbeError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolSessionValidity {
    Valid,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolProbeError {
    Timeout,
    Network,
}

impl SessionProtocolProber for TelegramAdapter {
    fn probe_session_protocol(
        &self,
        _session_file: &Path,
        _timeout: Duration,
    ) -> Result<ProtocolSessionValidity, ProtocolProbeError> {
        match std::env::var("RTG_STARTUP_PROBE_STUB").ok().as_deref() {
            Some("valid") => Ok(ProtocolSessionValidity::Valid),
            Some("revoked") => Ok(ProtocolSessionValidity::Revoked),
            Some("timeout") => Err(ProtocolProbeError::Timeout),
            _ => Err(ProtocolProbeError::Network),
        }
    }
}

pub fn plan_startup(
    telegram: &TelegramAdapter,
    probe_timeout_ms: Option<u64>,
) -> Result<StartupPlan, AppError> {
    let layout = StorageLayout::resolve()?;
    plan_startup_with_layout(&layout, telegram, probe_timeout_ms)
}

fn plan_startup_with_layout(
    layout: &StorageLayout,
    prober: &dyn SessionProtocolProber,
    probe_timeout_ms: Option<u64>,
) -> Result<StartupPlan, AppError> {
    layout.ensure_dirs()?;

    let lock_guard = acquire_session_lock(layout.session_lock_file())?;

    let (state, probe_warning) = evaluate_session_validity(layout, prober, probe_timeout_ms)?;

    Ok(StartupPlan {
        state,
        probe_warning,
        _layout: layout.clone(),
        _lock_guard: lock_guard,
    })
}

fn evaluate_session_validity(
    layout: &StorageLayout,
    prober: &dyn SessionProtocolProber,
    probe_timeout_ms: Option<u64>,
) -> Result<(StartupFlowState, Option<&'static str>), AppError> {
    if is_policy_invalid(layout)? {
        return Ok((
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::PolicyInvalid,
            },
            None,
        ));
    }

    let session_file = layout.session_file();

    match local_session_validity(&session_file) {
        LocalSessionValidity::Missing => Ok((
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Missing,
            },
            None,
        )),
        LocalSessionValidity::Broken => Ok((
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Broken,
            },
            None,
        )),
        LocalSessionValidity::Valid => {
            let timeout =
                Duration::from_millis(probe_timeout_ms.unwrap_or(DEFAULT_PROBE_TIMEOUT_MS));
            match prober.probe_session_protocol(&session_file, timeout) {
                Ok(ProtocolSessionValidity::Valid) => Ok((StartupFlowState::LaunchTui, None)),
                Ok(ProtocolSessionValidity::Revoked) => {
                    mark_policy_invalid(layout)?;
                    Ok((
                        StartupFlowState::GuidedAuth {
                            reason: GuidedAuthReason::Revoked,
                        },
                        None,
                    ))
                }
                Err(ProtocolProbeError::Timeout) => Ok((
                    StartupFlowState::LaunchTui,
                    Some("AUTH_PROBE_TIMEOUT_FALLBACK"),
                )),
                Err(ProtocolProbeError::Network) => Ok((
                    StartupFlowState::LaunchTui,
                    Some("AUTH_PROBE_NETWORK_FALLBACK"),
                )),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LocalSessionValidity {
    Missing,
    Broken,
    Valid,
}

fn local_session_validity(session_file: &Path) -> LocalSessionValidity {
    match Session::load_file(session_file) {
        Ok(session) if session.signed_in() => LocalSessionValidity::Valid,
        Ok(_) => LocalSessionValidity::Broken,
        Err(source) if source.kind() == ErrorKind::NotFound => LocalSessionValidity::Missing,
        Err(_) => LocalSessionValidity::Broken,
    }
}

fn policy_invalid_path(layout: &StorageLayout) -> PathBuf {
    layout.session_policy_invalid_file()
}

fn is_policy_invalid(layout: &StorageLayout) -> Result<bool, AppError> {
    let path = policy_invalid_path(layout);
    match fs::read_to_string(&path) {
        Ok(contents) => Ok(contents.trim() == POLICY_INVALID_MARKER),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(false),
        Err(source) => Err(AppError::SessionProbe { path, source }),
    }
}

fn mark_policy_invalid(layout: &StorageLayout) -> Result<(), AppError> {
    let path = policy_invalid_path(layout);
    let mut file = File::create(&path).map_err(|source| AppError::SessionProbe {
        path: path.clone(),
        source,
    })?;

    file.write_all(POLICY_INVALID_MARKER.as_bytes())
        .map_err(|source| AppError::SessionProbe { path, source })
}

fn acquire_session_lock(path: PathBuf) -> Result<SessionLockGuard, AppError> {
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(_) => Ok(SessionLockGuard { path }),
        Err(source) if source.kind() == ErrorKind::AlreadyExists => {
            Err(AppError::SessionStoreBusy { path })
        }
        Err(source) => Err(AppError::SessionLockCreate { path, source }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        fs::{self, create_dir_all},
        sync::{Arc, Mutex},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct StubSessionProber {
        outcome: Result<ProtocolSessionValidity, ProtocolProbeError>,
        captured_timeout: Arc<Mutex<Option<Duration>>>,
    }

    impl StubSessionProber {
        fn valid() -> Self {
            Self {
                outcome: Ok(ProtocolSessionValidity::Valid),
                captured_timeout: Arc::new(Mutex::new(None)),
            }
        }
    }

    impl SessionProtocolProber for StubSessionProber {
        fn probe_session_protocol(
            &self,
            _session_file: &Path,
            timeout: Duration,
        ) -> Result<ProtocolSessionValidity, ProtocolProbeError> {
            *self.captured_timeout.lock().expect("timeout lock") = Some(timeout);
            self.outcome.clone()
        }
    }

    fn make_layout() -> StorageLayout {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        let root = env::temp_dir().join(format!("rtg-startup-test-{suffix}"));

        StorageLayout {
            config_dir: root.clone(),
            session_dir: root.join("session"),
            cache_dir: root.join("cache"),
        }
    }

    fn write_signed_in_session(path: &Path) {
        let session = Session::load_file_or_create(path)
            .expect("session fixture file should be created before save");
        session.set_user(1, 1, false);
        session
            .save_to_file(path)
            .expect("signed-in session should be writable");
    }

    #[test]
    fn valid_session_and_probe_launch_tui() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");
        write_signed_in_session(&layout.session_file());

        let prober = StubSessionProber::valid();
        let plan = plan_startup_with_layout(&layout, &prober, Some(2500)).expect("startup plan");

        assert_eq!(plan.state, StartupFlowState::LaunchTui);
        assert_eq!(plan.probe_warning, None);
        assert_eq!(
            *prober.captured_timeout.lock().expect("timeout lock"),
            Some(Duration::from_millis(2500))
        );
    }

    #[test]
    fn missing_session_goes_to_guided_auth() {
        let layout = make_layout();
        let prober = StubSessionProber::valid();

        let plan = plan_startup_with_layout(&layout, &prober, None).expect("startup plan");

        assert_eq!(
            plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Missing
            }
        );
    }

    #[test]
    fn broken_session_goes_to_guided_auth() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");
        fs::write(layout.session_file(), b"").expect("broken empty session written");
        let prober = StubSessionProber::valid();

        let plan = plan_startup_with_layout(&layout, &prober, None).expect("startup plan");

        assert_eq!(
            plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Broken
            }
        );
    }

    #[test]
    fn legacy_marker_session_is_treated_as_broken_and_forces_reauth() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");
        fs::write(layout.session_file(), b"authorized").expect("legacy marker written");
        let prober = StubSessionProber::valid();

        let plan = plan_startup_with_layout(&layout, &prober, None).expect("startup plan");

        assert_eq!(
            plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Broken
            }
        );
    }

    #[test]
    fn revoked_protocol_session_goes_to_guided_auth_and_marks_policy_invalid() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");
        write_signed_in_session(&layout.session_file());

        let prober = StubSessionProber {
            outcome: Ok(ProtocolSessionValidity::Revoked),
            captured_timeout: Arc::new(Mutex::new(None)),
        };

        let plan = plan_startup_with_layout(&layout, &prober, None).expect("startup plan");

        assert_eq!(
            plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::Revoked
            }
        );
        assert!(layout.session_policy_invalid_file().exists());
    }

    #[test]
    fn policy_invalid_marker_short_circuits_to_guided_auth_without_probe() {
        let layout = make_layout();
        create_dir_all(&layout.session_dir).expect("session dir should exist");
        fs::write(layout.session_policy_invalid_file(), POLICY_INVALID_MARKER)
            .expect("policy marker should be written");

        let prober = StubSessionProber::valid();
        let plan = plan_startup_with_layout(&layout, &prober, None).expect("startup plan");

        assert_eq!(
            plan.state,
            StartupFlowState::GuidedAuth {
                reason: GuidedAuthReason::PolicyInvalid
            }
        );
        assert_eq!(*prober.captured_timeout.lock().expect("timeout lock"), None);
    }

    #[test]
    fn probe_timeout_falls_back_to_launch_tui_with_warning() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");
        write_signed_in_session(&layout.session_file());

        let prober = StubSessionProber {
            outcome: Err(ProtocolProbeError::Timeout),
            captured_timeout: Arc::new(Mutex::new(None)),
        };

        let plan = plan_startup_with_layout(&layout, &prober, None).expect("startup plan");

        assert_eq!(plan.state, StartupFlowState::LaunchTui);
        assert_eq!(plan.probe_warning, Some("AUTH_PROBE_TIMEOUT_FALLBACK"));
    }

    #[test]
    fn probe_network_error_falls_back_to_launch_tui_with_warning() {
        let layout = make_layout();
        layout.ensure_dirs().expect("dirs should be created");
        write_signed_in_session(&layout.session_file());

        let prober = StubSessionProber {
            outcome: Err(ProtocolProbeError::Network),
            captured_timeout: Arc::new(Mutex::new(None)),
        };

        let plan = plan_startup_with_layout(&layout, &prober, None).expect("startup plan");

        assert_eq!(plan.state, StartupFlowState::LaunchTui);
        assert_eq!(plan.probe_warning, Some("AUTH_PROBE_NETWORK_FALLBACK"));
    }
}
