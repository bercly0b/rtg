use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::infra::{
    config::{load, AppConfig},
    contracts::ConfigAdapter,
};

#[derive(Debug, Clone, Default)]
pub struct FileConfigAdapter {
    path: Option<PathBuf>,
}

impl FileConfigAdapter {
    pub fn new(path: Option<&Path>) -> Self {
        Self {
            path: path.map(Path::to_path_buf),
        }
    }
}

impl ConfigAdapter for FileConfigAdapter {
    fn load(&self) -> Result<AppConfig> {
        Ok(load(self.path.as_deref())?)
    }
}
