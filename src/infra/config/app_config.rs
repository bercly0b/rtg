use serde::{Deserialize, Serialize};

use crate::domain::message_cache::{
    DEFAULT_MAX_CACHED_CHATS, DEFAULT_MAX_MESSAGES_PER_CHAT, DEFAULT_MIN_DISPLAY_MESSAGES,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppConfig {
    pub logging: LogConfig,
    pub telegram: TelegramConfig,
    pub cache: CacheConfig,
    pub voice: VoiceConfig,
    pub open: OpenConfig,
    pub download: DownloadConfig,
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
    #[serde(default = "default_min_display_messages")]
    pub min_display_messages: usize,
}

fn default_max_cached_chats() -> usize {
    DEFAULT_MAX_CACHED_CHATS
}

fn default_max_messages_per_chat() -> usize {
    DEFAULT_MAX_MESSAGES_PER_CHAT
}

fn default_min_display_messages() -> usize {
    DEFAULT_MIN_DISPLAY_MESSAGES
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_cached_chats: default_max_cached_chats(),
            max_messages_per_chat: default_max_messages_per_chat(),
            min_display_messages: default_min_display_messages(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceConfig {
    #[serde(default = "default_voice_record_cmd")]
    pub record_cmd: String,
}

fn default_voice_record_cmd() -> String {
    crate::domain::voice_defaults::DEFAULT_RECORD_CMD.to_owned()
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            record_cmd: default_voice_record_cmd(),
        }
    }
}

/// Default maximum file size for auto-download (10 MB).
pub const DEFAULT_MAX_AUTO_DOWNLOAD: &str = "10MB";

/// Configuration for automatic file downloading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadConfig {
    /// Maximum file size for auto-download (e.g. "10MB", "500KB", "1GB").
    /// Files larger than this require manual download via Shift+D.
    #[serde(default = "default_max_auto_download_size")]
    pub max_auto_download_size: String,
}

fn default_max_auto_download_size() -> String {
    DEFAULT_MAX_AUTO_DOWNLOAD.to_owned()
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            max_auto_download_size: default_max_auto_download_size(),
        }
    }
}

impl DownloadConfig {
    /// Returns the auto-download limit in bytes.
    pub fn max_auto_download_bytes(&self) -> u64 {
        parse_size(&self.max_auto_download_size).unwrap_or(10_000_000)
    }
}

/// Parses a human-readable size string (e.g. "10MB", "500KB") into bytes.
///
/// Supports units: B, KB, MB, GB, TB (base-10, i.e. 1 KB = 1000 bytes).
/// Returns `None` if the string cannot be parsed.
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let (num_part, unit) = if let Some(pos) = s.find(|c: char| c.is_ascii_alphabetic()) {
        (&s[..pos], s[pos..].to_ascii_uppercase())
    } else {
        (s, "B".to_owned())
    };

    let num: f64 = num_part.trim().parse().ok()?;
    if num < 0.0 {
        return None;
    }
    let multiplier: u64 = match unit.as_str() {
        "B" => 1,
        "KB" => 1_000,
        "MB" => 1_000_000,
        "GB" => 1_000_000_000,
        "TB" => 1_000_000_000_000,
        _ => return None,
    };

    Some((num * multiplier as f64) as u64)
}

/// Configuration for opening message files (mailcap-style MIME → command mappings).
///
/// Keys are MIME types or wildcard patterns (e.g. `"audio/ogg"`, `"audio/*"`).
/// Values are command templates with a `{file_path}` placeholder.
///
/// When no matching handler is found, the platform default opener is used
/// (`open` on macOS, `xdg-open` on Linux).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OpenConfig {
    #[serde(flatten)]
    pub handlers: std::collections::HashMap<String, String>,
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
        assert_eq!(config.min_display_messages, DEFAULT_MIN_DISPLAY_MESSAGES);
    }

    #[test]
    fn default_voice_config_uses_platform_default_cmd() {
        let config = VoiceConfig::default();
        assert_eq!(
            config.record_cmd,
            crate::domain::voice_defaults::DEFAULT_RECORD_CMD
        );
    }

    #[test]
    fn voice_config_record_cmd_contains_file_path_placeholder() {
        let config = VoiceConfig::default();
        assert!(config.record_cmd.contains("{file_path}"));
    }

    // ── parse_size tests ──

    #[test]
    fn parse_size_bytes() {
        assert_eq!(parse_size("100B"), Some(100));
        assert_eq!(parse_size("0B"), Some(0));
    }

    #[test]
    fn parse_size_kilobytes() {
        assert_eq!(parse_size("10KB"), Some(10_000));
        assert_eq!(parse_size("500KB"), Some(500_000));
    }

    #[test]
    fn parse_size_megabytes() {
        assert_eq!(parse_size("10MB"), Some(10_000_000));
        assert_eq!(parse_size("1MB"), Some(1_000_000));
    }

    #[test]
    fn parse_size_gigabytes() {
        assert_eq!(parse_size("1GB"), Some(1_000_000_000));
    }

    #[test]
    fn parse_size_case_insensitive() {
        assert_eq!(parse_size("10mb"), Some(10_000_000));
        assert_eq!(parse_size("10Mb"), Some(10_000_000));
    }

    #[test]
    fn parse_size_with_spaces() {
        assert_eq!(parse_size("  10MB  "), Some(10_000_000));
    }

    #[test]
    fn parse_size_bare_number_treated_as_bytes() {
        assert_eq!(parse_size("1024"), Some(1024));
    }

    #[test]
    fn parse_size_invalid_unit_returns_none() {
        assert_eq!(parse_size("10XB"), None);
    }

    #[test]
    fn parse_size_non_numeric_returns_none() {
        assert_eq!(parse_size("abcMB"), None);
    }

    // ── DownloadConfig tests ──

    #[test]
    fn default_download_config_is_10mb() {
        let config = DownloadConfig::default();
        assert_eq!(config.max_auto_download_size, "10MB");
        assert_eq!(config.max_auto_download_bytes(), 10_000_000);
    }

    #[test]
    fn download_config_custom_size() {
        let config = DownloadConfig {
            max_auto_download_size: "5MB".to_owned(),
        };
        assert_eq!(config.max_auto_download_bytes(), 5_000_000);
    }

    #[test]
    fn download_config_invalid_falls_back_to_default() {
        let config = DownloadConfig {
            max_auto_download_size: "invalid".to_owned(),
        };
        assert_eq!(config.max_auto_download_bytes(), 10_000_000);
    }

    #[test]
    fn parse_size_negative_returns_none() {
        assert_eq!(parse_size("-10MB"), None);
        assert_eq!(parse_size("-1"), None);
    }
}
