use anyhow::Result;

use crate::{
    cli::{Cli, Command},
    domain, infra, telegram, ui,
    usecases::{
        self, bootstrap,
        guided_auth::{run_guided_auth, GuidedAuthOutcome, RetryPolicy, StdTerminal},
    },
};

const AUTH_TUI_BOOTSTRAP_FAILED: &str = "AUTH_TUI_BOOTSTRAP_FAILED";

pub fn run(cli: Cli) -> Result<()> {
    let mut context = bootstrap::bootstrap(cli.config.as_deref())?;
    let startup = usecases::startup::plan_startup(
        &context.telegram,
        Some(context.config.startup.session_probe_timeout_ms),
    )?;

    if let Some(code) = startup.probe_warning {
        tracing::warn!(code, "startup probe fell back to local session validity");
    }

    tracing::debug!(
        ui = ui::module_name(),
        domain = domain::module_name(),
        telegram = telegram::module_name(),
        usecases = usecases::module_name(),
        infra = infra::module_name(),
        "module boundaries loaded"
    );

    match cli.command_or_default() {
        Command::Run => match startup.state {
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
        },
    }

    Ok(())
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
    use super::*;

    #[test]
    fn post_auth_fallback_lines_include_non_secret_error_code_and_guidance() {
        let lines = post_auth_tui_fallback_lines(AUTH_TUI_BOOTSTRAP_FAILED);

        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Session is saved"));
        assert!(lines[1].contains(AUTH_TUI_BOOTSTRAP_FAILED));
        assert!(lines[2].contains("restart RTG"));
    }
}
