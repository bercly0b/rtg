use super::*;

#[test]
fn e2e_happy_path_without_2fa_authenticates() {
    let mut terminal = FakeTerminal::new(vec![Some("+15551234567"), Some("12345")]);
    let mut client = FakeClient::new(vec![
        Action::RequestCode(Ok(AuthCodeToken("token".into()))),
        Action::SignIn(Ok(SignInOutcome::Authorized)),
    ]);

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::Authenticated);
}

#[test]
fn e2e_2fa_required_authenticates_after_password_step() {
    let mut terminal = FakeTerminal::new(vec![Some("+15551234567"), Some("12345"), Some("s3cret")]);
    let mut client = FakeClient::new(vec![
        Action::RequestCode(Ok(AuthCodeToken("token".into()))),
        Action::SignIn(Ok(SignInOutcome::PasswordRequired)),
        Action::Verify(Ok(())),
    ]);

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::Authenticated);
}

#[test]
fn e2e_2fa_password_preserves_boundary_spaces_for_verification() {
    let mut terminal = FakeTerminal::new(vec![
        Some("+15551234567"),
        Some("12345"),
        Some("  pass phrase  "),
    ]);
    let mut client = FakeClient::new(vec![
        Action::RequestCode(Ok(AuthCodeToken("token".into()))),
        Action::SignIn(Ok(SignInOutcome::PasswordRequired)),
        Action::Verify(Ok(())),
    ]);

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::Authenticated);
    assert_eq!(
        client.verified_passwords,
        vec!["  pass phrase  ".to_owned()]
    );
}

#[test]
fn e2e_wrong_code_retries_then_succeeds() {
    let mut terminal = FakeTerminal::new(vec![Some("+15551234567"), Some("000"), Some("12345")]);
    let mut client = FakeClient::new(vec![
        Action::RequestCode(Ok(AuthCodeToken("token".into()))),
        Action::SignIn(Err(AuthBackendError::InvalidCode)),
        Action::SignIn(Ok(SignInOutcome::Authorized)),
    ]);

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::Authenticated);
    assert!(terminal
        .output
        .iter()
        .any(|line| line.contains("AUTH_INVALID_CODE")));
}

#[test]
fn e2e_wrong_password_exhausts_retries_and_exits_with_guidance() {
    let mut terminal = FakeTerminal::new(vec![
        Some("+15551234567"),
        Some("12345"),
        Some("wrong-1"),
        Some("wrong-2"),
        Some("wrong-3"),
    ]);
    let mut client = FakeClient::new(vec![
        Action::RequestCode(Ok(AuthCodeToken("token".into()))),
        Action::SignIn(Ok(SignInOutcome::PasswordRequired)),
        Action::Verify(Err(AuthBackendError::WrongPassword)),
        Action::Verify(Err(AuthBackendError::WrongPassword)),
        Action::Verify(Err(AuthBackendError::WrongPassword)),
    ]);

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::ExitWithGuidance);
}

#[test]
fn flood_wait_exits_immediately() {
    let mut terminal = FakeTerminal::new(vec![Some("+15551234567")]);
    let mut client = FakeClient::new(vec![Action::RequestCode(Err(
        AuthBackendError::FloodWait { seconds: 120 },
    ))]);

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::ExitWithGuidance);
    assert!(terminal
        .output
        .iter()
        .any(|line| line.contains("AUTH_FLOOD_WAIT")));
}

#[test]
fn eof_cancels_flow_cleanly() {
    let mut terminal = FakeTerminal::new(vec![None]);
    let mut client = FakeClient::new(vec![]);

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::ExitWithGuidance);
}

#[test]
fn timeout_then_successful_retry() {
    let mut terminal = FakeTerminal::new(vec![Some("+15551234567"), Some("12345")]);
    let mut client = FakeClient::new(vec![
        Action::RequestCode(Err(AuthBackendError::Timeout)),
        Action::RequestCode(Ok(AuthCodeToken("token".into()))),
        Action::SignIn(Ok(SignInOutcome::Authorized)),
    ]);

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::Authenticated);
    assert!(terminal
        .output
        .iter()
        .any(|line| line.contains("AUTH_TIMEOUT")));
}

#[test]
fn invalid_phone_from_backend_retries_then_exits() {
    let mut terminal = FakeTerminal::new(vec![
        Some("+15551234567"),
        Some("+15551234567"),
        Some("+15551234567"),
    ]);
    let mut client = FakeClient::new(vec![
        Action::RequestCode(Err(AuthBackendError::InvalidPhone)),
        Action::RequestCode(Err(AuthBackendError::InvalidPhone)),
        Action::RequestCode(Err(AuthBackendError::InvalidPhone)),
    ]);

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::ExitWithGuidance);
    assert!(terminal
        .output
        .iter()
        .any(|line| line.contains("AUTH_INVALID_PHONE")));
}

#[test]
fn backend_unavailable_fails_fast_with_actionable_message_and_without_leaks() {
    let mut terminal = FakeTerminal::new(vec![Some("+15551234567")]);
    let mut client = FakeClient::new(vec![Action::RequestCode(Err(
        AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            message: "password=s3cret code=12345".to_owned(),
        },
    ))]);

    let result = run_guided_auth(
        &mut terminal,
        &mut client,
        &RetryPolicy {
            phone_attempts: 3,
            code_attempts: 1,
            password_attempts: 1,
        },
    )
    .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::ExitWithGuidance);

    let joined = terminal.output.join("\n");
    assert!(joined.contains("AUTH_BACKEND_UNAVAILABLE"));
    assert!(joined.contains("Check Telegram API config"));
    assert!(!joined.contains("Attempts left:"));
    assert!(!joined.contains("password=s3cret"));
    assert!(!joined.contains("code=12345"));
}

#[test]
fn status_snapshot_is_printed_for_ui_actions_when_available() {
    let mut terminal = FakeTerminal::new(vec![Some("+15551234567"), Some("12345")]);
    let snapshot = crate::domain::status::AuthConnectivityStatus {
        auth: crate::domain::status::AuthStatus::InProgress,
        connectivity: crate::domain::status::ConnectivityHealth::Ok,
        updated_at_unix_ms: 1,
        last_error: None,
    };
    let mut client = FakeClient::with_snapshot(
        vec![
            Action::RequestCode(Ok(AuthCodeToken("token".into()))),
            Action::SignIn(Ok(SignInOutcome::Authorized)),
        ],
        snapshot,
    );

    let result = run_guided_auth(&mut terminal, &mut client, &RetryPolicy::default())
        .expect("guided auth should complete");

    assert_eq!(result, GuidedAuthOutcome::Authenticated);
    assert!(terminal
        .output
        .iter()
        .any(|line| line.contains("status[start]: auth=AUTH_IN_PROGRESS")));
    assert!(terminal
        .output
        .iter()
        .any(|line| line.contains("status[code]: auth=AUTH_IN_PROGRESS")));
}
