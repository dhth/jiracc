use crate::domain::{Assignee, Issue, IssueReference};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurrentUser {
    pub name: String,
    pub display_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchRequest<'a> {
    pub jql: &'a str,
    pub start_at: usize,
    pub max_results: usize,
    pub fields: &'static [&'static str],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchResponse {
    pub start_at: usize,
    pub max_results: usize,
    pub total: usize,
    pub issues: Vec<JiraIssue>,
}

#[derive(Deserialize)]
pub(super) struct JiraIssue {
    id: String,
    key: String,
    fields: IssueFields,
}

#[derive(Deserialize)]
struct IssueFields {
    summary: String,
    status: NamedField,
    #[serde(rename = "issuetype")]
    issue_type: NamedField,
    assignee: Option<JiraAssignee>,
    description: Option<String>,
    updated: String,
    parent: Option<JiraIssueReference>,
    #[serde(default)]
    subtasks: Vec<JiraIssueReference>,
}

#[derive(Deserialize)]
struct NamedField {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JiraAssignee {
    name: String,
    display_name: String,
}

#[derive(Deserialize)]
struct JiraIssueReference {
    id: String,
    key: String,
}

impl From<JiraIssue> for Issue {
    fn from(issue: JiraIssue) -> Self {
        let fields = issue.fields;

        Self {
            id: issue.id,
            key: issue.key,
            summary: fields.summary,
            status: fields.status.name,
            issue_type: fields.issue_type.name,
            assignee: fields.assignee.map(Assignee::from),
            description: fields.description,
            updated_at: fields.updated,
            parent: fields.parent.map(IssueReference::from),
            subtasks: fields
                .subtasks
                .into_iter()
                .map(IssueReference::from)
                .collect(),
        }
    }
}

impl From<JiraAssignee> for Assignee {
    fn from(assignee: JiraAssignee) -> Self {
        Self {
            username: assignee.name,
            display_name: assignee.display_name,
        }
    }
}

impl From<JiraIssueReference> for IssueReference {
    fn from(reference: JiraIssueReference) -> Self {
        Self {
            id: reference.id,
            key: reference.key,
        }
    }
}
