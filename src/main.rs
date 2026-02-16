mod domain;
mod infra;
mod telegram;
mod ui;
mod usecases;

use anyhow::Result;

fn main() -> Result<()> {
    let config = infra::config::load(None)?;
    infra::logging::init(&config.logging)?;

    tracing::debug!(
        ui = ui::module_name(),
        domain = domain::module_name(),
        telegram = telegram::module_name(),
        usecases = usecases::module_name(),
        infra = infra::module_name(),
        "module boundaries loaded"
    );
    tracing::info!(?config, "bootstrap completed");
    println!("RTG workspace initialized");

    Ok(())
}
