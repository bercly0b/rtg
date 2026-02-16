use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rtg", about = "Rust Telegram client (CLI + TUI)")]
pub struct Cli {
    /// Path to config file (default: ./config.toml)
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Start TUI shell
    Run,
}

impl Cli {
    pub fn command_or_default(&self) -> Command {
        self.command.clone().unwrap_or(Command::Run)
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    #[test]
    fn defaults_to_run_when_command_is_missing() {
        let cli = Cli::parse_from(["rtg"]);

        assert!(matches!(cli.command_or_default(), Command::Run));
    }

    #[test]
    fn parses_explicit_run_command() {
        let cli = Cli::parse_from(["rtg", "run", "--config", "custom.toml"]);

        assert!(matches!(cli.command_or_default(), Command::Run));
        assert_eq!(
            cli.config
                .as_deref()
                .map(|p| p.to_string_lossy().to_string()),
            Some("custom.toml".to_owned())
        );
    }
}
