use anyhow::Result;

use crate::{
    cli::{Cli, Command},
    domain, infra,
    telegram::{self, TelegramAdapter},
    ui,
    usecases::{
        self, bootstrap,
        guided_auth::{run_guided_auth, GuidedAuthOutcome, RetryPolicy, StdTerminal},
        logout::logout_and_reset,
    },
};

const AUTH_TUI_BOOTSTRAP_FAILED: &str = "AUTH_TUI_BOOTSTRAP_FAILED";

pub fn run(cli: Cli) -> Result<()> {
    tracing::debug!(
        ui = ui::module_name(),
        domain = domain::module_name(),
        telegram = telegram::module_name(),
        usecases = usecases::module_name(),
        infra = infra::module_name(),
        "module boundaries loaded"
    );

    match cli.command_or_default() {
        Command::Run => {
            let mut context = bootstrap::bootstrap(cli.config.as_deref())?;
            let startup = usecases::startup::plan_startup(
                &context.telegram,
                Some(context.config.startup.session_probe_timeout_ms),
            )?;

            if let Some(code) = startup.probe_warning {
                tracing::warn!(code, "startup probe fell back to local session validity");
            }

            match startup.state {
                usecases::startup::StartupFlowState::LaunchTui => {
                    let mut shell = bootstrap::compose_shell(&context);
                    ui::shell::start(
                        &context,
                        shell.event_source.as_mut(),
                        shell.orchestrator.as_mut(),
                    )?
                }
                usecases::startup::StartupFlowState::GuidedAuth { ref reason } => {
                    tracing::info!(
                        code = reason.code(),
                        message = reason.user_message(),
                        "starting guided CLI authorization"
                    );

                    let mut terminal = StdTerminal;
                    let auth_outcome = run_guided_auth(
                        &mut terminal,
                        &mut context.telegram,
                        &startup.session_file(),
                        &RetryPolicy::default(),
                    )?;

                    if matches!(auth_outcome, GuidedAuthOutcome::Authenticated) {
                        let mut shell = bootstrap::compose_shell(&context);
                        if let Err(error) = ui::shell::start(
                            &context,
                            shell.event_source.as_mut(),
                            shell.orchestrator.as_mut(),
                        ) {
                            report_post_auth_tui_bootstrap_failure(&error);
                        }
                    }
                }
            }
        }
        Command::Logout => {
            let mut telegram = build_logout_telegram(cli.config.as_deref());
            let outcome = logout_and_reset(&mut telegram)?;
            tracing::info!(
                session_removed = outcome.session_removed,
                policy_marker_removed = outcome.policy_marker_removed,
                "logout/reset completed"
            );
            println!("Logout completed. State is disconnected and ready for clean re-login.");
        }
    }

    Ok(())
}

fn build_logout_telegram(config_path: Option<&std::path::Path>) -> TelegramAdapter {
    match bootstrap::bootstrap(config_path) {
        Ok(context) => context.telegram,
        Err(error) => {
            tracing::warn!(
                error = ?error,
                "logout fallback: telegram bootstrap failed, continuing with local cleanup"
            );
            TelegramAdapter::stub()
        }
    }
}

fn report_post_auth_tui_bootstrap_failure(error: &anyhow::Error) {
    tracing::error!(
        code = AUTH_TUI_BOOTSTRAP_FAILED,
        error = ?error,
        "post-auth TUI bootstrap failed after successful session persist"
    );

    for line in post_auth_tui_fallback_lines(AUTH_TUI_BOOTSTRAP_FAILED) {
        eprintln!("{line}");
    }
}

fn post_auth_tui_fallback_lines(error_code: &str) -> [String; 3] {
    [
        "Authentication successful. Session is saved.".to_owned(),
        format!("{error_code}: TUI failed to start in this run."),
        "Please restart RTG to enter TUI using the saved session.".to_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;
    use crate::{cli::Cli, test_support::env_lock};

    #[test]
    fn post_auth_fallback_lines_include_non_secret_error_code_and_guidance() {
        let lines = post_auth_tui_fallback_lines(AUTH_TUI_BOOTSTRAP_FAILED);

        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Session is saved"));
        assert!(lines[1].contains(AUTH_TUI_BOOTSTRAP_FAILED));
        assert!(lines[2].contains("restart RTG"));
    }

    #[test]
    fn logout_succeeds_when_telegram_bootstrap_fails() {
        let _guard = env_lock();

        let root = env::temp_dir().join(format!(
            "rtg-app-logout-bootstrap-fail-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        ));
        let xdg = root.join("xdg");
        fs::create_dir_all(&xdg).expect("xdg dir should be creatable");

        let old_xdg = env::var_os("XDG_CONFIG_HOME");
        // SAFETY: env is guarded by process-wide test mutex.
        unsafe { env::set_var("XDG_CONFIG_HOME", &xdg) };

        let config_path = root.join("invalid-config.toml");
        fs::write(&config_path, "[telegram]\napi_id = 1\n")
            .expect("invalid config fixture should be writable");

        let layout = crate::infra::storage_layout::StorageLayout::resolve().expect("layout");
        layout.ensure_dirs().expect("layout dirs should be created");
        fs::write(layout.session_file(), b"session").expect("session should be written");

        let cli = Cli {
            config: Some(config_path),
            command: Some(crate::cli::Command::Logout),
        };

        run(cli).expect("logout should succeed despite bootstrap failure");
        assert!(!layout.session_file().exists());

        match old_xdg {
            Some(value) => {
                // SAFETY: restoring env while guard is held.
                unsafe { env::set_var("XDG_CONFIG_HOME", value) }
            }
            None => {
                // SAFETY: restoring env while guard is held.
                unsafe { env::remove_var("XDG_CONFIG_HOME") }
            }
        }

        let _ = fs::remove_dir_all(root);
    }
}
