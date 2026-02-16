use anyhow::Result;

use crate::{
    cli::{Cli, Command},
    domain, infra, telegram, ui,
    usecases::{self, bootstrap},
};

pub fn run(cli: Cli) -> Result<()> {
    let context = bootstrap::bootstrap(cli.config.as_deref())?;

    tracing::debug!(
        ui = ui::module_name(),
        domain = domain::module_name(),
        telegram = telegram::module_name(),
        usecases = usecases::module_name(),
        infra = infra::module_name(),
        "module boundaries loaded"
    );

    match cli.command_or_default() {
        Command::Run => ui::shell::start(&context),
    }

    Ok(())
}
