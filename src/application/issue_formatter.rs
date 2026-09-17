use crate::domain::Issue;

pub(crate) fn format_issue(issue: &Issue) -> String {
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
        "==> {key} — {summary}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Assignee, IssueReference};

    #[test]
    fn formats_a_fully_populated_issue() {
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

        insta::assert_snapshot!(format_issue(&issue), @"
        ==> TEST-42 — Investigate intermittent timeout

        Type:       Bug
        Status:     In Progress
        Assignee:   Alice Example (@alice)
        Updated:    2026-09-15T10:00:00.000+0000
        Jira ID:    10001
        Parent:     TEST-40
        Subtasks:   TEST-43, TEST-44

        Description
        -----------
        First line of the description.
        Second line with more detail.
        ");
    }

    #[test]
    fn formats_an_issue_without_optional_details() {
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

        insta::assert_snapshot!(format_issue(&issue), @"
        ==> TEST-50 — Document the deployment process

        Type:       Task
        Status:     Open
        Assignee:   Unassigned
        Updated:    2026-09-16T14:30:00.000+0000
        Jira ID:    20001
        Parent:     None
        Subtasks:   None

        Description
        -----------
        No description.
        ");
    }
}
