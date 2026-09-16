use crate::application::SnapshotStore;
use crate::config;
use crate::domain::Issue;
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
        .get_issue(&key)?
        .ok_or_else(|| ShowError::IssueNotFound { key })?;

    let mut stdout = std::io::stdout().lock();
    let output = format_issue(&issue);
    writeln!(stdout, "{output}").map_err(ShowError::WriteOutput)
}

fn format_issue(issue: &Issue) -> String {
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
        updated_at = issue.updated_at,
        id = issue.id,
    )
}
