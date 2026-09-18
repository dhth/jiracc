mod common;

use common::Fixture;
use insta_cmd::assert_cmd_snapshot;

#[test]
fn shows_help() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    jiracc lets you access your JIRA issues offline

    Usage: jiracc <COMMAND>

    Commands:
      auth    Work with Jira authentication
      config  Work with jiracc's configuration
      sync    Synchronize issues from Jira
      status  Show information about the local issue cache
      show    Show the cached details of an issue
      search  Search cached issues
      help    Print this message or the help of the given subcommand(s)

    Options:
      -h, --help  Print help

    ----- stderr -----
    ");
}
