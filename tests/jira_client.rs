use anyhow::{Context, Result, bail};
use jiracc::application::IssueFetcher;
use jiracc::config::{JiraJql, JiraToken, JiraUrl};
use jiracc::jira::JiraClient;
use serde_json::{Value, json};
use wiremock::matchers::{bearer_token, body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";
const JQL: &str = "project = TEST
ORDER BY updated DESC";

#[tokio::test]
async fn fetches_and_normalizes_a_single_page() -> Result<()> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/search"))
        .and(bearer_token(TOKEN))
        .and(header("content-type", "application/json"))
        .and(body_json(search_request(JQL, 0, 100)))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_response()))
        .expect(1)
        .mount(&server)
        .await;

    let url = JiraUrl::try_from(server.uri())?;
    let token = JiraToken::try_from(TOKEN.to_owned())?;
    let jql = JiraJql::try_from(JQL.to_owned())?;
    let client = JiraClient::new(&url, &token);

    let issues = client.fetch_issues(&jql).await?;

    let [issue] = issues.as_slice() else {
        bail!("expected one issue, got {}", issues.len());
    };
    assert_eq!(issue.id, "10001");
    assert_eq!(issue.key, "TEST-1");
    assert_eq!(issue.summary, "Example issue");
    assert_eq!(issue.status, "In Progress");
    assert_eq!(issue.issue_type, "Task");
    assert_eq!(issue.description.as_deref(), Some("Example description"));
    assert_eq!(issue.updated_at, "2026-09-15T10:00:00.000+0000");

    let assignee = issue.assignee.as_ref().context("expected an assignee")?;
    assert_eq!(assignee.username, "alice");
    assert_eq!(assignee.display_name, "Alice Example");

    let parent = issue.parent.as_ref().context("expected a parent")?;
    assert_eq!(parent.id, "10000");
    assert_eq!(parent.key, "TEST-0");

    let [subtask] = issue.subtasks.as_slice() else {
        bail!("expected one subtask, got {}", issue.subtasks.len());
    };
    assert_eq!(subtask.id, "10002");
    assert_eq!(subtask.key, "TEST-2");

    Ok(())
}

fn search_request(jql: &str, start_at: usize, max_results: usize) -> Value {
    json!({
        "jql": jql,
        "startAt": start_at,
        "maxResults": max_results,
        "fields": [
            "summary",
            "status",
            "issuetype",
            "assignee",
            "description",
            "updated",
            "parent",
            "subtasks"
        ]
    })
}

fn search_response() -> Value {
    json!({
        "startAt": 0,
        "maxResults": 100,
        "total": 1,
        "issues": [{
            "id": "10001",
            "key": "TEST-1",
            "fields": {
                "summary": "Example issue",
                "status": { "name": "In Progress" },
                "issuetype": { "name": "Task" },
                "assignee": {
                    "name": "alice",
                    "displayName": "Alice Example"
                },
                "description": "Example description",
                "updated": "2026-09-15T10:00:00.000+0000",
                "parent": {
                    "id": "10000",
                    "key": "TEST-0"
                },
                "subtasks": [{
                    "id": "10002",
                    "key": "TEST-2"
                }]
            }
        }]
    })
}
