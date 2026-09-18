mod common;

use anyhow::{Context, ensure};
use common::Fixture;
use insta_cmd::assert_cmd_snapshot;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";
const TOKEN_ENVIRONMENT_VARIABLE: &str = "JIRACC_SHOW_TEST_TOKEN";
const JQL: &str = "project = TEST
ORDER BY updated DESC
";
const SEARCH_RESPONSE: &str = include_str!("testdata/jira/search-response-2-issues.json");

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

    fn show_command(&self, key: &str) -> std::process::Command {
        let mut command = self.fixture.cmd(["show", key, "--config-path"]);
        command
            .arg(&self.config_path)
            .env("XDG_DATA_HOME", &self.data_home)
            .env_remove(TOKEN_ENVIRONMENT_VARIABLE);
        command
    }

    async fn seed_snapshot(&self) -> anyhow::Result<()> {
        Mock::given(method("POST"))
            .and(path("/rest/api/2/search"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(SEARCH_RESPONSE, "application/json"),
            )
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

    fn snapshot_path(&self) -> anyhow::Result<PathBuf> {
        let namespace_directory = self.data_home.join("jiracc").join("snapshots").join("v1");
        let mut namespaces = std::fs::read_dir(&namespace_directory)?;
        let namespace = namespaces
            .next()
            .transpose()?
            .context("snapshot namespace is missing")?;
        ensure!(
            namespaces.next().is_none(),
            "expected exactly one snapshot namespace in {namespace_directory:?}"
        );
        let snapshot = namespace.path().join("snapshot.json");
        ensure!(
            snapshot.is_file(),
            "snapshot file is missing at {snapshot:?}"
        );

        Ok(snapshot)
    }
}

#[test]
fn shows_help() {
    // GIVEN
    let fixture = Fixture::new();
    let mut cmd = fixture.cmd(["show", "--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    Show the cached details of an issue

    Usage: jiracc show [OPTIONS] <ISSUE-KEY>

    Arguments:
      <ISSUE-KEY>  Jira issue key

    Options:
      -p, --config-path <PATH>  Path to the configuration file
      -h, --help                Print help

    ----- stderr -----
    ");
}

#[cfg(unix)]
#[tokio::test]
async fn shows_a_cached_issue_without_loading_the_config() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot().await?;
    let mut cmd = context.show_command("TEST-1");

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    TEST-1 — Example issue

    Type:       Task
    Status:     In Progress
    Assignee:   Alice Example (@alice)
    Updated:    2026-09-15T10:00:00.000+0000
    Jira ID:    10001
    Parent:     TEST-0
    Subtasks:   TEST-2

    Description
    -----------
    Example description

    ----- stderr -----
    ");

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn reports_an_unknown_issue() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot().await?;
    let mut cmd = context.show_command("TEST-99");

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: issue TEST-99 was not found in the cached snapshot
    ");

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn reports_a_missing_snapshot() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    let mut cmd = context.show_command("TEST-1");

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

#[cfg(unix)]
#[tokio::test]
async fn reports_a_malformed_snapshot() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot().await?;
    std::fs::write(context.snapshot_path()?, "not valid JSON")?;
    let mut cmd = context.show_command("TEST-1");

    // WHEN
    // THEN
    insta::with_settings!({
        filters => vec![(
            r#"at ".*/snapshots/v1/[0-9a-f]{64}/snapshot\.json""#,
            "at [snapshot-path]"
        )]
    }, {
        assert_cmd_snapshot!(cmd, @"
        success: false
        exit_code: 1
        ----- stdout -----

        ----- stderr -----
        Error: couldn't deserialize snapshot at [snapshot-path]

        Caused by:
            expected ident at line 1 column 2
        ");
    });

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
