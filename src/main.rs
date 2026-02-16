mod app;
mod cli;
mod domain;
mod infra;
mod telegram;
mod ui;
mod usecases;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    app::run(cli)
}
