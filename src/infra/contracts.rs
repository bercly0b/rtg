use anyhow::Result;

use crate::infra::config::AppConfig;

pub trait ConfigAdapter {
    fn load(&self) -> Result<AppConfig>;
}

pub trait StorageAdapter {
    fn save_last_action(&mut self, action: &str) -> Result<()>;
}

pub trait ExternalOpener {
    fn open(&self, target: &str) -> Result<()>;
}
