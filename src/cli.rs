use clap::{Parser, Subcommand};
use jiracc::application;

/// jiracc lets you access your JIRA issues offline
#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Work with jiracc's configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Print a sample configuration to stdout
    Sample,
}

impl From<Args> for application::Command {
    fn from(args: Args) -> Self {
        match args.command {
            Command::Config {
                command: ConfigCommand::Sample,
            } => Self::Config(application::ConfigCommand::Sample),
        }
    }
}
