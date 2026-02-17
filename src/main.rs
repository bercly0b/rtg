mod app;
mod cli;
mod domain;
mod infra;
mod telegram;
#[cfg(test)]
mod test_support;
mod ui;
mod usecases;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    infra::secrets::install_panic_redaction_hook();

    let cli = cli::Cli::parse();
    app::run(cli)
}
