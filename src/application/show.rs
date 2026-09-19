use super::presentation::format_relative_time;
use crate::config;
use crate::domain::Issue;
use crate::paths;
use crate::persistence::{FileSnapshotStore, FileSnapshotStoreError};
use chrono::{DateTime, Utc};
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
    WriteOutput(#[from] std::io::Error),
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

    let output = format_issue(&issue, Utc::now());
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{output}")?;

    Ok(())
}

fn format_issue(issue: &Issue, reference_time: DateTime<Utc>) -> String {
    let assignee = if let Some(assignee) = &issue.assignee {
        format!("{} (@{})", assignee.display_name, assignee.username)
    } else {
        "Unassigned".to_owned()
    };
    let parent = issue
        .parent
        .as_ref()
        .map_or("None", |parent| parent.key.as_str());
    let subtasks = if issue.subtasks.is_empty() {
        "None".to_owned()
    } else {
        issue
            .subtasks
            .iter()
            .map(|subtask| subtask.key.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let description = issue.description.as_deref().unwrap_or("No description.");
    let updated_at = format_updated_at(&issue.updated_at, reference_time);

    format!(
        "{key} — {summary}

Type:       {issue_type}
Status:     {status}
Assignee:   {assignee}
Updated:    {updated_at}
Jira ID:    {id}
Parent:     {parent}
Subtasks:   {subtasks}

Description
-----------
{description}",
        key = issue.key,
        summary = issue.summary,
        issue_type = issue.issue_type,
        status = issue.status,
        id = issue.id,
    )
}

fn format_updated_at(updated_at: &str, reference_time: DateTime<Utc>) -> String {
    let relative_time = DateTime::parse_from_str(updated_at, "%Y-%m-%dT%H:%M:%S%.f%z")
        .ok()
        .and_then(|timestamp| format_relative_time(timestamp.with_timezone(&Utc), reference_time));

    match relative_time {
        Some(relative_time) => format!("{updated_at} ({relative_time})"),
        None => updated_at.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Assignee, IssueReference};
    use insta::assert_snapshot;

    #[test]
    fn format_issue_with_all_fields() -> anyhow::Result<()> {
        let issue = Issue {
            id: "10001".to_owned(),
            key: "TEST-42".to_owned(),
            summary: "Investigate intermittent timeout".to_owned(),
            status: "In Progress".to_owned(),
            issue_type: "Bug".to_owned(),
            assignee: Some(Assignee {
                username: "alice".to_owned(),
                display_name: "Alice Example".to_owned(),
            }),
            description: Some(
                "First line of the description.\nSecond line with more detail.".to_owned(),
            ),
            updated_at: "2026-09-15T10:00:00.000+0000".to_owned(),
            parent: Some(IssueReference {
                id: "10000".to_owned(),
                key: "TEST-40".to_owned(),
            }),
            subtasks: vec![
                IssueReference {
                    id: "10002".to_owned(),
                    key: "TEST-43".to_owned(),
                },
                IssueReference {
                    id: "10003".to_owned(),
                    key: "TEST-44".to_owned(),
                },
            ],
        };
        let now = "2026-09-18T15:15:00Z".parse::<DateTime<Utc>>()?;

        assert_snapshot!(format_issue(&issue, now), @"
        TEST-42 — Investigate intermittent timeout

        Type:       Bug
        Status:     In Progress
        Assignee:   Alice Example (@alice)
        Updated:    2026-09-15T10:00:00.000+0000 (3d 5h ago)
        Jira ID:    10001
        Parent:     TEST-40
        Subtasks:   TEST-43, TEST-44

        Description
        -----------
        First line of the description.
        Second line with more detail.
        ");

        Ok(())
    }

    #[test]
    fn format_issue_without_optional_fields() -> anyhow::Result<()> {
        let issue = Issue {
            id: "20001".to_owned(),
            key: "TEST-50".to_owned(),
            summary: "Document the deployment process".to_owned(),
            status: "Open".to_owned(),
            issue_type: "Task".to_owned(),
            assignee: None,
            description: None,
            updated_at: "2026-09-16T14:30:00.000+0000".to_owned(),
            parent: None,
            subtasks: Vec::new(),
        };
        let now = "2026-09-18T15:15:00Z".parse::<DateTime<Utc>>()?;

        assert_snapshot!(format_issue(&issue, now), @"
        TEST-50 — Document the deployment process

        Type:       Task
        Status:     Open
        Assignee:   Unassigned
        Updated:    2026-09-16T14:30:00.000+0000 (2d ago)
        Jira ID:    20001
        Parent:     None
        Subtasks:   None

        Description
        -----------
        No description.
        ");

        Ok(())
    }

    #[test]
    fn format_updated_at_respects_offset() -> anyhow::Result<()> {
        let now = "2026-09-21T16:59:39Z".parse::<DateTime<Utc>>()?;

        assert_eq!(
            format_updated_at("2026-09-18T12:59:39.000-0400", now),
            "2026-09-18T12:59:39.000-0400 (3d ago)"
        );

        Ok(())
    }

    #[test]
    fn format_updated_at_falls_back_to_raw_value() -> anyhow::Result<()> {
        let now = "2026-09-21T16:59:39Z".parse::<DateTime<Utc>>()?;
        let cases = ["not-a-timestamp", "2026-09-21T17:00:00.000+0000"];

        for updated_at in cases {
            assert_eq!(format_updated_at(updated_at, now), updated_at);
        }

        Ok(())
    }
}
