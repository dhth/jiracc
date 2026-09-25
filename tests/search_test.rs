mod common;

use anyhow::ensure;
use common::Fixture;
use insta_cmd::assert_cmd_snapshot;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";
const TOKEN_ENVIRONMENT_VARIABLE: &str = "JIRACC_SEARCH_TEST_TOKEN";
const JQL: &str = "project = TEST
ORDER BY updated DESC
";
const SEARCH_RESPONSE_FIVE_ISSUES: &str =
    include_str!("testdata/jira/search-response-5-issues.json");
const SEARCH_RESPONSE_EMPTY: &str = include_str!("testdata/jira/search-response-empty.json");

struct TestContext {
    fixture: Fixture,
    server: MockServer,
    _temp_dir: tempfile::TempDir,
    config_path: PathBuf,
    data_home: PathBuf,
}

impl TestContext {
    async fn new() -> anyhow::Result<Self> {
        let fixture = Fixture::new();
        let server = MockServer::start().await;
        let temp_dir = tempfile::tempdir()?;
        let config_path = temp_dir.path().join("jiracc.toml");
        let data_home = temp_dir.path().join("data");
        std::fs::write(&config_path, config_contents(&server.uri()))?;

        Ok(Self {
            fixture,
            server,
            _temp_dir: temp_dir,
            config_path,
            data_home,
        })
    }

    fn sync_command(&self) -> std::process::Command {
        let mut command = self.fixture.cmd(["sync", "--config-path"]);
        command
            .arg(&self.config_path)
            .env("XDG_DATA_HOME", &self.data_home)
            .env(TOKEN_ENVIRONMENT_VARIABLE, TOKEN);
        command
    }

    fn search_command(&self, query: Option<&str>) -> std::process::Command {
        let mut command = self.fixture.cmd(["search"]);
        if let Some(query) = query {
            command.arg(query);
        }
        command
            .arg("--config-path")
            .arg(&self.config_path)
            .env("XDG_DATA_HOME", &self.data_home)
            .env_remove(TOKEN_ENVIRONMENT_VARIABLE);
        command
    }

    async fn seed_snapshot(&self, response: &'static str) -> anyhow::Result<()> {
        Mock::given(method("POST"))
            .and(path("/rest/api/2/search"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(response, "application/json"))
            .expect(1)
            .mount(&self.server)
            .await;

        let mut command = self.sync_command();
        let output = tokio::task::spawn_blocking(move || command.output()).await??;
        ensure!(
            output.status.success(),
            "couldn't seed snapshot: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        self.server.verify().await;

        Ok(())
    }
}

//-------------//
//  SUCCESSES  //
//-------------//

#[test]
fn shows_help() {
    // GIVEN
    let fixture = Fixture::new();
    let mut cmd = fixture.cmd(["search", "--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    Search cached issues

    With no query or filters, displays all cached issues.

    Usage: jiracc search [OPTIONS] [QUERY]

    Arguments:
      [QUERY]
              Text to find in issue keys, summaries, or descriptions

    Options:
      -a, --assignee <USERNAME>
              Match a Jira username; may be repeated

      -s, --status <STATUS>
              Match a Jira status; may be repeated

      -t, --type <TYPE>
              Match a Jira issue type; may be repeated

      -p, --config-path <PATH>
              Path to the configuration file

      -h, --help
              Print help (see a summary with '-h')

    ----- stderr -----
    ");
}

#[cfg(unix)]
#[tokio::test]
async fn finds_and_prints_matched_issues() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot(SEARCH_RESPONSE_FIVE_ISSUES).await?;
    let mut cmd = context.search_command(Some("connection"));
    cmd.args(["--assignee", "alice", "--status", "Open", "--type", "Bug"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    KEY     TYPE  STATUS  ASSIGNEE  SUMMARY
    TEST-1  Bug   Open    @alice    Investigate connection timeout
    TEST-3  Bug   Open    @alice    Retry synchronization failures

    ----- stderr -----
    ");

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn accepts_repeated_filter_values() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot(SEARCH_RESPONSE_FIVE_ISSUES).await?;
    let mut cmd = context.search_command(Some("VPN"));
    cmd.args([
        "--assignee",
        "alice",
        "--assignee",
        "bob",
        "--status",
        "Open",
        "--status",
        "In Progress",
        "--type",
        "Bug",
        "--type",
        "Story",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    KEY     TYPE   STATUS       ASSIGNEE  SUMMARY
    TEST-1  Bug    Open         @alice    Investigate connection timeout
    TEST-6  Story  In Progress  @bob      Improve VPN error reporting

    ----- stderr -----
    ");

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn prints_nothing_when_no_issues_match() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot(SEARCH_RESPONSE_FIVE_ISSUES).await?;
    let mut cmd = context.search_command(Some("database migration"));

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn lists_all_cached_issues_without_criteria() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot(SEARCH_RESPONSE_FIVE_ISSUES).await?;
    let mut cmd = context.search_command(None);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    KEY     TYPE   STATUS       ASSIGNEE  SUMMARY
    TEST-1  Bug    Open         @alice    Investigate connection timeout
    TEST-3  Bug    Open         @alice    Retry synchronization failures
    TEST-4  Bug    Open         @charlie  Diagnose connection pool exhaustion
    TEST-5  Bug    Done         @alice    Document connection recovery
    TEST-6  Story  In Progress  @bob      Improve VPN error reporting

    ----- stderr -----
    ");

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn exits_quietly_when_stdout_pipe_has_no_reader() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot(SEARCH_RESPONSE_FIVE_ISSUES).await?;
    let (reader, writer) = std::io::pipe()?;
    drop(reader);
    let mut cmd = context.search_command(None);
    cmd.stdout(writer);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn prints_nothing_when_the_snapshot_is_empty() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot(SEARCH_RESPONSE_EMPTY).await?;
    let mut cmd = context.search_command(None);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    Ok(())
}

//------------//
//  FAILURES  //
//------------//

#[test]
fn rejects_an_explicitly_empty_query() {
    // GIVEN
    let fixture = Fixture::new();
    let mut cmd = fixture.cmd(["search", ""]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: 'query' must not be empty
    ");
}

#[test]
fn rejects_a_whitespace_only_query() {
    // GIVEN
    let fixture = Fixture::new();
    let mut cmd = fixture.cmd(["search", "   "]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: 'query' must not be empty
    ");
}

#[test]
fn rejects_an_empty_issue_type() {
    // GIVEN
    let fixture = Fixture::new();
    let mut cmd = fixture.cmd(["search", "--type", ""]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: 'issue type' must not be empty
    ");
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn reports_stdout_write_error_when_device_is_full() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot(SEARCH_RESPONSE_FIVE_ISSUES).await?;
    let full = std::fs::OpenOptions::new().write(true).open("/dev/full")?;
    let mut cmd = context.search_command(None);
    cmd.stdout(full);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: couldn't write search results to stdout

    Caused by:
        No space left on device (os error 28)
    ");

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn reports_a_missing_snapshot() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    let mut cmd = context.search_command(None);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: no cached snapshot exists for this configuration
    ");

    Ok(())
}

fn config_contents(jira_url: &str) -> String {
    format!(
        r#"
[jira]
url = {jira_url:?}
token = "${{{TOKEN_ENVIRONMENT_VARIABLE}}}"
jql = """
{JQL}"""
"#
    )
}
