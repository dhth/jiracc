use crate::application::{SnapshotStore, issue_formatter};
use crate::config;
use crate::paths;
use crate::persistence::{FileSnapshotStore, FileSnapshotStoreError};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ShowError {
    #[error(transparent)]
    Paths(#[from] paths::PathsError),

    #[error(transparent)]
    ConfigPath(#[from] config::ConfigError),

    #[error(transparent)]
    Store(#[from] FileSnapshotStoreError),

    #[error("issue {key} was not found in the cached snapshot")]
    IssueNotFound { key: String },

    #[error("couldn't write issue details to stdout")]
    WriteOutput(#[source] std::io::Error),
}

pub fn show(key: String, config_path: Option<PathBuf>) -> Result<(), ShowError> {
    let paths = paths::get()?;
    let config_path = config_path.unwrap_or(paths.config);
    let config_path = config::resolve_path(&config_path)?;
    let store = FileSnapshotStore::new(&paths.data, &config_path);
    let issue = store
        .get_snapshot()?
        .issues
        .into_iter()
        .find(|issue| issue.key == key)
        .ok_or_else(|| ShowError::IssueNotFound { key })?;

    let output = issue_formatter::format_issue(&issue);
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{output}").map_err(ShowError::WriteOutput)
}
