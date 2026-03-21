use serde::Deserialize;

use crate::infra::config::{AppConfig, CacheConfig, LogConfig, TelegramConfig};

#[derive(Debug, Deserialize, Default)]
pub struct FileConfig {
    pub logging: Option<FileLogConfig>,
    pub telegram: Option<FileTelegramConfig>,
    pub cache: Option<FileCacheConfig>,
}

impl FileConfig {
    pub fn merge_into(self, config: &mut AppConfig) {
        if let Some(logging) = self.logging {
            logging.merge_into(&mut config.logging);
        }

        if let Some(telegram) = self.telegram {
            telegram.merge_into(&mut config.telegram);
        }

        if let Some(cache) = self.cache {
            cache.merge_into(&mut config.cache);
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

#[derive(Debug, Deserialize, Default)]
pub struct FileCacheConfig {
    pub max_cached_chats: Option<usize>,
    pub max_messages_per_chat: Option<usize>,
    pub min_display_messages: Option<usize>,
}

impl FileCacheConfig {
    fn merge_into(self, config: &mut CacheConfig) {
        if let Some(max_cached_chats) = self.max_cached_chats {
            config.max_cached_chats = max_cached_chats;
        }
        if let Some(max_messages_per_chat) = self.max_messages_per_chat {
            config.max_messages_per_chat = max_messages_per_chat;
        }
        if let Some(min_display_messages) = self.min_display_messages {
            config.min_display_messages = min_display_messages;
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

[cache]
max_cached_chats = 100
max_messages_per_chat = 500
min_display_messages = 3
"#;
        let config: FileConfig = toml::from_str(toml).unwrap();
        let logging = config.logging.unwrap();
        assert_eq!(logging.level.unwrap(), "debug");
        assert_eq!(logging.max_log_files.unwrap(), 5);

        let telegram = config.telegram.unwrap();
        assert_eq!(telegram.api_id.unwrap(), 123);
        assert_eq!(telegram.api_hash.unwrap(), "abc");

        let cache = config.cache.unwrap();
        assert_eq!(cache.max_cached_chats.unwrap(), 100);
        assert_eq!(cache.max_messages_per_chat.unwrap(), 500);
        assert_eq!(cache.min_display_messages.unwrap(), 3);
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
            cache: Some(FileCacheConfig {
                max_cached_chats: Some(75),
                max_messages_per_chat: None,
                min_display_messages: None,
            }),
        };

        let mut config = AppConfig::default();
        file.merge_into(&mut config);

        assert_eq!(config.logging.level, "trace");
        assert_eq!(config.logging.max_log_files, 3); // default preserved
        assert_eq!(config.telegram.api_id, 0); // default preserved
        assert_eq!(config.cache.max_cached_chats, 75);
        assert_eq!(
            config.cache.max_messages_per_chat,
            crate::domain::message_cache::DEFAULT_MAX_MESSAGES_PER_CHAT
        );
        assert_eq!(
            config.cache.min_display_messages,
            crate::domain::message_cache::DEFAULT_MIN_DISPLAY_MESSAGES
        );
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
            cache: Some(FileCacheConfig {
                max_cached_chats: Some(100),
                max_messages_per_chat: Some(500),
                min_display_messages: Some(10),
            }),
        };

        let mut config = AppConfig::default();
        file.merge_into(&mut config);

        assert_eq!(config.logging.level, "error");
        assert_eq!(config.logging.max_log_files, 10);
        assert_eq!(config.telegram.api_id, 999);
        assert_eq!(config.telegram.api_hash, "hash");
        assert_eq!(config.cache.max_cached_chats, 100);
        assert_eq!(config.cache.max_messages_per_chat, 500);
        assert_eq!(config.cache.min_display_messages, 10);
    }
}
