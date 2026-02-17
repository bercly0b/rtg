use anyhow::{bail, Result};

use crate::{
    cli::{Cli, Command},
    domain, infra, telegram, ui,
    usecases::{self, bootstrap},
};

pub fn run(cli: Cli) -> Result<()> {
    let context = bootstrap::bootstrap(cli.config.as_deref())?;
    let _startup = usecases::startup::plan_startup()?;

    tracing::debug!(
        ui = ui::module_name(),
        domain = domain::module_name(),
        telegram = telegram::module_name(),
        usecases = usecases::module_name(),
        infra = infra::module_name(),
        "module boundaries loaded"
    );

    match cli.command_or_default() {
        Command::Run => match _startup.state {
            usecases::startup::StartupFlowState::LaunchTui => {
                let mut shell = bootstrap::compose_shell();
                ui::shell::start(
                    &context,
                    shell.event_source.as_mut(),
                    shell.orchestrator.as_mut(),
                )?
            }
            usecases::startup::StartupFlowState::GuidedAuth => {
                bail!(
                    "guided CLI authorization is not implemented yet (startup detected missing session)"
                )
            }
        },
    }

    Ok(())
}
