use serde::Deserialize;

use crate::infra::config::{AppConfig, LogConfig, TelegramConfig};

#[derive(Debug, Deserialize, Default)]
pub struct FileConfig {
    pub logging: Option<FileLogConfig>,
    pub telegram: Option<FileTelegramConfig>,
}

impl FileConfig {
    pub fn merge_into(self, config: &mut AppConfig) {
        if let Some(logging) = self.logging {
            logging.merge_into(&mut config.logging);
        }

        if let Some(telegram) = self.telegram {
            telegram.merge_into(&mut config.telegram);
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct FileLogConfig {
    pub level: Option<String>,
}

impl FileLogConfig {
    fn merge_into(self, config: &mut LogConfig) {
        if let Some(level) = self.level {
            config.level = level;
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct FileTelegramConfig {
    pub api_id: Option<i32>,
    pub api_hash: Option<String>,
}

impl FileTelegramConfig {
    fn merge_into(self, config: &mut TelegramConfig) {
        if let Some(api_id) = self.api_id {
            config.api_id = api_id;
        }

        if let Some(api_hash) = self.api_hash {
            config.api_hash = api_hash;
        }
    }
}
