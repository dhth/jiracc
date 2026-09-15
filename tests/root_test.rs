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
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    jiracc lets you access your JIRA issues offline

    Usage: jiracc <COMMAND>

    Commands:
      config  Work with jiracc's configuration
      help    Print this message or the help of the given subcommand(s)

    Options:
      -h, --help  Print help

    ----- stderr -----
    ");
}
