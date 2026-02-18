use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::infra::{
    config::{file_config::FileConfig, AppConfig},
    error::AppError,
};

const DEFAULT_CONFIG_PATH: &str = "config.toml";
const TELEGRAM_API_ID_ENV: &str = "RTG_TELEGRAM_API_ID";
const TELEGRAM_API_HASH_ENV: &str = "RTG_TELEGRAM_API_HASH";

#[allow(dead_code)]
pub fn load(path: Option<&Path>) -> Result<AppConfig, AppError> {
    load_internal(path, true)
}

pub(crate) fn load_internal(path: Option<&Path>, load_env: bool) -> Result<AppConfig, AppError> {
    let config_path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));

    let mut config = AppConfig::default();

    if config_path.exists() {
        let raw = fs::read_to_string(&config_path).map_err(|source| AppError::ConfigRead {
            path: config_path.clone(),
            source,
        })?;

        let file_config: FileConfig =
            toml::from_str(&raw).map_err(|source| AppError::ConfigParse {
                path: config_path,
                source,
            })?;

        file_config.merge_into(&mut config);
    }

    if load_env {
        let _ = dotenvy::dotenv();
    }
    apply_env_overrides(&mut config)?;

    Ok(config)
}

fn apply_env_overrides(config: &mut AppConfig) -> Result<(), AppError> {
    if let Some(api_id_raw) = read_env_non_empty(TELEGRAM_API_ID_ENV) {
        let api_id = api_id_raw
            .parse::<i32>()
            .map_err(|_| AppError::ConfigValidation {
                code: "telegram_api_id_invalid",
                details: format!("{TELEGRAM_API_ID_ENV} must be a valid i32 integer"),
            })?;

        config.telegram.api_id = api_id;
    }

    if let Some(api_hash) = read_env_non_empty(TELEGRAM_API_HASH_ENV) {
        config.telegram.api_hash = api_hash;
    }

    Ok(())
}

fn read_env_non_empty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::*;

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn clear_env() {
        std::env::remove_var(TELEGRAM_API_ID_ENV);
        std::env::remove_var(TELEGRAM_API_HASH_ENV);
    }

    #[test]
    fn returns_defaults_when_file_is_missing() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");
        clear_env();

        let config = load_internal(Some(Path::new("./missing-config.toml")), false)
            .expect("config must load");

        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn merges_file_values_over_defaults() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");
        clear_env();

        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("rtg-task2-config.toml");

        fs::write(
            &config_path,
            r#"[logging]
level = "debug"

[telegram]
api_id = 123
api_hash = "abc"
"#,
        )
        .expect("must write test config");

        let config = load_internal(Some(&config_path), false).expect("config must load");
        fs::remove_file(config_path).expect("must remove test config");

        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.telegram.api_id, 123);
        assert_eq!(config.telegram.api_hash, "abc");
    }

    #[test]
    fn env_overrides_file_telegram_credentials() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");
        clear_env();

        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("rtg-task2-config-env-override.toml");

        fs::write(
            &config_path,
            r#"[telegram]
api_id = 111
api_hash = "from-file"
"#,
        )
        .expect("must write test config");

        std::env::set_var(TELEGRAM_API_ID_ENV, "777");
        std::env::set_var(TELEGRAM_API_HASH_ENV, "from-env");

        let config = load(Some(&config_path)).expect("config must load");
        fs::remove_file(config_path).expect("must remove test config");
        clear_env();

        assert_eq!(config.telegram.api_id, 777);
        assert_eq!(config.telegram.api_hash, "from-env");
    }

    #[test]
    fn fails_when_env_api_id_is_invalid() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");
        clear_env();

        std::env::set_var(TELEGRAM_API_ID_ENV, "not-a-number");

        let error = load(Some(Path::new("./missing-config.toml"))).expect_err("must fail");
        clear_env();

        let rendered = error.to_string();
        assert!(rendered.contains("telegram_api_id_invalid"));
        assert!(rendered.contains(TELEGRAM_API_ID_ENV));
    }
}
