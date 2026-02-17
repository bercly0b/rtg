use std::{env, fs, path::PathBuf};

use crate::infra::error::AppError;

const APP_DIR_NAME: &str = "rtg";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageLayout {
    pub config_dir: PathBuf,
    pub session_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl StorageLayout {
    pub fn resolve() -> Result<Self, AppError> {
        let config_base = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".config")))
            .ok_or_else(|| AppError::StoragePathResolution {
                details: "unable to resolve config base directory (XDG_CONFIG_HOME/HOME)".into(),
            })?;

        let config_dir = config_base.join(APP_DIR_NAME);
        let session_dir = config_dir.join("session");
        let cache_dir = config_dir.join("cache");

        Ok(Self {
            config_dir,
            session_dir,
            cache_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<(), AppError> {
        for dir in [&self.config_dir, &self.session_dir, &self.cache_dir] {
            fs::create_dir_all(dir).map_err(|source| AppError::StorageDirCreate {
                path: dir.clone(),
                source,
            })?;
        }

        Ok(())
    }

    pub fn session_file(&self) -> PathBuf {
        self.session_dir.join("session.dat")
    }

    pub fn session_lock_file(&self) -> PathBuf {
        self.session_dir.join("session.lock")
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_and_cache_are_under_config_dir() {
        let layout = StorageLayout::resolve().expect("layout should resolve");

        assert!(layout.session_dir.starts_with(&layout.config_dir));
        assert!(layout.cache_dir.starts_with(&layout.config_dir));
    }
}
