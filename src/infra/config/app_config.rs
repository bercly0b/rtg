use serde::{Deserialize, Serialize};

use crate::domain::message_cache::{DEFAULT_MAX_CACHED_CHATS, DEFAULT_MAX_MESSAGES_PER_CHAT};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppConfig {
    pub logging: LogConfig,
    pub telegram: TelegramConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogConfig {
    pub level: String,
    #[serde(default = "default_max_log_files")]
    pub max_log_files: usize,
}

fn default_max_log_files() -> usize {
    3
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_owned(),
            max_log_files: default_max_log_files(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: String,
}

impl TelegramConfig {
    pub fn is_configured(&self) -> bool {
        self.api_id > 0 && !self.api_hash.trim().is_empty() && self.api_hash != "replace-me"
    }
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            api_id: 0,
            api_hash: "replace-me".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheConfig {
    #[serde(default = "default_max_cached_chats")]
    pub max_cached_chats: usize,
    #[serde(default = "default_max_messages_per_chat")]
    pub max_messages_per_chat: usize,
}

fn default_max_cached_chats() -> usize {
    DEFAULT_MAX_CACHED_CHATS
}

fn default_max_messages_per_chat() -> usize {
    DEFAULT_MAX_MESSAGES_PER_CHAT
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_cached_chats: default_max_cached_chats(),
            max_messages_per_chat: default_max_messages_per_chat(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_log_config_is_info_with_three_files() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.max_log_files, 3);
    }

    #[test]
    fn default_telegram_config_is_not_configured() {
        let config = TelegramConfig::default();
        assert!(!config.is_configured());
    }

    #[test]
    fn telegram_config_is_configured_with_valid_values() {
        let config = TelegramConfig {
            api_id: 12345,
            api_hash: "valid-hash".to_owned(),
        };
        assert!(config.is_configured());
    }

    #[test]
    fn telegram_config_not_configured_with_zero_api_id() {
        let config = TelegramConfig {
            api_id: 0,
            api_hash: "valid-hash".to_owned(),
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn telegram_config_not_configured_with_placeholder_hash() {
        let config = TelegramConfig {
            api_id: 123,
            api_hash: "replace-me".to_owned(),
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn telegram_config_not_configured_with_empty_hash() {
        let config = TelegramConfig {
            api_id: 123,
            api_hash: "".to_owned(),
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn telegram_config_not_configured_with_whitespace_hash() {
        let config = TelegramConfig {
            api_id: 123,
            api_hash: "   ".to_owned(),
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn default_cache_config_has_expected_values() {
        let config = CacheConfig::default();
        assert_eq!(config.max_cached_chats, DEFAULT_MAX_CACHED_CHATS);
        assert_eq!(config.max_messages_per_chat, DEFAULT_MAX_MESSAGES_PER_CHAT);
    }
}
