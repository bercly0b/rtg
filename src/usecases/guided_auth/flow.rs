use std::io;

use super::{
    helpers::{handle_backend_error, is_valid_code, is_valid_phone, print_status_snapshot},
    AuthCodeToken, AuthTerminal, GuidedAuthOutcome, RetryPolicy, SignInOutcome, TelegramAuthClient,
};

/// Runs interactive guided authentication flow.
///
/// TDLib handles session persistence automatically — no explicit session
/// file management is needed. After successful auth, TDLib's database
/// contains the session state.
pub fn run_guided_auth(
    terminal: &mut dyn AuthTerminal,
    auth_client: &mut dyn TelegramAuthClient,
    retry_policy: &RetryPolicy,
) -> io::Result<GuidedAuthOutcome> {
    terminal.print_line("No valid session found. Starting guided authentication.")?;
    print_status_snapshot(terminal, auth_client, "start")?;

    let Some(phone) = collect_phone(terminal, retry_policy.phone_attempts)? else {
        return Ok(GuidedAuthOutcome::ExitWithGuidance);
    };

    let Some(token) = request_code(terminal, auth_client, &phone, retry_policy.phone_attempts)?
    else {
        return Ok(GuidedAuthOutcome::ExitWithGuidance);
    };

    let Some(outcome) = collect_code(terminal, auth_client, &token, retry_policy.code_attempts)?
    else {
        return Ok(GuidedAuthOutcome::ExitWithGuidance);
    };

    if matches!(outcome, SignInOutcome::PasswordRequired)
        && collect_password(terminal, auth_client, retry_policy.password_attempts)?.is_none()
    {
        return Ok(GuidedAuthOutcome::ExitWithGuidance);
    }

    terminal.print_line("Authentication successful. Session saved.")?;

    Ok(GuidedAuthOutcome::Authenticated)
}

fn collect_phone(terminal: &mut dyn AuthTerminal, attempts: usize) -> io::Result<Option<String>> {
    for attempt in 1..=attempts {
        terminal.print_line(
            "Step 1/3 — Enter your phone number in international format, e.g. +15551234567.",
        )?;
        let Some(phone) = terminal.prompt_line("Phone: ")? else {
            terminal.print_line("Input cancelled (EOF). Run rtg again to retry.")?;
            return Ok(None);
        };

        if !is_valid_phone(&phone) {
            terminal.print_line(&format!(
                "Invalid format. Use + followed by 8-15 digits. Attempts left: {}",
                attempts.saturating_sub(attempt)
            ))?;
            continue;
        }

        return Ok(Some(phone));
    }

    terminal
        .print_line("Phone step failed too many times. Please restart rtg and try again later.")?;
    Ok(None)
}

fn request_code(
    terminal: &mut dyn AuthTerminal,
    auth_client: &mut dyn TelegramAuthClient,
    phone: &str,
    attempts: usize,
) -> io::Result<Option<AuthCodeToken>> {
    for attempt in 1..=attempts {
        match auth_client.request_login_code(phone) {
            Ok(token) => {
                print_status_snapshot(terminal, auth_client, "start")?;
                terminal
                    .print_line("Code has been sent in Telegram. Continue to the next step.")?;
                return Ok(Some(token));
            }
            Err(err) => {
                print_status_snapshot(terminal, auth_client, "start")?;
                if !handle_backend_error(terminal, err, attempt, attempts, "phone")? {
                    return Ok(None);
                }
                terminal.print_line("Action: retry(start)")?;
            }
        }
    }

    terminal.print_line("Unable to request login code. Please restart rtg later.")?;
    Ok(None)
}

fn collect_code(
    terminal: &mut dyn AuthTerminal,
    auth_client: &mut dyn TelegramAuthClient,
    token: &AuthCodeToken,
    attempts: usize,
) -> io::Result<Option<SignInOutcome>> {
    for attempt in 1..=attempts {
        terminal.print_line("Step 2/3 — Enter the code from Telegram (digits only).")?;
        let Some(code) = terminal.prompt_line("Code: ")? else {
            terminal.print_line("Input cancelled (EOF). Run rtg again to retry.")?;
            return Ok(None);
        };

        if !is_valid_code(&code) {
            terminal.print_line(&format!(
                "Invalid code format. Use 3-8 digits. Attempts left: {}",
                attempts.saturating_sub(attempt)
            ))?;
            continue;
        }

        match auth_client.sign_in_with_code(token, &code) {
            Ok(outcome) => {
                print_status_snapshot(terminal, auth_client, "code")?;
                return Ok(Some(outcome));
            }
            Err(err) => {
                print_status_snapshot(terminal, auth_client, "code")?;
                if !handle_backend_error(terminal, err, attempt, attempts, "code")? {
                    return Ok(None);
                }
                terminal.print_line("Action: retry(code)")?;
            }
        }
    }

    terminal.print_line("Code step failed too many times. Please restart rtg.")?;
    Ok(None)
}

fn collect_password(
    terminal: &mut dyn AuthTerminal,
    auth_client: &mut dyn TelegramAuthClient,
    attempts: usize,
) -> io::Result<Option<()>> {
    for attempt in 1..=attempts {
        terminal.print_line("Step 3/3 — 2FA password is required for this account.")?;
        let Some(password) = terminal.prompt_secret("2FA password: ")? else {
            terminal.print_line("Input cancelled (EOF). Run rtg again to retry.")?;
            return Ok(None);
        };

        if password.trim().is_empty() {
            terminal.print_line(&format!(
                "Password cannot be empty. Attempts left: {}",
                attempts.saturating_sub(attempt)
            ))?;
            continue;
        }

        match auth_client.verify_password(&password) {
            Ok(()) => {
                print_status_snapshot(terminal, auth_client, "password")?;
                return Ok(Some(()));
            }
            Err(err) => {
                print_status_snapshot(terminal, auth_client, "password")?;
                if !handle_backend_error(terminal, err, attempt, attempts, "2fa")? {
                    return Ok(None);
                }
                terminal.print_line("Action: retry(password)")?;
            }
        }
    }

    terminal.print_line("2FA step failed too many times. Please restart rtg.")?;
    Ok(None)
}
