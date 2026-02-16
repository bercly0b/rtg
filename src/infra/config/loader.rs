use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::infra::{
    config::{file_config::FileConfig, AppConfig},
    error::AppError,
};

const DEFAULT_CONFIG_PATH: &str = "config.toml";

pub fn load(path: Option<&Path>) -> Result<AppConfig, AppError> {
    let config_path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));

    let mut config = AppConfig::default();

    if !config_path.exists() {
        return Ok(config);
    }

    let raw = fs::read_to_string(&config_path).map_err(|source| AppError::ConfigRead {
        path: config_path.clone(),
        source,
    })?;

    let file_config: FileConfig = toml::from_str(&raw).map_err(|source| AppError::ConfigParse {
        path: config_path,
        source,
    })?;

    file_config.merge_into(&mut config);
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_defaults_when_file_is_missing() {
        let config = load(Some(Path::new("./missing-config.toml"))).expect("config must load");

        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn merges_file_values_over_defaults() {
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

        let config = load(Some(&config_path)).expect("config must load");
        fs::remove_file(config_path).expect("must remove test config");

        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.telegram.api_id, 123);
        assert_eq!(config.telegram.api_hash, "abc");
    }
}
