use std::path::Path;

use crate::{
    infra::{
        config::{AppConfig, TelegramConfig},
        stubs::StubConfigAdapter,
    },
    usecases::guided_auth::AuthBackendError,
};

use super::super::{build_context_with, build_context_with_factories};

use super::{FixedConfigAdapter, StubTelegramAdapterFactory};

#[test]
fn builds_context_with_default_config_when_file_is_missing() {
    let config_adapter =
        crate::infra::config::FileConfigAdapter::new(Some(Path::new("./missing-config.toml")));
    let context = build_context_with(&config_adapter).expect("context should build from defaults");

    assert_eq!(context.config, AppConfig::default());
    assert!(!context.telegram.uses_real_backend());
}

#[test]
fn builds_context_via_config_contract() {
    let adapter = StubConfigAdapter;
    let context = build_context_with(&adapter).expect("context should build from config adapter");

    assert_eq!(context.config, AppConfig::default());
    assert!(!context.telegram.uses_real_backend());
}

#[test]
fn rejects_partially_configured_telegram_config() {
    let mut config = AppConfig::default();
    config.telegram.api_id = 100;

    let config_adapter = FixedConfigAdapter { config };
    let telegram_factory = StubTelegramAdapterFactory { result: Ok(()) };

    let error = build_context_with_factories(&config_adapter, &telegram_factory)
        .expect_err("must fail on partial telegram config");
    let rendered = error.to_string();

    assert!(rendered.contains("TELEGRAM_CONFIG_INVALID"));
    assert!(rendered.contains("telegram.api_id"));
    assert!(!rendered.contains("api_hash ="));
}

#[test]
fn build_context_uses_stub_when_telegram_is_unconfigured() {
    let config_adapter = FixedConfigAdapter {
        config: AppConfig::default(),
    };
    let telegram_factory = StubTelegramAdapterFactory { result: Ok(()) };

    let context = build_context_with_factories(&config_adapter, &telegram_factory)
        .expect("unconfigured bootstrap should succeed");

    assert!(!context.telegram.uses_real_backend());
}

#[test]
fn maps_transient_bootstrap_error_to_safe_validation_error() {
    let mut config = AppConfig::default();
    config.telegram = TelegramConfig {
        api_id: 777,
        api_hash: "hash".to_owned(),
    };

    let config_adapter = FixedConfigAdapter { config };
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
    let mut config = AppConfig::default();
    config.telegram = TelegramConfig {
        api_id: 777,
        api_hash: "hash".to_owned(),
    };

    let config_adapter = FixedConfigAdapter { config };
    let telegram_factory = StubTelegramAdapterFactory {
        result: Err(AuthBackendError::InvalidPhone),
    };

    let error = build_context_with_factories(&config_adapter, &telegram_factory)
        .expect_err("error should be mapped");

    assert!(error
        .to_string()
        .contains("telegram client initialization failed [AUTH_INVALID_PHONE]"));
}
