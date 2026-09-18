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

    /// Synchronize issues from Jira
    Sync {
        /// Path to the configuration file
        #[arg(short = 'p', long, value_name = "PATH")]
        config_path: Option<PathBuf>,
    },

    /// Show the cached details of an issue
    Show {
        /// Jira issue key
        #[arg(value_name = "ISSUE-KEY")]
        key: String,

        /// Path to the configuration file
        #[arg(short = 'p', long, value_name = "PATH")]
        config_path: Option<PathBuf>,
    },

    /// Search cached issues
    ///
    /// With no query or filters, displays all cached issues.
    Search {
        /// Text to find in issue keys, summaries, or descriptions
        #[arg(value_name = "QUERY")]
        query: Option<String>,

        /// Match a Jira username; may be repeated
        #[arg(short = 'a', long = "assignee", value_name = "USERNAME")]
        assignees: Vec<String>,

        /// Match a Jira status; may be repeated
        #[arg(short = 's', long = "status", value_name = "STATUS")]
        statuses: Vec<String>,

        /// Match a Jira issue type; may be repeated
        #[arg(short = 't', long = "type", value_name = "TYPE")]
        issue_types: Vec<String>,

        /// Path to the configuration file
        #[arg(short = 'p', long, value_name = "PATH")]
        config_path: Option<PathBuf>,
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
        #[arg(short = 'p', long, value_name = "PATH")]
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
            Command::Sync { config_path } => Self::Sync { config_path },
            Command::Show { key, config_path } => Self::Show { key, config_path },
            Command::Search {
                query,
                assignees,
                statuses,
                issue_types,
                config_path,
            } => Self::Search {
                query,
                assignees,
                statuses,
                issue_types,
                config_path,
            },
        }
    }
}
