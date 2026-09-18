mod config;
mod issue_filter;
mod search;
mod show;
mod store;
mod sync;

use std::path::PathBuf;

pub use config::{InitConfigError, SampleConfigError, ValidateConfigError};
pub use issue_filter::{IssueFilter, IssueFilterError};
pub use search::{SearchError, SearchOperationError};
pub use show::ShowError;
pub use store::SnapshotStore;
pub use sync::{IssueFetcher, SyncError, SyncOperationError, sync};

pub enum Command {
    Config(ConfigCommand),
    Sync {
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

pub enum ConfigCommand {
    Init,
    Sample,
    Validate { config_path: Option<PathBuf> },
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error(transparent)]
    InitConfig(#[from] InitConfigError),

    #[error(transparent)]
    SampleConfig(#[from] SampleConfigError),

    #[error(transparent)]
    ValidateConfig(#[from] ValidateConfigError),

    #[error(transparent)]
    Sync(#[from] SyncError),

    #[error(transparent)]
    Show(#[from] ShowError),

    #[error(transparent)]
    Search(#[from] SearchError),
}

pub async fn run(command: Command) -> Result<(), ApplicationError> {
    match command {
        Command::Config(ConfigCommand::Init) => config::init()?,
        Command::Config(ConfigCommand::Sample) => config::sample()?,
        Command::Config(ConfigCommand::Validate { config_path }) => config::validate(config_path)?,
        Command::Sync { config_path } => sync::sync(config_path).await?,
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
