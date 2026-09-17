use crate::application::{IssueFilter, IssueFilterError, SnapshotStore, issue_formatter};
use crate::config;
use crate::paths;
use crate::persistence::{FileSnapshotStore, FileSnapshotStoreError};
use std::error::Error;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error(transparent)]
    InvalidFilter(#[from] IssueFilterError),

    #[error(transparent)]
    Paths(#[from] paths::PathsError),

    #[error(transparent)]
    ConfigPath(#[from] config::ConfigError),

    #[error(transparent)]
    Operation(#[from] SearchOperationError<FileSnapshotStoreError>),
}

#[derive(Debug, thiserror::Error)]
pub enum SearchOperationError<StoreError>
where
    StoreError: Error + 'static,
{
    #[error(transparent)]
    LoadSnapshot(StoreError),

    #[error("couldn't write search results to stdout")]
    WriteOutput(#[source] std::io::Error),
}

pub fn search(
    query: Option<String>,
    assignees: Vec<String>,
    statuses: Vec<String>,
    issue_types: Vec<String>,
    config_path: Option<PathBuf>,
) -> Result<(), SearchError> {
    let filter = IssueFilter::new(query, assignees, statuses, issue_types)?;
    let paths = paths::get()?;
    let config_path = config_path.unwrap_or(paths.config);
    let config_path = config::resolve_path(&config_path)?;
    let store = FileSnapshotStore::new(&paths.data, &config_path);
    let mut stdout = std::io::stdout().lock();

    search_with(&store, &mut stdout, &filter)?;

    Ok(())
}

fn search_with<S, W>(
    store: &S,
    output: &mut W,
    filter: &IssueFilter,
) -> Result<(), SearchOperationError<S::Error>>
where
    S: SnapshotStore,
    W: Write,
{
    let snapshot = store
        .get_snapshot()
        .map_err(SearchOperationError::LoadSnapshot)?;
    let issues = snapshot
        .issues
        .iter()
        .filter(|issue| filter.matches(issue))
        .collect::<Vec<_>>();

    if issues.is_empty() {
        return Ok(());
    }

    let formatted_issues = issues
        .into_iter()
        .map(issue_formatter::format_issue)
        .collect::<Vec<_>>()
        .join("\n\n\n");
    writeln!(output, "{formatted_issues}").map_err(SearchOperationError::WriteOutput)?;

    Ok(())
}
