use clap::{Parser, Subcommand};
use jiracc::application;
use std::path::PathBuf;

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
    /// Create a sample configuration at the default path
    Init,

    /// Print a sample configuration to stdout
    Sample,

    /// Validate a configuration file
    Validate {
        /// Path to the configuration file
        #[arg(long, value_name = "PATH")]
        config_path: Option<PathBuf>,
    },
}

impl From<Args> for application::Command {
    fn from(args: Args) -> Self {
        match args.command {
            Command::Config {
                command: ConfigCommand::Init,
            } => Self::Config(application::ConfigCommand::Init),
            Command::Config {
                command: ConfigCommand::Sample,
            } => Self::Config(application::ConfigCommand::Sample),
            Command::Config {
                command: ConfigCommand::Validate { config_path },
            } => Self::Config(application::ConfigCommand::Validate { config_path }),
        }
    }
}
