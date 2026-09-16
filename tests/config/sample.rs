use crate::common::Fixture;
use insta_cmd::assert_cmd_snapshot;

#[test]
fn shows_help() {
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
fn prints_config() {
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
