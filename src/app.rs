use anyhow::Result;

use crate::{
    cli::{Cli, Command},
    domain, infra, telegram, ui,
    usecases::{
        self, bootstrap,
        guided_auth::{run_guided_auth, GuidedAuthOutcome, RetryPolicy, StdTerminal},
    },
};

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
                let mut shell = bootstrap::compose_shell();
                ui::shell::start(
                    &context,
                    shell.event_source.as_mut(),
                    shell.orchestrator.as_mut(),
                )?
            }
            usecases::startup::StartupFlowState::GuidedAuth { reason } => {
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
                    let mut shell = bootstrap::compose_shell();
                    ui::shell::start(
                        &context,
                        shell.event_source.as_mut(),
                        shell.orchestrator.as_mut(),
                    )?;
                }
            }
        },
    }

    Ok(())
}
