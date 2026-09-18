use super::dto::{CurrentUser, SearchRequest, SearchResponse};
use crate::config::{JiraJql, JiraToken, JiraUrl};
use crate::domain::Issue;
use reqwest::StatusCode;
use std::time::Duration;

const CURRENT_USER_PATH: &str = "/rest/api/2/myself";
const SEARCH_PATH: &str = "/rest/api/2/search";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const INITIAL_PAGE_SIZE: usize = 100;
const ISSUE_FIELDS: &[&str] = &[
    "summary",
    "status",
    "issuetype",
    "assignee",
    "description",
    "updated",
    "parent",
    "subtasks",
];

pub struct JiraClient {
    http_client: reqwest::Client,
    url: JiraUrl,
    token: JiraToken,
}

#[derive(Debug, Eq, PartialEq)]
pub struct JiraUser {
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum JiraClientError {
    #[error("failed to build Jira HTTP client")]
    BuildClient(#[source] reqwest::Error),

    #[error("failed to send request to Jira")]
    Request(#[source] reqwest::Error),

    #[error("Jira request failed with HTTP status: {status}")]
    ResponseStatus { status: StatusCode },

    #[error("failed to decode response from Jira")]
    Decode(#[source] reqwest::Error),

    #[error("Jira returned page start {received} when {requested} was requested")]
    UnexpectedPageStart { requested: usize, received: usize },

    #[error("Jira returned a zero page size at offset {start_at} with {total} results remaining")]
    ZeroPageSize { start_at: usize, total: usize },

    #[error("Jira page offset overflowed at offset {start_at} with page size {page_size}")]
    PageOffsetOverflow { start_at: usize, page_size: usize },
}

impl JiraClient {
    pub fn new(url: &JiraUrl, token: &JiraToken) -> Result<Self, JiraClientError> {
        let http_client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(JiraClientError::BuildClient)?;

        Ok(Self {
            http_client,
            url: url.clone(),
            token: token.clone(),
        })
    }

    pub async fn get_current_user(&self) -> Result<JiraUser, JiraClientError> {
        let response = self
            .http_client
            .get(format!("{}{CURRENT_USER_PATH}", self.url.as_str()))
            .bearer_auth(self.token.as_str())
            .send()
            .await
            .map_err(JiraClientError::Request)?;
        let status = response.status();

        if status != StatusCode::OK {
            return Err(JiraClientError::ResponseStatus { status });
        }

        let user = response
            .json::<CurrentUser>()
            .await
            .map_err(JiraClientError::Decode)?;

        Ok(JiraUser {
            username: user.name,
            display_name: user.display_name,
        })
    }

    pub async fn fetch_issues(&self, jql: &JiraJql) -> Result<Vec<Issue>, JiraClientError> {
        let mut issues = Vec::new();
        let mut start_at = 0;
        let mut page_size = INITIAL_PAGE_SIZE;

        loop {
            let response = self.fetch_page(jql, start_at, page_size).await?;
            let next_start_at = next_page_start(&response, start_at)?;

            page_size = response.max_results;
            issues.extend(response.issues.into_iter().map(Issue::from));

            match next_start_at {
                Some(next_start_at) => start_at = next_start_at,
                None => return Ok(issues),
            }
        }
    }

    async fn fetch_page(
        &self,
        jql: &JiraJql,
        start_at: usize,
        max_results: usize,
    ) -> Result<SearchResponse, JiraClientError> {
        let request = SearchRequest {
            jql: jql.as_str(),
            start_at,
            max_results,
            fields: ISSUE_FIELDS,
        };
        let response = self
            .http_client
            .post(format!("{}{SEARCH_PATH}", self.url.as_str()))
            .bearer_auth(self.token.as_str())
            .json(&request)
            .send()
            .await
            .map_err(JiraClientError::Request)?;
        let status = response.status();

        if !status.is_success() {
            return Err(JiraClientError::ResponseStatus { status });
        }

        response.json().await.map_err(JiraClientError::Decode)
    }
}

fn next_page_start(
    response: &SearchResponse,
    requested_start_at: usize,
) -> Result<Option<usize>, JiraClientError> {
    if response.start_at != requested_start_at {
        return Err(JiraClientError::UnexpectedPageStart {
            requested: requested_start_at,
            received: response.start_at,
        });
    }

    if response.start_at >= response.total {
        return Ok(None);
    }

    if response.max_results == 0 {
        return Err(JiraClientError::ZeroPageSize {
            start_at: response.start_at,
            total: response.total,
        });
    }

    let next_start_at = response.start_at.checked_add(response.max_results).ok_or(
        JiraClientError::PageOffsetOverflow {
            start_at: response.start_at,
            page_size: response.max_results,
        },
    )?;

    Ok((next_start_at < response.total).then_some(next_start_at))
}
