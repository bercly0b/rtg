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
    pub max_log_files: Option<usize>,
}

impl FileLogConfig {
    fn merge_into(self, config: &mut LogConfig) {
        if let Some(level) = self.level {
            config.level = level;
        }
        if let Some(max_log_files) = self.max_log_files {
            config.max_log_files = max_log_files;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_full_config() {
        let toml = r#"
[logging]
level = "debug"
max_log_files = 5

[telegram]
api_id = 123
api_hash = "abc"
"#;
        let config: FileConfig = toml::from_str(toml).unwrap();
        let logging = config.logging.unwrap();
        assert_eq!(logging.level.unwrap(), "debug");
        assert_eq!(logging.max_log_files.unwrap(), 5);

        let telegram = config.telegram.unwrap();
        assert_eq!(telegram.api_id.unwrap(), 123);
        assert_eq!(telegram.api_hash.unwrap(), "abc");
    }

    #[test]
    fn deserialize_partial_config_omits_none_fields() {
        let toml = r#"
[logging]
level = "warn"
"#;
        let config: FileConfig = toml::from_str(toml).unwrap();
        let logging = config.logging.unwrap();
        assert_eq!(logging.level.unwrap(), "warn");
        assert!(logging.max_log_files.is_none());
        assert!(config.telegram.is_none());
    }

    #[test]
    fn deserialize_empty_config() {
        let config: FileConfig = toml::from_str("").unwrap();
        assert!(config.logging.is_none());
        assert!(config.telegram.is_none());
    }

    #[test]
    fn merge_into_overrides_only_set_fields() {
        let file = FileConfig {
            logging: Some(FileLogConfig {
                level: Some("trace".to_owned()),
                max_log_files: None,
            }),
            telegram: None,
        };

        let mut config = AppConfig::default();
        file.merge_into(&mut config);

        assert_eq!(config.logging.level, "trace");
        assert_eq!(config.logging.max_log_files, 3); // default preserved
        assert_eq!(config.telegram.api_id, 0); // default preserved
    }

    #[test]
    fn merge_into_with_all_fields() {
        let file = FileConfig {
            logging: Some(FileLogConfig {
                level: Some("error".to_owned()),
                max_log_files: Some(10),
            }),
            telegram: Some(FileTelegramConfig {
                api_id: Some(999),
                api_hash: Some("hash".to_owned()),
            }),
        };

        let mut config = AppConfig::default();
        file.merge_into(&mut config);

        assert_eq!(config.logging.level, "error");
        assert_eq!(config.logging.max_log_files, 10);
        assert_eq!(config.telegram.api_id, 999);
        assert_eq!(config.telegram.api_hash, "hash");
    }
}
