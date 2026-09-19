use super::presentation::format_relative_time;
use crate::config;
use crate::domain::Snapshot;
use crate::paths;
use crate::persistence::{FileSnapshotStore, FileSnapshotStoreError};
use chrono::{DateTime, SecondsFormat, Utc};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error(transparent)]
    Paths(#[from] paths::PathsError),

    #[error(transparent)]
    ConfigPath(#[from] config::ConfigError),

    #[error(transparent)]
    Operation(#[from] StatusOperationError),
}

#[derive(Debug, thiserror::Error)]
pub enum StatusOperationError {
    #[error(transparent)]
    LoadSnapshot(#[from] FileSnapshotStoreError),

    #[error("couldn't write cache status to stdout")]
    WriteOutput(#[from] std::io::Error),
}

pub fn status(config_path: Option<PathBuf>) -> Result<(), StatusError> {
    let paths = paths::get()?;
    let config_path = config_path.unwrap_or(paths.config);
    let config_path = config::resolve_path(&config_path)?;
    let store = FileSnapshotStore::new(&paths.data, &config_path);
    let mut stdout = std::io::stdout().lock();

    status_with(&store, &mut stdout, Utc::now())?;

    Ok(())
}

fn status_with<W>(
    store: &FileSnapshotStore,
    output: &mut W,
    reference_time: DateTime<Utc>,
) -> Result<(), StatusOperationError>
where
    W: Write,
{
    let status = match store.get_snapshot() {
        Ok(snapshot) => format_status(&snapshot, reference_time),
        Err(FileSnapshotStoreError::SnapshotNotFound) => "No local issue cache found".to_owned(),
        Err(error) => return Err(error.into()),
    };

    writeln!(output, "{status}")?;

    Ok(())
}

fn format_status(snapshot: &Snapshot, reference_time: DateTime<Utc>) -> String {
    let issue_count = snapshot.issues.len();
    let issue_label = if issue_count == 1 { "issue" } else { "issues" };
    let fetched_at = snapshot
        .metadata
        .fetched_at
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let fetched_at = match format_relative_time(snapshot.metadata.fetched_at, reference_time) {
        Some(relative_time) => format!("{fetched_at} ({relative_time})"),
        None => fetched_at,
    };

    format!(
        "{issue_count} {issue_label} cached

Fetched: {fetched_at}
URL:     {jira_url}
JQL:
{jql}",
        jira_url = snapshot.metadata.jira_url,
        jql = snapshot.metadata.jql,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Issue, SnapshotMetadata};

    #[test]
    fn format_status_includes_snapshot_provenance() -> anyhow::Result<()> {
        let now = "2026-09-18T15:15:00Z".parse::<DateTime<Utc>>()?;
        let fetched_at = "2026-09-18T12:15:00Z".parse::<DateTime<Utc>>()?;
        let cases = [
            (
                5,
                "5 issues cached

Fetched: 2026-09-18T12:15:00Z (3h ago)
URL:     https://jira.example.com
JQL:
project = TEST
AND status = Open",
            ),
            (
                1,
                "1 issue cached

Fetched: 2026-09-18T12:15:00Z (3h ago)
URL:     https://jira.example.com
JQL:
project = TEST
AND status = Open",
            ),
            (
                0,
                "0 issues cached

Fetched: 2026-09-18T12:15:00Z (3h ago)
URL:     https://jira.example.com
JQL:
project = TEST
AND status = Open",
            ),
        ];

        for (issue_count, expected) in cases {
            assert_eq!(
                format_status(&snapshot(issue_count, fetched_at), now),
                expected
            );
        }

        Ok(())
    }

    #[test]
    fn format_status_omits_relative_time_for_future_snapshot() -> anyhow::Result<()> {
        let now = "2026-09-18T15:15:00Z".parse::<DateTime<Utc>>()?;
        let fetched_at = "2026-09-18T15:16:00Z".parse::<DateTime<Utc>>()?;

        assert_eq!(
            format_status(&snapshot(1, fetched_at), now),
            "1 issue cached

Fetched: 2026-09-18T15:16:00Z
URL:     https://jira.example.com
JQL:
project = TEST
AND status = Open"
        );

        Ok(())
    }

    fn snapshot(issue_count: usize, fetched_at: DateTime<Utc>) -> Snapshot {
        Snapshot {
            metadata: SnapshotMetadata {
                fetched_at,
                jira_url: "https://jira.example.com".to_owned(),
                jql: "project = TEST
AND status = Open"
                    .to_owned(),
            },
            issues: (0..issue_count).map(issue).collect(),
        }
    }

    fn issue(index: usize) -> Issue {
        Issue {
            id: index.to_string(),
            key: format!("TEST-{index}"),
            summary: "Summary".to_owned(),
            status: "Open".to_owned(),
            issue_type: "Task".to_owned(),
            assignee: None,
            description: None,
            updated_at: "2026-09-18T10:00:00.000+0000".to_owned(),
            parent: None,
            subtasks: Vec::new(),
        }
    }
}
