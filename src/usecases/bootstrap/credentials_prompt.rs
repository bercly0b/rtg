use std::io;

use crate::{
    infra::{config::AppConfig, contracts::ConfigAdapter, error::AppError},
    usecases::guided_auth::AuthTerminal,
};

const ATTEMPTS: usize = 3;
const API_HASH_LEN: usize = 32;

pub(super) fn ensure_telegram_credentials(
    config: &mut AppConfig,
    config_adapter: &dyn ConfigAdapter,
    terminal: &mut dyn AuthTerminal,
    interactive: bool,
) -> Result<(), AppError> {
    if config.telegram.is_configured() {
        return Ok(());
    }

    if !interactive {
        return Err(AppError::ConfigValidation {
            code: "TELEGRAM_CREDENTIALS_MISSING_NONINTERACTIVE",
            details:
                "telegram.api_id and telegram.api_hash are not set; create your config (see https://my.telegram.org/apps for credentials)"
                    .to_owned(),
        });
    }

    print_intro(terminal).map_err(io_to_app_error)?;

    let api_id = match collect_api_id(terminal, ATTEMPTS).map_err(io_to_app_error)? {
        Some(id) => id,
        None => return Err(setup_cancelled()),
    };

    let api_hash = match collect_api_hash(terminal, ATTEMPTS).map_err(io_to_app_error)? {
        Some(hash) => hash,
        None => return Err(setup_cancelled()),
    };

    config_adapter
        .save_telegram_credentials(api_id, &api_hash)
        .map_err(AppError::Other)?;

    config.telegram.api_id = api_id;
    config.telegram.api_hash = api_hash;

    terminal
        .print_line("Credentials saved. Continuing.")
        .map_err(io_to_app_error)?;

    Ok(())
}

fn print_intro(terminal: &mut dyn AuthTerminal) -> io::Result<()> {
    terminal.print_line("")?;
    terminal.print_line("First-time setup — Telegram API credentials are required.")?;
    terminal.print_line(
        "Get them at https://my.telegram.org/apps (sign in with your phone, then create an application).",
    )?;
    terminal.print_line("")
}

fn collect_api_id(terminal: &mut dyn AuthTerminal, attempts: usize) -> io::Result<Option<i32>> {
    for attempt in 1..=attempts {
        let Some(input) = terminal.prompt_line("api_id: ")? else {
            terminal.print_line("Input cancelled (EOF). Run rtg again to retry.")?;
            return Ok(None);
        };

        if let Some(parsed) = is_valid_api_id(&input) {
            return Ok(Some(parsed));
        }

        terminal.print_line(&format!(
            "Invalid api_id. Expected a positive integer (e.g. 12345). Attempts left: {}",
            attempts.saturating_sub(attempt)
        ))?;
    }

    terminal.print_line("api_id step failed too many times. Please restart rtg and try again.")?;
    Ok(None)
}

fn collect_api_hash(
    terminal: &mut dyn AuthTerminal,
    attempts: usize,
) -> io::Result<Option<String>> {
    for attempt in 1..=attempts {
        let Some(input) = terminal.prompt_line("api_hash: ")? else {
            terminal.print_line("Input cancelled (EOF). Run rtg again to retry.")?;
            return Ok(None);
        };

        let trimmed = input.trim();
        if is_valid_api_hash(trimmed) {
            return Ok(Some(trimmed.to_owned()));
        }

        terminal.print_line(&format!(
            "Invalid api_hash. Expected 32 hexadecimal characters. Attempts left: {}",
            attempts.saturating_sub(attempt)
        ))?;
    }

    terminal
        .print_line("api_hash step failed too many times. Please restart rtg and try again.")?;
    Ok(None)
}

fn is_valid_api_id(input: &str) -> Option<i32> {
    let trimmed = input.trim();
    let parsed: i32 = trimmed.parse().ok()?;
    if parsed > 0 {
        Some(parsed)
    } else {
        None
    }
}

fn is_valid_api_hash(input: &str) -> bool {
    if input == "replace-me" {
        return false;
    }
    input.len() == API_HASH_LEN && input.chars().all(|c| c.is_ascii_hexdigit())
}

fn io_to_app_error(source: io::Error) -> AppError {
    AppError::ConfigValidation {
        code: "TELEGRAM_CREDENTIALS_IO",
        details: format!("terminal IO failed during credentials setup: {source}"),
    }
}

fn setup_cancelled() -> AppError {
    AppError::ConfigValidation {
        code: "TELEGRAM_CREDENTIALS_SETUP_CANCELLED",
        details: "telegram credentials setup cancelled; rerun rtg to try again".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    use crate::infra::config::TelegramConfig;

    struct FakeTerminal {
        inputs: RefCell<Vec<Option<String>>>,
        printed: RefCell<Vec<String>>,
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
                printed: RefCell::new(Vec::new()),
                prompt_calls: RefCell::new(0),
            }
        }

        fn prompt_call_count(&self) -> usize {
            *self.prompt_calls.borrow()
        }
    }

    impl AuthTerminal for FakeTerminal {
        fn print_line(&mut self, line: &str) -> io::Result<()> {
            self.printed.borrow_mut().push(line.to_owned());
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

        fn prompt_secret(&mut self, _prompt: &str) -> io::Result<Option<String>> {
            self.prompt_line(_prompt)
        }
    }

    struct RecordingConfigAdapter {
        saved: RefCell<Option<(i32, String)>>,
    }

    impl RecordingConfigAdapter {
        fn new() -> Self {
            Self {
                saved: RefCell::new(None),
            }
        }
    }

    impl ConfigAdapter for RecordingConfigAdapter {
        fn load(&self) -> anyhow::Result<AppConfig> {
            Ok(AppConfig::default())
        }

        fn save_telegram_credentials(&self, api_id: i32, api_hash: &str) -> anyhow::Result<()> {
            *self.saved.borrow_mut() = Some((api_id, api_hash.to_owned()));
            Ok(())
        }
    }

    const VALID_HASH: &str = "abcdef0123456789abcdef0123456789";

    #[test]
    fn is_valid_api_id_accepts_positive() {
        assert_eq!(is_valid_api_id("12345"), Some(12345));
        assert_eq!(is_valid_api_id("  42  "), Some(42));
    }

    #[test]
    fn is_valid_api_id_rejects_zero_negative_non_numeric_overflow() {
        assert!(is_valid_api_id("0").is_none());
        assert!(is_valid_api_id("-1").is_none());
        assert!(is_valid_api_id("abc").is_none());
        assert!(is_valid_api_id(&i64::MAX.to_string()).is_none());
        assert!(is_valid_api_id("").is_none());
    }

    #[test]
    fn is_valid_api_hash_accepts_32_hex() {
        assert!(is_valid_api_hash("0123456789abcdef0123456789abcdef"));
        assert!(is_valid_api_hash("ABCDEF0123456789ABCDEF0123456789"));
        assert!(is_valid_api_hash("AbCdEf0123456789AbCdEf0123456789"));
    }

    #[test]
    fn is_valid_api_hash_rejects_invalid() {
        assert!(!is_valid_api_hash("short"));
        assert!(!is_valid_api_hash(&"z".repeat(32)));
        assert!(!is_valid_api_hash("replace-me"));
        assert!(!is_valid_api_hash(""));
        assert!(!is_valid_api_hash(&"a".repeat(33)));
    }

    #[test]
    fn collector_returns_some_on_first_valid_input() {
        let mut term = FakeTerminal::new(vec![Some("12345"), Some(VALID_HASH)]);
        let id = collect_api_id(&mut term, ATTEMPTS).unwrap();
        let hash = collect_api_hash(&mut term, ATTEMPTS).unwrap();
        assert_eq!(id, Some(12345));
        assert_eq!(hash.as_deref(), Some(VALID_HASH));
    }

    #[test]
    fn collector_retries_on_invalid_then_accepts() {
        let mut term = FakeTerminal::new(vec![Some("bad"), Some("0"), Some("777")]);
        let id = collect_api_id(&mut term, ATTEMPTS).unwrap();
        assert_eq!(id, Some(777));
    }

    #[test]
    fn collector_returns_none_after_exhausted_attempts() {
        let mut term = FakeTerminal::new(vec![Some("a"), Some("b"), Some("c")]);
        let id = collect_api_id(&mut term, ATTEMPTS).unwrap();
        assert_eq!(id, None);
    }

    #[test]
    fn collector_returns_none_on_eof_at_api_id() {
        let mut term = FakeTerminal::new(vec![None]);
        let id = collect_api_id(&mut term, ATTEMPTS).unwrap();
        assert_eq!(id, None);
    }

    #[test]
    fn collector_returns_none_on_eof_at_api_hash() {
        let mut term = FakeTerminal::new(vec![None]);
        let hash = collect_api_hash(&mut term, ATTEMPTS).unwrap();
        assert_eq!(hash, None);
    }

    #[test]
    fn ensure_returns_err_when_non_interactive_and_unconfigured() {
        let mut config = AppConfig::default();
        let adapter = RecordingConfigAdapter::new();
        let mut term = FakeTerminal::new(vec![]);

        let err = ensure_telegram_credentials(&mut config, &adapter, &mut term, false)
            .expect_err("must fail in non-interactive mode");
        assert!(err
            .to_string()
            .contains("TELEGRAM_CREDENTIALS_MISSING_NONINTERACTIVE"));
        assert!(adapter.saved.borrow().is_none());
        assert_eq!(term.prompt_call_count(), 0);
    }

    #[test]
    fn ensure_skips_prompt_when_already_configured() {
        let mut config = AppConfig {
            telegram: TelegramConfig {
                api_id: 1,
                api_hash: "real-hash".to_owned(),
            },
            ..AppConfig::default()
        };
        let adapter = RecordingConfigAdapter::new();
        let mut term = FakeTerminal::new(vec![]);

        ensure_telegram_credentials(&mut config, &adapter, &mut term, true)
            .expect("must succeed without prompting");
        assert_eq!(term.prompt_call_count(), 0);
        assert!(adapter.saved.borrow().is_none());
    }

    #[test]
    fn ensure_prompts_saves_and_updates_config_when_unconfigured() {
        let mut config = AppConfig::default();
        let adapter = RecordingConfigAdapter::new();
        let mut term = FakeTerminal::new(vec![Some("9999"), Some(VALID_HASH)]);

        ensure_telegram_credentials(&mut config, &adapter, &mut term, true)
            .expect("must succeed with valid input");

        assert_eq!(config.telegram.api_id, 9999);
        assert_eq!(config.telegram.api_hash, VALID_HASH);
        assert_eq!(*adapter.saved.borrow(), Some((9999, VALID_HASH.to_owned())));
    }

    #[test]
    fn ensure_returns_cancelled_error_on_eof() {
        let mut config = AppConfig::default();
        let adapter = RecordingConfigAdapter::new();
        let mut term = FakeTerminal::new(vec![None]);

        let err = ensure_telegram_credentials(&mut config, &adapter, &mut term, true)
            .expect_err("must fail on EOF");
        assert!(err
            .to_string()
            .contains("TELEGRAM_CREDENTIALS_SETUP_CANCELLED"));
        assert!(adapter.saved.borrow().is_none());
    }
}
