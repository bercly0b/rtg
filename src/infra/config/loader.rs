use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::infra::{
    config::{file_config::FileConfig, AppConfig},
    error::AppError,
    storage_layout::StorageLayout,
};

const CONFIG_FILE_NAME: &str = "config.toml";

pub(crate) fn load(path: Option<&Path>) -> Result<AppConfig, AppError> {
    let config_path = match path {
        Some(p) => p.to_path_buf(),
        None => resolve_default_config_path()?,
    };

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

    Ok(config)
}

/// Resolves the default config file path: `~/.config/rtg/config.toml`.
fn resolve_default_config_path() -> Result<PathBuf, AppError> {
    let layout = StorageLayout::resolve()?;
    Ok(layout.config_dir.join(CONFIG_FILE_NAME))
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

    #[test]
    fn fails_on_malformed_toml() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("rtg-test-malformed.toml");
        fs::write(&config_path, "this is not valid [toml = ").expect("must write");

        let error = load(Some(&config_path)).expect_err("must fail");
        fs::remove_file(&config_path).expect("must remove");

        assert!(
            error.to_string().contains("parse"),
            "error should mention parsing: {}",
            error
        );
    }

    #[test]
    fn partial_toml_preserves_unset_defaults() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("rtg-test-partial.toml");
        fs::write(
            &config_path,
            r#"[telegram]
api_id = 42
"#,
        )
        .expect("must write");

        let config = load(Some(&config_path)).expect("config must load");
        fs::remove_file(&config_path).expect("must remove");

        assert_eq!(config.telegram.api_id, 42);
        assert_eq!(config.telegram.api_hash, "replace-me"); // default preserved
        assert_eq!(config.logging.level, "info"); // default preserved
    }

    #[test]
    fn default_config_path_resolves_to_config_dir() {
        let path = resolve_default_config_path().expect("should resolve");
        assert!(path.ends_with("rtg/config.toml"));
    }

    #[test]
    fn voice_config_overridden_from_toml() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("rtg-test-voice-config.toml");

        fs::write(
            &config_path,
            r#"[voice]
record_cmd = "my-recorder --output {file_path}"
"#,
        )
        .expect("must write test config");

        let config = load(Some(&config_path)).expect("config must load");
        fs::remove_file(config_path).expect("must remove test config");

        assert_eq!(config.voice.record_cmd, "my-recorder --output {file_path}");
    }

    #[test]
    fn voice_config_uses_default_when_not_specified() {
        let config = load(Some(Path::new("./missing-config.toml"))).expect("config must load");

        assert_eq!(
            config.voice.record_cmd,
            crate::domain::voice_defaults::DEFAULT_RECORD_CMD
        );
    }
}
