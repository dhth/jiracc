mod common;

use chrono::{DateTime, TimeDelta, Utc};
use common::Fixture;
use insta_cmd::assert_cmd_snapshot;
use jiracc::config;
use jiracc::domain::{Issue, Snapshot, SnapshotMetadata};
use jiracc::persistence::FileSnapshotStore;
use std::path::PathBuf;

const TOKEN_ENVIRONMENT_VARIABLE: &str = "JIRACC_STATUS_TEST_TOKEN";

struct TestContext {
    fixture: Fixture,
    _temp_dir: tempfile::TempDir,
    config_path: PathBuf,
    data_home: PathBuf,
}

impl TestContext {
    fn new() -> anyhow::Result<Self> {
        let fixture = Fixture::new();
        let temp_dir = tempfile::tempdir()?;
        let config_path = temp_dir.path().join("jiracc.toml");
        let data_home = temp_dir.path().join("data");
        std::fs::write(&config_path, config_contents())?;

        Ok(Self {
            fixture,
            _temp_dir: temp_dir,
            config_path,
            data_home,
        })
    }

    fn status_command(&self) -> std::process::Command {
        let mut command = self.fixture.cmd(["status", "-p"]);
        command
            .arg(&self.config_path)
            .env("XDG_DATA_HOME", &self.data_home)
            .env_remove(TOKEN_ENVIRONMENT_VARIABLE);
        command
    }

    fn save_snapshot(&self, issue_count: usize, fetched_at: DateTime<Utc>) -> anyhow::Result<()> {
        let config_path = config::resolve_path(&self.config_path)?;
        let store = FileSnapshotStore::new(&self.data_home.join("jiracc"), &config_path);
        store.save_snapshot(&snapshot(issue_count, fetched_at))?;

        Ok(())
    }
}

#[test]
fn shows_help() {
    // GIVEN
    let fixture = Fixture::new();
    let mut cmd = fixture.cmd(["status", "--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    Show information about the local issue cache

    Usage: jiracc status [OPTIONS]

    Options:
      -p, --config-path <PATH>  Path to the configuration file
      -h, --help                Print help

    ----- stderr -----
    ");
}

#[cfg(unix)]
#[test]
fn reports_a_populated_snapshot() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new()?;
    let fetched_at =
        Utc::now() - TimeDelta::days(4) - TimeDelta::hours(14) - TimeDelta::minutes(30);
    context.save_snapshot(5, fetched_at)?;
    let mut cmd = context.status_command();

    // WHEN
    // THEN
    insta::with_settings!({
        filters => vec![(
            r"Fetched: \d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z",
            "Fetched: [timestamp]"
        )]
    }, {
        assert_cmd_snapshot!(cmd, @"
        success: true
        exit_code: 0
        ----- stdout -----
        5 issues cached
        Fetched: [timestamp] (4d 14h ago)

        ----- stderr -----
        ");
    });

    Ok(())
}

#[cfg(unix)]
#[test]
fn reports_an_empty_snapshot() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new()?;
    let fetched_at = Utc::now() - TimeDelta::minutes(24) - TimeDelta::seconds(30);
    context.save_snapshot(0, fetched_at)?;
    let mut cmd = context.status_command();

    // WHEN
    // THEN
    insta::with_settings!({
        filters => vec![(
            r"Fetched: \d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z",
            "Fetched: [timestamp]"
        )]
    }, {
        assert_cmd_snapshot!(cmd, @"
        success: true
        exit_code: 0
        ----- stdout -----
        0 issues cached
        Fetched: [timestamp] (24m ago)

        ----- stderr -----
        ");
    });

    Ok(())
}

#[cfg(unix)]
#[test]
fn reports_an_absent_snapshot_successfully() -> anyhow::Result<()> {
    // GIVEN
    let context = TestContext::new()?;
    let mut cmd = context.status_command();

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    No local issue cache found

    ----- stderr -----
    ");

    Ok(())
}

fn snapshot(issue_count: usize, fetched_at: DateTime<Utc>) -> Snapshot {
    Snapshot {
        metadata: SnapshotMetadata {
            fetched_at,
            jira_url: "https://jira.example.com".to_owned(),
            jql: "project = TEST".to_owned(),
        },
        issues: (0..issue_count).map(issue).collect(),
    }
}

fn issue(index: usize) -> Issue {
    Issue {
        id: index.to_string(),
        key: format!("TEST-{index}"),
        summary: "Summary".to_owned(),
        status: "Open".to_owned(),
        issue_type: "Task".to_owned(),
        assignee: None,
        description: None,
        updated_at: "2026-09-18T10:00:00.000+0000".to_owned(),
        parent: None,
        subtasks: Vec::new(),
    }
}

fn config_contents() -> String {
    format!(
        r#"
[jira]
url = "https://jira.example.com"
token = "${{{TOKEN_ENVIRONMENT_VARIABLE}}}"
jql = "project = TEST"
"#
    )
}
