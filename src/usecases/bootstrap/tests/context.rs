use std::{cell::RefCell, io, path::Path};

use crate::{
    infra::{
        config::{AppConfig, TelegramConfig},
        stubs::StubConfigAdapter,
    },
    usecases::guided_auth::{AuthBackendError, AuthTerminal},
};

use super::super::{build_context_with_factories, build_context_with_factories_inner};

use super::{FixedConfigAdapter, StubTelegramAdapterFactory};

const VALID_HASH: &str = "abcdef0123456789abcdef0123456789";

struct FakeTerminal {
    inputs: RefCell<Vec<Option<String>>>,
    prompt_calls: RefCell<usize>,
}

impl FakeTerminal {
    fn new(inputs: Vec<Option<&str>>) -> Self {
        Self {
            inputs: RefCell::new(
                inputs
                    .into_iter()
                    .map(|s| s.map(|v| v.to_owned()))
                    .collect(),
            ),
            prompt_calls: RefCell::new(0),
        }
    }

    fn prompt_call_count(&self) -> usize {
        *self.prompt_calls.borrow()
    }
}

impl AuthTerminal for FakeTerminal {
    fn print_line(&mut self, _line: &str) -> io::Result<()> {
        Ok(())
    }

    fn prompt_line(&mut self, _prompt: &str) -> io::Result<Option<String>> {
        *self.prompt_calls.borrow_mut() += 1;
        let mut inputs = self.inputs.borrow_mut();
        if inputs.is_empty() {
            return Ok(None);
        }
        Ok(inputs.remove(0))
    }

    fn prompt_secret(&mut self, prompt: &str) -> io::Result<Option<String>> {
        self.prompt_line(prompt)
    }
}

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
fn build_context_with_factories_errors_in_non_interactive_mode_for_unconfigured() {
    let adapter = FixedConfigAdapter::new(AppConfig::default());
    let factory = StubTelegramAdapterFactory { result: Ok(()) };
    let mut term = FakeTerminal::new(vec![]);

    let error = build_context_with_factories_inner(&adapter, &factory, &mut term, false)
        .expect_err("non-interactive bootstrap with default config must fail");

    assert!(error
        .to_string()
        .contains("TELEGRAM_CREDENTIALS_MISSING_NONINTERACTIVE"));
    assert_eq!(term.prompt_call_count(), 0);
}

#[test]
fn build_context_via_public_api_uses_real_terminal_path() {
    let adapter = FixedConfigAdapter::new(AppConfig {
        telegram: configured_telegram(),
        ..AppConfig::default()
    });
    let factory = StubTelegramAdapterFactory { result: Ok(()) };

    build_context_with_factories(&adapter, &factory)
        .expect("configured bootstrap must succeed via public API");
}

#[test]
fn build_context_with_loads_via_stub_adapter_after_credentials_prompt() {
    let adapter = StubConfigAdapter;
    let mut term = FakeTerminal::new(vec![Some("12345"), Some(VALID_HASH)]);
    let factory = StubTelegramAdapterFactory { result: Ok(()) };

    let context = build_context_with_factories_inner(&adapter, &factory, &mut term, true)
        .expect("interactive bootstrap with stub adapter must succeed");

    assert_eq!(context.config.telegram.api_id, 12345);
    assert_eq!(context.config.telegram.api_hash, VALID_HASH);
}

#[test]
fn rejects_partially_configured_telegram_config() {
    let mut config = AppConfig::default();
    config.telegram.api_id = 100;

    let config_adapter = FixedConfigAdapter::new(config);
    let telegram_factory = StubTelegramAdapterFactory { result: Ok(()) };

    let error = build_context_with_factories_inner(
        &config_adapter,
        &telegram_factory,
        &mut FakeTerminal::new(vec![]),
        false,
    )
    .expect_err("partial config must fail");
    let rendered = error.to_string();

    assert!(rendered.contains("TELEGRAM_CONFIG_INVALID"));
    assert!(rendered.contains("telegram.api_id"));
    assert!(!rendered.contains("api_hash ="));
}

#[test]
fn prompts_for_credentials_when_unconfigured_and_interactive() {
    let adapter = FixedConfigAdapter::new(AppConfig::default());
    let mut term = FakeTerminal::new(vec![Some("4242"), Some(VALID_HASH)]);
    let factory = StubTelegramAdapterFactory { result: Ok(()) };

    let context = build_context_with_factories_inner(&adapter, &factory, &mut term, true)
        .expect("interactive bootstrap with valid input must succeed");

    assert_eq!(context.config.telegram.api_id, 4242);
    assert_eq!(context.config.telegram.api_hash, VALID_HASH);
    assert_eq!(
        adapter.saved_credentials(),
        Some((4242, VALID_HASH.to_owned()))
    );
}

#[test]
fn bails_on_eof_during_credentials_setup() {
    let adapter = FixedConfigAdapter::new(AppConfig::default());
    let mut term = FakeTerminal::new(vec![None]);
    let factory = StubTelegramAdapterFactory { result: Ok(()) };

    let error = build_context_with_factories_inner(&adapter, &factory, &mut term, true)
        .expect_err("EOF during credentials must fail");

    assert!(error
        .to_string()
        .contains("TELEGRAM_CREDENTIALS_SETUP_CANCELLED"));
    assert!(adapter.saved_credentials().is_none());
}

#[test]
fn does_not_prompt_when_already_configured() {
    let adapter = FixedConfigAdapter::new(AppConfig {
        telegram: configured_telegram(),
        ..AppConfig::default()
    });
    let mut term = FakeTerminal::new(vec![]);
    let factory = StubTelegramAdapterFactory { result: Ok(()) };

    build_context_with_factories_inner(&adapter, &factory, &mut term, true)
        .expect("configured bootstrap must succeed without prompts");

    assert_eq!(term.prompt_call_count(), 0);
    assert!(adapter.saved_credentials().is_none());
}

#[test]
fn maps_transient_bootstrap_error_to_safe_validation_error() {
    let mut config = AppConfig::default();
    config.telegram = TelegramConfig {
        api_id: 777,
        api_hash: "hash".to_owned(),
    };

    let config_adapter = FixedConfigAdapter::new(config);
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

    let config_adapter = FixedConfigAdapter::new(config);
    let telegram_factory = StubTelegramAdapterFactory {
        result: Err(AuthBackendError::InvalidPhone),
    };

    let error = build_context_with_factories(&config_adapter, &telegram_factory)
        .expect_err("error should be mapped");

    assert!(error
        .to_string()
        .contains("telegram client initialization failed [AUTH_INVALID_PHONE]"));
}
