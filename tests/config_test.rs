mod common;

use common::Fixture;
use insta_cmd::assert_cmd_snapshot;
use std::path::PathBuf;

#[test]
fn sample_shows_help() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["config", "sample", "--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Print a sample configuration to stdout

    Usage: jiracc config sample

    Options:
      -h, --help  Print help

    ----- stderr -----
    ");
}

#[test]
fn sample_prints_config() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["config", "sample"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r##"
    success: true
    exit_code: 0
    ----- stdout -----
    # Configuration values can reference environment variables. Referenced
    # variables need to be set before running jiracc.

    [jira]
    # Base URL of your on-premise Jira Server or Data Center installation.
    url = "https://jira.example.com"

    # Personal access token used to authenticate with Jira.
    token = "$JIRACC_JIRA_TOKEN"

    # JQL that defines the issues jiracc synchronizes.
    jql = """
    project = EXAMPLE
    ORDER BY updated DESC
    """

    ----- stderr -----
    "##);
}

#[test]
fn validate_shows_help() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["config", "validate", "--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Validate a configuration file

    Usage: jiracc config validate [OPTIONS]

    Options:
          --config-path <PATH>  Path to the configuration file
      -h, --help                Print help

    ----- stderr -----
    ");
}

#[test]
fn validate_accepts_valid_config_at_explicit_path() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["config", "validate"]);
    cmd.arg("--config-path")
        .arg(testdata_path(&["config", "valid.toml"]));
    cmd.env("JIRACC_TEST_TOKEN", "secret-token");

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Configuration is valid.

    ----- stderr -----
    ");
}

#[cfg(unix)]
#[test]
fn validate_accepts_valid_config_at_default_path() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["config", "validate"]);
    cmd.env("XDG_CONFIG_HOME", testdata_path(&["xdg"]));
    cmd.env("JIRACC_TEST_TOKEN", "secret-token");

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Configuration is valid.

    ----- stderr -----
    ");
}

#[cfg(unix)]
#[test]
fn validate_reports_a_missing_config_file() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["config", "validate", "--config-path", "missing.toml"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: configuration is invalid

    Caused by:
        0: failed to read configuration from missing.toml
        1: No such file or directory (os error 2)
    ");
}

#[test]
fn validate_reports_invalid_toml() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["config", "validate"]);
    cmd.arg("--config-path")
        .arg(testdata_path(&["config", "invalid-toml.toml"]));

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r#"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: configuration is invalid

    Caused by:
        0: failed to parse configuration
        1: TOML parse error at line 4, column 22
             |
           4 | jql = "project = TEST
             |                      ^
           invalid basic string, expected `"`
    "#);
}

#[test]
fn validate_reports_semantically_invalid_config() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["config", "validate"]);
    cmd.arg("--config-path")
        .arg(testdata_path(&["config", "invalid-url.toml"]));

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: configuration is invalid

    Caused by:
        jira.url must use http or https, not ftp
    ");
}

// Accept separate components so paths use the target platform's native separators.
fn testdata_path(components: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata");
    path.extend(components);
    path
}
