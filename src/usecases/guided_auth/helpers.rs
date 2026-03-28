use std::io;

use crate::infra::secrets::sanitize_error_code;

use super::{AuthBackendError, AuthTerminal, TelegramAuthClient};

pub(super) fn print_status_snapshot(
    terminal: &mut dyn AuthTerminal,
    auth_client: &dyn TelegramAuthClient,
    action: &str,
) -> io::Result<()> {
    let Some(snapshot) = auth_client.auth_status_snapshot() else {
        return Ok(());
    };

    let last_error = snapshot
        .last_error
        .as_ref()
        .map(|error| error.code.as_str())
        .unwrap_or("none");

    terminal.print_line(&format!(
        "status[{action}]: auth={} connectivity={} last_error={last_error}",
        snapshot.auth.as_label(),
        snapshot.connectivity.as_label()
    ))
}

pub(super) fn handle_backend_error(
    terminal: &mut dyn AuthTerminal,
    error: AuthBackendError,
    attempt: usize,
    max_attempts: usize,
    step: &str,
) -> io::Result<bool> {
    let attempts_left = max_attempts.saturating_sub(attempt);

    match error {
        AuthBackendError::InvalidPhone => {
            terminal.print_line(&format!(
                "AUTH_INVALID_PHONE: Telegram rejected the phone. Check number and retry. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
        AuthBackendError::InvalidCode => {
            terminal.print_line(&format!(
                "AUTH_INVALID_CODE: The code is incorrect or expired. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
        AuthBackendError::WrongPassword => {
            terminal.print_line(&format!(
                "AUTH_WRONG_2FA: Incorrect 2FA password. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
        AuthBackendError::Timeout => {
            terminal.print_line(&format!(
                "AUTH_TIMEOUT: Request timed out at {step} step. Check network and retry. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
        AuthBackendError::FloodWait { seconds } => {
            terminal.print_line(&format!(
                "AUTH_FLOOD_WAIT: Too many attempts. Wait about {seconds}s before retrying."
            ))?;
            Ok(false)
        }
        AuthBackendError::Transient { code, .. } => {
            let safe_code = sanitize_error_code(code);

            if safe_code == "AUTH_BACKEND_UNAVAILABLE" {
                terminal.print_line(&format!(
                    "{safe_code}: Telegram auth backend is unavailable at {step} step. Check Telegram API config (api_id/api_hash) and network, then retry."
                ))?;
                return Ok(false);
            }

            terminal.print_line(&format!(
                "{safe_code}: temporary authorization issue at {step} step. Please retry. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
    }
}

pub(super) fn is_valid_phone(phone: &str) -> bool {
    let digits = phone.strip_prefix('+').unwrap_or_default();
    phone.starts_with('+')
        && (8..=15).contains(&digits.len())
        && digits.chars().all(|ch| ch.is_ascii_digit())
}

pub(super) fn is_valid_code(code: &str) -> bool {
    (3..=8).contains(&code.len()) && code.chars().all(|ch| ch.is_ascii_digit())
}
