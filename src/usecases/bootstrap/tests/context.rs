use std::path::Path;

use crate::{
    infra::{
        config::{AppConfig, TelegramConfig},
        stubs::StubConfigAdapter,
    },
    usecases::guided_auth::AuthBackendError,
};

use super::super::build_context_with_factories;

use super::{FixedConfigAdapter, StubTelegramAdapterFactory};

fn configured_telegram() -> TelegramConfig {
    TelegramConfig {
        api_id: 777,
        api_hash: "real-hash".to_owned(),
    }
}

#[test]
fn returns_default_config_when_file_is_missing() {
    let adapter =
        crate::infra::config::FileConfigAdapter::new(Some(Path::new("./missing-config.toml")));
    let loaded =
        crate::infra::contracts::ConfigAdapter::load(&adapter).expect("default config should load");

    assert_eq!(loaded, AppConfig::default());
}

#[test]
fn falls_back_to_built_in_credentials_when_own_are_not_set() {
    let factory = StubTelegramAdapterFactory { result: Ok(()) };

    let context = build_context_with_factories(&StubConfigAdapter, &factory)
        .expect("bootstrap without own credentials must succeed");

    assert_eq!(
        context.config.telegram,
        TelegramConfig::default().or_built_in()
    );
    assert!(context.config.telegram.is_configured());
}

#[test]
fn own_credentials_take_precedence_over_built_in() {
    let adapter = FixedConfigAdapter::new(AppConfig {
        telegram: configured_telegram(),
        ..AppConfig::default()
    });
    let factory = StubTelegramAdapterFactory { result: Ok(()) };

    let context = build_context_with_factories(&adapter, &factory)
        .expect("bootstrap with own credentials must succeed");

    assert_eq!(context.config.telegram, configured_telegram());
}

#[test]
fn rejects_partially_configured_telegram_config() {
    let mut config = AppConfig::default();
    config.telegram.api_id = 100;

    let config_adapter = FixedConfigAdapter::new(config);
    let telegram_factory = StubTelegramAdapterFactory { result: Ok(()) };

    let error = build_context_with_factories(&config_adapter, &telegram_factory)
        .expect_err("partial config must fail");
    let rendered = error.to_string();

    assert!(rendered.contains("TELEGRAM_CONFIG_INVALID"));
    assert!(rendered.contains("telegram.api_id"));
    assert!(!rendered.contains("api_hash ="));
}

#[test]
fn maps_transient_bootstrap_error_to_safe_validation_error() {
    let config_adapter = FixedConfigAdapter::new(AppConfig {
        telegram: configured_telegram(),
        ..AppConfig::default()
    });
    let telegram_factory = StubTelegramAdapterFactory {
        result: Err(AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            message: "token=supersecret".to_owned(),
        }),
    };

    let error = build_context_with_factories(&config_adapter, &telegram_factory)
        .expect_err("configured bootstrap should fail fast");
    let rendered = error.to_string();

    assert!(rendered.contains("TELEGRAM_BOOTSTRAP_FAILED"));
    assert!(rendered.contains("AUTH_BACKEND_UNAVAILABLE"));
    assert!(!rendered.contains("supersecret"));
}

#[test]
fn maps_non_transient_bootstrap_errors_to_stable_codes() {
    let config_adapter = FixedConfigAdapter::new(AppConfig {
        telegram: configured_telegram(),
        ..AppConfig::default()
    });
    let telegram_factory = StubTelegramAdapterFactory {
        result: Err(AuthBackendError::InvalidPhone),
    };

    let error = build_context_with_factories(&config_adapter, &telegram_factory)
        .expect_err("error should be mapped");

    assert!(error
        .to_string()
        .contains("telegram client initialization failed [AUTH_INVALID_PHONE]"));
}
