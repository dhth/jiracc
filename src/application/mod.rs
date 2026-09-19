mod auth;
mod config;
mod issue_filter;
mod presentation;
mod search;
mod show;
mod status;
mod sync;

use std::path::PathBuf;

pub use auth::CheckAuthError;
pub use config::{InitConfigError, SampleConfigError, ValidateConfigError};
pub use issue_filter::{IssueFilter, IssueFilterError};
pub use search::{SearchError, SearchOperationError};
pub use show::ShowError;
pub use status::{StatusError, StatusOperationError};
pub use sync::{SyncError, SyncOperationError, sync};

pub enum Command {
    Auth(AuthCommand),
    Config(ConfigCommand),
    Sync {
        config_path: Option<PathBuf>,
    },
    Status {
        config_path: Option<PathBuf>,
    },
    Show {
        key: String,
        config_path: Option<PathBuf>,
    },
    Search {
        query: Option<String>,
        assignees: Vec<String>,
        statuses: Vec<String>,
        issue_types: Vec<String>,
        config_path: Option<PathBuf>,
    },
}

pub enum AuthCommand {
    Check { config_path: Option<PathBuf> },
}

pub enum ConfigCommand {
    Init,
    Sample,
    Validate { config_path: Option<PathBuf> },
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error(transparent)]
    CheckAuth(#[from] CheckAuthError),

    #[error(transparent)]
    InitConfig(#[from] InitConfigError),

    #[error(transparent)]
    SampleConfig(#[from] SampleConfigError),

    #[error(transparent)]
    ValidateConfig(#[from] ValidateConfigError),

    #[error(transparent)]
    Sync(#[from] SyncError),

    #[error(transparent)]
    Status(#[from] StatusError),

    #[error(transparent)]
    Show(#[from] ShowError),

    #[error(transparent)]
    Search(#[from] SearchError),
}

pub async fn run(command: Command) -> Result<(), ApplicationError> {
    match command {
        Command::Auth(AuthCommand::Check { config_path }) => auth::check(config_path).await?,
        Command::Config(ConfigCommand::Init) => config::init()?,
        Command::Config(ConfigCommand::Sample) => config::sample()?,
        Command::Config(ConfigCommand::Validate { config_path }) => config::validate(config_path)?,
        Command::Sync { config_path } => sync::sync(config_path).await?,
        Command::Status { config_path } => status::status(config_path)?,
        Command::Show { key, config_path } => show::show(key, config_path)?,
        Command::Search {
            query,
            assignees,
            statuses,
            issue_types,
            config_path,
        } => search::search(query, assignees, statuses, issue_types, config_path)?,
    }

    Ok(())
}
