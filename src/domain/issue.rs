use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub key: String,
    pub summary: String,
    pub status: String,
    pub issue_type: String,
    pub assignee: Option<Assignee>,
    pub description: Option<String>,
    pub updated_at: String,
    pub parent: Option<IssueReference>,
    pub subtasks: Vec<IssueReference>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Assignee {
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueReference {
    pub id: String,
    pub key: String,
}
