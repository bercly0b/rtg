use std::{
    fs::{self, OpenOptions},
    io::ErrorKind,
    path::PathBuf,
};

use crate::infra::{error::AppError, storage_layout::StorageLayout};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupFlowState {
    Startup,
    SessionLoad,
    SessionProbe,
    LaunchTui,
    GuidedAuth,
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
    pub layout: StorageLayout,
    pub lock_guard: SessionLockGuard,
    pub state: StartupFlowState,
}

pub fn plan_startup() -> Result<StartupPlan, AppError> {
    let layout = StorageLayout::resolve()?;
    layout.ensure_dirs()?;

    let lock_guard = acquire_session_lock(layout.session_lock_file())?;

    let state = if layout.session_file().exists() {
        StartupFlowState::LaunchTui
    } else {
        StartupFlowState::GuidedAuth
    };

    Ok(StartupPlan {
        layout,
        lock_guard,
        state,
    })
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

    #[test]
    fn startup_plan_resolves_layout() {
        let plan = plan_startup().expect("startup plan should be built");
        assert!(plan.layout.config_dir.ends_with("rtg"));
    }
}
