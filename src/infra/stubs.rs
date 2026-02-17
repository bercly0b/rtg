use anyhow::Result;

use crate::infra::contracts::{ExternalOpener, StorageAdapter};

#[cfg(test)]
use crate::infra::{config::AppConfig, contracts::ConfigAdapter};

#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct StubConfigAdapter;

#[cfg(test)]
impl ConfigAdapter for StubConfigAdapter {
    fn load(&self) -> Result<AppConfig> {
        Ok(AppConfig::default())
    }
}

#[derive(Debug, Clone, Default)]
pub struct StubStorageAdapter {
    pub last_action: Option<String>,
}

impl StorageAdapter for StubStorageAdapter {
    fn save_last_action(&mut self, action: &str) -> Result<()> {
        self.last_action = Some(action.to_owned());
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct NoopOpener;

impl ExternalOpener for NoopOpener {
    fn open(&self, _target: &str) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_config_returns_defaults() {
        let adapter = StubConfigAdapter;
        let config = adapter.load().expect("stub config must load");

        assert_eq!(config, AppConfig::default());
    }
}
