use std::{env, fs, path::PathBuf};

use crate::infra::error::AppError;

const APP_DIR_NAME: &str = "rtg";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageLayout {
    pub config_dir: PathBuf,
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
        let cache_dir = config_dir.join("cache");

        Ok(Self {
            config_dir,
            cache_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<(), AppError> {
        for dir in [&self.config_dir, &self.cache_dir] {
            fs::create_dir_all(dir).map_err(|source| AppError::StorageDirCreate {
                path: dir.clone(),
                source,
            })?;
        }

        Ok(())
    }

    /// Returns the path for the single-instance advisory lock file.
    ///
    /// This prevents multiple RTG processes from running simultaneously,
    /// which would cause conflicts with TDLib's SQLite database.
    pub fn instance_lock_file(&self) -> PathBuf {
        self.config_dir.join("rtg.lock")
    }

    /// Returns the directory for TDLib's SQLite database.
    pub fn tdlib_database_dir(&self) -> PathBuf {
        self.cache_dir.join("tdlib")
    }

    /// Returns the directory for TDLib's downloaded files.
    pub fn tdlib_files_dir(&self) -> PathBuf {
        self.cache_dir.join("tdlib_files")
    }

    /// Checks whether a TDLib session (database) exists on disk.
    ///
    /// Returns `true` if the TDLib database directory exists and contains
    /// at least one file, indicating a previous session was established.
    pub fn tdlib_session_exists(&self) -> bool {
        let db_dir = self.tdlib_database_dir();
        db_dir.is_dir()
            && fs::read_dir(&db_dir)
                .map(|mut entries| entries.next().is_some())
                .unwrap_or(false)
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_is_under_config_dir() {
        let layout = StorageLayout::resolve().expect("layout should resolve");

        assert!(layout.cache_dir.starts_with(&layout.config_dir));
    }

    #[test]
    fn tdlib_dirs_are_under_cache_dir() {
        let layout = StorageLayout::resolve().expect("layout should resolve");

        assert!(layout.tdlib_database_dir().starts_with(&layout.cache_dir));
        assert!(layout.tdlib_files_dir().starts_with(&layout.cache_dir));
    }

    #[test]
    fn instance_lock_file_is_under_config_dir() {
        let layout = StorageLayout::resolve().expect("layout should resolve");

        assert!(layout.instance_lock_file().starts_with(&layout.config_dir));
        assert!(layout
            .instance_lock_file()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("rtg.lock"));
    }

    #[test]
    fn tdlib_session_exists_returns_false_for_missing_dir() {
        let layout = StorageLayout {
            config_dir: PathBuf::from("/nonexistent/path"),
            cache_dir: PathBuf::from("/nonexistent/path/cache"),
        };
        assert!(!layout.tdlib_session_exists());
    }

    #[test]
    fn tdlib_session_exists_returns_false_for_empty_dir() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let db_dir = tmp.path().join("cache").join("tdlib");
        fs::create_dir_all(&db_dir).expect("create db dir");

        let layout = StorageLayout {
            config_dir: tmp.path().to_path_buf(),
            cache_dir: tmp.path().join("cache"),
        };
        assert!(!layout.tdlib_session_exists());
    }

    #[test]
    fn tdlib_session_exists_returns_true_when_files_present() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let db_dir = tmp.path().join("cache").join("tdlib");
        fs::create_dir_all(&db_dir).expect("create db dir");
        fs::write(db_dir.join("td.binlog"), b"data").expect("write file");

        let layout = StorageLayout {
            config_dir: tmp.path().to_path_buf(),
            cache_dir: tmp.path().join("cache"),
        };
        assert!(layout.tdlib_session_exists());
    }
}
