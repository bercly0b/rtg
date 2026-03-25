mod app;
mod cli;
mod domain;
mod infra;
mod telegram;
#[cfg(test)]
mod test_support;
mod ui;
mod usecases;

use std::time::Instant;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let startup_instant = Instant::now();

    infra::secrets::install_panic_redaction_hook();

    let cli = cli::Cli::parse();
    app::run(cli, startup_instant)
}
