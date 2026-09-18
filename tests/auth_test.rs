mod common;

use common::Fixture;
use insta_cmd::assert_cmd_snapshot;
use serde_json::json;
use std::path::PathBuf;
use wiremock::matchers::{bearer_token, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";

struct TestContext {
    fixture: Fixture,
    server: MockServer,
    _temp_dir: tempfile::TempDir,
    config_path: PathBuf,
}

impl TestContext {
    async fn new() -> anyhow::Result<Self> {
        let fixture = Fixture::new();
        let server = MockServer::start().await;
        let temp_dir = tempfile::tempdir()?;
        let config_path = temp_dir.path().join("jiracc.toml");
        std::fs::write(&config_path, config_contents(&server.uri()))?;

        Ok(Self {
            fixture,
            server,
            _temp_dir: temp_dir,
            config_path,
        })
    }

    fn check_command(&self) -> std::process::Command {
        let mut command = self.fixture.cmd(["auth", "check", "--config-path"]);
        command.arg(&self.config_path);
        command
    }

    async fn mount_current_user(&self, response: ResponseTemplate) {
        Mock::given(method("GET"))
            .and(path("/rest/api/2/myself"))
            .and(bearer_token(TOKEN))
            .respond_with(response)
            .expect(1)
            .mount(&self.server)
            .await;
    }
}

#[test]
fn shows_auth_help() {
    // GIVEN
    let fixture = Fixture::new();
    let mut cmd = fixture.cmd(["auth", "--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    Work with Jira authentication

    Usage: jiracc auth <COMMAND>

    Commands:
      check  Check Jira authentication
      help   Print this message or the help of the given subcommand(s)

    Options:
      -h, --help  Print help

    ----- stderr -----
    ");
}

#[test]
fn shows_check_help() {
    // GIVEN
    let fixture = Fixture::new();
    let mut cmd = fixture.cmd(["auth", "check", "--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    Check Jira authentication

    Usage: jiracc auth check [OPTIONS]

    Options:
      -p, --config-path <PATH>  Path to the configuration file
      -h, --help                Print help

    ----- stderr -----
    ");
}

#[tokio::test]
async fn reports_the_authenticated_user() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .mount_current_user(ResponseTemplate::new(200).set_body_json(json!({
            "name": "alice",
            "displayName": "Alice Example",
            "active": true,
            "emailAddress": "alice@example.com"
        })))
        .await;
    let mut cmd = context.check_command();

    // WHEN
    // THEN
    tokio::task::spawn_blocking(move || {
        insta::with_settings!({
            filters => vec![(r"http://127\.0\.0\.1:\d+", "[mock-server-url]")]
        }, {
            assert_cmd_snapshot!(cmd, @"
            success: true
            exit_code: 0
            ----- stdout -----
            Authenticated to [mock-server-url] as Alice Example (alice).

            ----- stderr -----
            ");
        });
    })
    .await?;
    context.server.verify().await;

    Ok(())
}

#[tokio::test]
async fn reports_a_jira_http_failure() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .mount_current_user(
            ResponseTemplate::new(401)
                .set_body_string("sensitive diagnostic that must not be shown"),
        )
        .await;
    let mut cmd = context.check_command();

    // WHEN
    // THEN
    tokio::task::spawn_blocking(move || {
        assert_cmd_snapshot!(cmd, @"
        success: false
        exit_code: 1
        ----- stdout -----

        ----- stderr -----
        Error: couldn't check Jira authentication

        Caused by:
            Jira request failed with HTTP status: 401 Unauthorized
        ");
    })
    .await?;
    context.server.verify().await;

    Ok(())
}

#[tokio::test]
async fn reports_an_unexpected_jira_response() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .mount_current_user(ResponseTemplate::new(200).set_body_json(json!({
            "name": "alice"
        })))
        .await;
    let mut cmd = context.check_command();

    // WHEN
    // THEN
    tokio::task::spawn_blocking(move || {
        assert_cmd_snapshot!(cmd, @"
        success: false
        exit_code: 1
        ----- stdout -----

        ----- stderr -----
        Error: couldn't check Jira authentication

        Caused by:
            0: failed to decode response from Jira
            1: error decoding response body
            2: missing field `displayName` at line 1 column 16
        ");
    })
    .await?;
    context.server.verify().await;

    Ok(())
}

fn config_contents(jira_url: &str) -> String {
    format!(
        r#"
[jira]
url = {jira_url:?}
token = {TOKEN:?}
jql = "project = TEST"
"#
    )
}
