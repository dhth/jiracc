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
Fetched: {fetched_at}"
    )
}

fn format_relative_time(
    snapshot_time: DateTime<Utc>,
    reference_time: DateTime<Utc>,
) -> Option<String> {
    if reference_time < snapshot_time {
        return None;
    }

    let elapsed_seconds = reference_time
        .signed_duration_since(snapshot_time)
        .num_seconds();

    let relative_time = if elapsed_seconds < 60 {
        "just now".to_owned()
    } else if elapsed_seconds < 60 * 60 {
        format!("{}m ago", elapsed_seconds / 60)
    } else if elapsed_seconds < 24 * 60 * 60 {
        let hours = elapsed_seconds / (60 * 60);
        let minutes = (elapsed_seconds % (60 * 60)) / 60;

        if minutes == 0 {
            format!("{hours}h ago")
        } else {
            format!("{hours}h {minutes}m ago")
        }
    } else {
        let days = elapsed_seconds / (24 * 60 * 60);
        let hours = (elapsed_seconds % (24 * 60 * 60)) / (60 * 60);

        if hours == 0 {
            format!("{days}d ago")
        } else {
            format!("{days}d {hours}h ago")
        }
    };

    Some(relative_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Issue, SnapshotMetadata};
    use chrono::TimeDelta;

    #[test]
    fn format_status_reports_issue_count_and_fetch_time() -> anyhow::Result<()> {
        let now = "2026-09-18T15:15:00Z".parse::<DateTime<Utc>>()?;
        let fetched_at = "2026-09-18T12:15:00Z".parse::<DateTime<Utc>>()?;
        let cases = [
            (
                5,
                "5 issues cached
Fetched: 2026-09-18T12:15:00Z (3h ago)",
            ),
            (
                1,
                "1 issue cached
Fetched: 2026-09-18T12:15:00Z (3h ago)",
            ),
            (
                0,
                "0 issues cached
Fetched: 2026-09-18T12:15:00Z (3h ago)",
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
Fetched: 2026-09-18T15:16:00Z"
        );

        Ok(())
    }

    #[test]
    fn format_relative_time_handles_elapsed_time_boundaries() -> anyhow::Result<()> {
        let reference_time = "2026-09-18T15:15:00Z".parse::<DateTime<Utc>>()?;
        let cases = [
            (TimeDelta::milliseconds(-1), None),
            (TimeDelta::zero(), Some("just now")),
            (TimeDelta::seconds(59), Some("just now")),
            (TimeDelta::seconds(60), Some("1m ago")),
            (TimeDelta::seconds(24 * 60 + 59), Some("24m ago")),
            (TimeDelta::seconds(3_599), Some("59m ago")),
            (TimeDelta::seconds(3_600), Some("1h ago")),
            (
                TimeDelta::seconds(13 * 3_600 + 45 * 60 + 59),
                Some("13h 45m ago"),
            ),
            (TimeDelta::seconds(86_399), Some("23h 59m ago")),
            (TimeDelta::seconds(86_400), Some("1d ago")),
            (TimeDelta::seconds(4 * 86_400), Some("4d ago")),
            (
                TimeDelta::seconds(4 * 86_400 + 14 * 3_600 + 3_599),
                Some("4d 14h ago"),
            ),
        ];

        for (elapsed, expected) in cases {
            let snapshot_time = reference_time - elapsed;

            assert_eq!(
                format_relative_time(snapshot_time, reference_time).as_deref(),
                expected
            );
        }

        Ok(())
    }

    fn snapshot(issue_count: usize, fetched_at: DateTime<Utc>) -> Snapshot {
        Snapshot {
            metadata: SnapshotMetadata {
                fetched_at,
                jira_url: "https://jira.example.com".to_owned(),
                jql: "project = TEST".to_owned(),
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
