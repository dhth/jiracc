use anyhow::{Context, bail};
use jiracc::config::{JiraJql, JiraToken, JiraUrl};
use jiracc::jira::{JiraClient, JiraClientError};
use reqwest::StatusCode;
use serde_json::{Value, json};
use wiremock::matchers::{bearer_token, body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";
const JQL: &str = "project = TEST
ORDER BY updated DESC";

struct TestContext {
    server: MockServer,
    client: JiraClient,
    jql: JiraJql,
}

impl TestContext {
    async fn new() -> anyhow::Result<Self> {
        let server = MockServer::start().await;
        let url = JiraUrl::try_from(server.uri())?;
        let token = JiraToken::try_from(TOKEN.to_owned())?;
        let jql = JiraJql::try_from(JQL.to_owned())?;
        let client = JiraClient::new(&url, &token)?;

        Ok(Self {
            server,
            client,
            jql,
        })
    }

    async fn expect_search(&self, start_at: usize, max_results: usize, response: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path("/rest/api/2/search"))
            .and(bearer_token(TOKEN))
            .and(header("content-type", "application/json"))
            .and(body_json(search_request(JQL, start_at, max_results)))
            .respond_with(response)
            .expect(1)
            .mount(&self.server)
            .await;
    }

    async fn expect_current_user(&self, response: ResponseTemplate) {
        Mock::given(method("GET"))
            .and(path("/rest/api/2/myself"))
            .and(bearer_token(TOKEN))
            .respond_with(response)
            .expect(1)
            .mount(&self.server)
            .await;
    }
}

//-------------//
//  SUCCESSES  //
//-------------//

#[tokio::test]
async fn gets_the_authenticated_user() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_current_user(ResponseTemplate::new(200).set_body_json(json!({
            "name": "alice",
            "displayName": "Alice Example",
            "active": true,
            "emailAddress": "alice@example.com"
        })))
        .await;

    // WHEN
    let user = context.client.get_current_user().await?;

    // THEN
    assert_eq!(user.username, "alice");
    assert_eq!(user.display_name, "Alice Example");

    Ok(())
}

#[tokio::test]
async fn fetches_and_normalizes_a_single_page() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(
                0,
                100,
                1,
                vec![fully_populated_issue()],
            )),
        )
        .await;

    // WHEN
    let issues = context.client.fetch_issues(&context.jql).await?;

    // THEN
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

#[tokio::test]
async fn fetches_all_pages_using_the_page_size_returned_by_jira() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(
                0,
                2,
                3,
                vec![
                    minimal_jira_issue("10001", "TEST-1"),
                    minimal_jira_issue("10002", "TEST-2"),
                ],
            )),
        )
        .await;
    context
        .expect_search(
            2,
            2,
            ResponseTemplate::new(200).set_body_json(search_page(
                2,
                2,
                3,
                vec![minimal_jira_issue("10003", "TEST-3")],
            )),
        )
        .await;

    // WHEN
    let issues = context.client.fetch_issues(&context.jql).await?;

    // THEN
    let issue_keys = issues
        .iter()
        .map(|issue| issue.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(issue_keys, ["TEST-1", "TEST-2", "TEST-3"]);

    Ok(())
}

#[tokio::test]
async fn an_empty_result_set_returns_no_issues() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(0, 100, 0, vec![])),
        )
        .await;

    // WHEN
    let issues = context.client.fetch_issues(&context.jql).await?;

    // THEN
    assert!(issues.is_empty());

    Ok(())
}

#[tokio::test]
async fn continues_after_an_empty_intermediate_page() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(
                0,
                2,
                5,
                vec![
                    minimal_jira_issue("10001", "TEST-1"),
                    minimal_jira_issue("10002", "TEST-2"),
                ],
            )),
        )
        .await;
    context
        .expect_search(
            2,
            2,
            ResponseTemplate::new(200).set_body_json(search_page(2, 2, 5, vec![])),
        )
        .await;
    context
        .expect_search(
            4,
            2,
            ResponseTemplate::new(200).set_body_json(search_page(
                4,
                2,
                5,
                vec![minimal_jira_issue("10005", "TEST-5")],
            )),
        )
        .await;

    // WHEN
    let issues = context.client.fetch_issues(&context.jql).await?;

    // THEN
    let issue_keys = issues
        .iter()
        .map(|issue| issue.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(issue_keys, ["TEST-1", "TEST-2", "TEST-5"]);

    Ok(())
}

#[tokio::test]
async fn follows_an_increasing_total() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(
                0,
                2,
                3,
                vec![
                    minimal_jira_issue("10001", "TEST-1"),
                    minimal_jira_issue("10002", "TEST-2"),
                ],
            )),
        )
        .await;
    context
        .expect_search(
            2,
            2,
            ResponseTemplate::new(200).set_body_json(search_page(
                2,
                2,
                5,
                vec![
                    minimal_jira_issue("10003", "TEST-3"),
                    minimal_jira_issue("10004", "TEST-4"),
                ],
            )),
        )
        .await;
    context
        .expect_search(
            4,
            2,
            ResponseTemplate::new(200).set_body_json(search_page(
                4,
                2,
                5,
                vec![minimal_jira_issue("10005", "TEST-5")],
            )),
        )
        .await;

    // WHEN
    let issues = context.client.fetch_issues(&context.jql).await?;

    // THEN
    let issue_keys = issues
        .iter()
        .map(|issue| issue.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        issue_keys,
        ["TEST-1", "TEST-2", "TEST-3", "TEST-4", "TEST-5"]
    );

    Ok(())
}

#[tokio::test]
async fn stops_after_a_decreasing_total() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(
                0,
                2,
                5,
                vec![
                    minimal_jira_issue("10001", "TEST-1"),
                    minimal_jira_issue("10002", "TEST-2"),
                ],
            )),
        )
        .await;
    context
        .expect_search(
            2,
            2,
            ResponseTemplate::new(200).set_body_json(search_page(
                2,
                2,
                3,
                vec![minimal_jira_issue("10003", "TEST-3")],
            )),
        )
        .await;

    // WHEN
    let issues = context.client.fetch_issues(&context.jql).await?;

    // THEN
    let issue_keys = issues
        .iter()
        .map(|issue| issue.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(issue_keys, ["TEST-1", "TEST-2", "TEST-3"]);

    Ok(())
}

//------------//
//  FAILURES  //
//------------//

#[tokio::test]
async fn non_200_current_user_responses_return_their_status() -> anyhow::Result<()> {
    for expected_status in [
        StatusCode::NO_CONTENT,
        StatusCode::UNAUTHORIZED,
        StatusCode::FORBIDDEN,
        StatusCode::INTERNAL_SERVER_ERROR,
    ] {
        // GIVEN
        let context = TestContext::new().await?;
        context
            .expect_current_user(
                ResponseTemplate::new(expected_status.as_u16()).set_body_string("not inspected"),
            )
            .await;

        // WHEN
        let result = context.client.get_current_user().await;

        // THEN
        let Err(JiraClientError::ResponseStatus { status }) = result else {
            bail!("expected a response status error for {expected_status}, got {result:?}");
        };
        assert_eq!(status, expected_status);
    }

    Ok(())
}

#[tokio::test]
async fn an_unexpected_current_user_response_returns_an_error() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_current_user(ResponseTemplate::new(200).set_body_json(json!({
            "name": "alice"
        })))
        .await;

    // WHEN
    let result = context.client.get_current_user().await;

    // THEN
    assert!(matches!(result, Err(JiraClientError::Decode(_))));

    Ok(())
}

#[tokio::test]
async fn an_http_failure_on_a_later_page_returns_an_error() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(
                0,
                2,
                3,
                vec![
                    minimal_jira_issue("10001", "TEST-1"),
                    minimal_jira_issue("10002", "TEST-2"),
                ],
            )),
        )
        .await;
    context
        .expect_search(2, 2, ResponseTemplate::new(500))
        .await;

    // WHEN
    let result = context.client.fetch_issues(&context.jql).await;

    // THEN
    assert!(matches!(
        result,
        Err(JiraClientError::ResponseStatus {
            status: StatusCode::INTERNAL_SERVER_ERROR
        })
    ));

    Ok(())
}

#[tokio::test]
async fn an_unexpected_response_shape_on_a_later_page_returns_an_error() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(
                0,
                2,
                3,
                vec![
                    minimal_jira_issue("10001", "TEST-1"),
                    minimal_jira_issue("10002", "TEST-2"),
                ],
            )),
        )
        .await;
    let mut issue_with_invalid_status = minimal_jira_issue("10003", "TEST-3");
    issue_with_invalid_status["fields"]["status"] = json!("Open");
    context
        .expect_search(
            2,
            2,
            ResponseTemplate::new(200).set_body_json(search_page(
                2,
                2,
                3,
                vec![issue_with_invalid_status],
            )),
        )
        .await;

    // WHEN
    let result = context.client.fetch_issues(&context.jql).await;

    // THEN
    assert!(matches!(result, Err(JiraClientError::Decode(_))));

    Ok(())
}

#[tokio::test]
async fn invalid_json_on_a_later_page_returns_an_error() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(
                0,
                2,
                3,
                vec![
                    minimal_jira_issue("10001", "TEST-1"),
                    minimal_jira_issue("10002", "TEST-2"),
                ],
            )),
        )
        .await;
    context
        .expect_search(
            2,
            2,
            ResponseTemplate::new(200).set_body_string("{not valid JSON"),
        )
        .await;

    // WHEN
    let result = context.client.fetch_issues(&context.jql).await;

    // THEN
    assert!(matches!(result, Err(JiraClientError::Decode(_))));

    Ok(())
}

#[tokio::test]
async fn an_unexpected_page_start_returns_an_error() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(1, 100, 1, vec![])),
        )
        .await;

    // WHEN
    let result = context.client.fetch_issues(&context.jql).await;

    // THEN
    assert!(matches!(
        result,
        Err(JiraClientError::UnexpectedPageStart {
            requested: 0,
            received: 1
        })
    ));

    Ok(())
}

#[tokio::test]
async fn a_zero_page_size_with_results_remaining_returns_an_error() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .expect_search(
            0,
            100,
            ResponseTemplate::new(200).set_body_json(search_page(0, 0, 1, vec![])),
        )
        .await;

    // WHEN
    let result = context.client.fetch_issues(&context.jql).await;

    // THEN
    assert!(matches!(
        result,
        Err(JiraClientError::ZeroPageSize {
            start_at: 0,
            total: 1
        })
    ));

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

fn search_page(start_at: usize, max_results: usize, total: usize, issues: Vec<Value>) -> Value {
    json!({
        "startAt": start_at,
        "maxResults": max_results,
        "total": total,
        "issues": issues
    })
}

fn minimal_jira_issue(id: &str, key: &str) -> Value {
    json!({
        "id": id,
        "key": key,
        "fields": {
            "summary": format!("Issue {key}"),
            "status": { "name": "Open" },
            "issuetype": { "name": "Task" },
            "assignee": null,
            "description": null,
            "updated": "2026-09-15T10:00:00.000+0000",
            "parent": null,
            "subtasks": []
        }
    })
}

fn fully_populated_issue() -> Value {
    json!({
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
    })
}
