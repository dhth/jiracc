use crate::config::{self, JiraJql, JiraUrl};
use crate::domain::{Snapshot, SnapshotMetadata};
use crate::jira::{JiraClient, JiraClientError};
use crate::paths;
use crate::persistence::{FileSnapshotStore, FileSnapshotStoreError};
use chrono::Utc;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error(transparent)]
    Paths(#[from] paths::PathsError),

    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error(transparent)]
    Operation(#[from] SyncOperationError),

    #[error("couldn't write synchronization result to stdout")]
    WriteOutput(#[source] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum SyncOperationError {
    #[error("couldn't fetch issues")]
    FetchIssues(#[source] JiraClientError),

    #[error("couldn't save snapshot")]
    SaveSnapshot(#[source] FileSnapshotStoreError),
}

pub async fn sync(config_path: Option<PathBuf>) -> Result<(), SyncError> {
    let paths = paths::get()?;
    let config_path = config_path.unwrap_or(paths.config);
    let loaded = config::load(&config_path)?;
    let jira = loaded.config.jira;
    let fetcher = JiraClient::new(&jira.url, &jira.token);
    let store = FileSnapshotStore::new(&paths.data, &loaded.path);
    let issue_count = sync_with(&fetcher, &store, &jira.url, &jira.jql).await?;

    let mut stdout = std::io::stdout().lock();
    match issue_count {
        0 => writeln!(stdout, "No issues found."),
        1 => writeln!(stdout, "Synchronized 1 issue."),
        _ => writeln!(stdout, "Synchronized {issue_count} issues."),
    }
    .map_err(SyncError::WriteOutput)
}

async fn sync_with(
    fetcher: &JiraClient,
    store: &FileSnapshotStore,
    jira_url: &JiraUrl,
    jql: &JiraJql,
) -> Result<usize, SyncOperationError> {
    let issues = fetcher
        .fetch_issues(jql)
        .await
        .map_err(SyncOperationError::FetchIssues)?;
    let issue_count = issues.len();
    let snapshot = Snapshot {
        metadata: SnapshotMetadata {
            fetched_at: Utc::now(),
            jira_url: jira_url.as_str().to_owned(),
            jql: jql.as_str().to_owned(),
        },
        issues,
    };
    store
        .save_snapshot(&snapshot)
        .map_err(SyncOperationError::SaveSnapshot)?;

    Ok(issue_count)
}
