use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::infra::{
    config::{load_internal, AppConfig},
    contracts::ConfigAdapter,
};

#[derive(Debug, Clone, Default)]
pub struct FileConfigAdapter {
    path: Option<PathBuf>,
    load_env: bool,
}

impl FileConfigAdapter {
    pub fn new(path: Option<&Path>) -> Self {
        Self {
            path: path.map(Path::to_path_buf),
            load_env: true,
        }
    }

    #[cfg(test)]
    pub fn without_env(path: Option<&Path>) -> Self {
        Self {
            path: path.map(Path::to_path_buf),
            load_env: false,
        }
    }
}

impl ConfigAdapter for FileConfigAdapter {
    fn load(&self) -> Result<AppConfig> {
        Ok(load_internal(self.path.as_deref(), self.load_env)?)
    }
}
