use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppConfig {
    pub logging: LogConfig,
    pub telegram: TelegramConfig,
    pub startup: StartupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogConfig {
    pub level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: String,
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
pub struct StartupConfig {
    pub session_probe_timeout_ms: u64,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            session_probe_timeout_ms: 1_500,
        }
    }
}
