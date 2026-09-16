use crate::common::Fixture;
use insta_cmd::assert_cmd_snapshot;

#[test]
fn shows_help() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["config", "init", "--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Create a sample configuration at the default path

    Usage: jiracc config init

    Options:
      -h, --help  Print help

    ----- stderr -----
    ");
}

#[cfg(unix)]
#[test]
fn creates_sample_config_at_default_path() -> anyhow::Result<()> {
    // GIVEN
    let fx = Fixture::new();
    let temp_dir = tempfile::tempdir()?;
    let config_root = temp_dir.path().join("xdg");
    let config_path = config_root.join("jiracc").join("jiracc.toml");
    let mut cmd = fx.cmd(["config", "init"]);
    cmd.env("XDG_CONFIG_HOME", &config_root);
    let sample_config = {
        let output = fx.cmd(["config", "sample"]).output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        String::from_utf8(output.stdout)?
    };

    // WHEN
    let output = cmd.output()?;

    // THEN
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)?,
        format!(
            "Created sample configuration at {}.

Edit it to match your Jira setup.\n",
            config_path.display()
        )
    );
    assert!(output.stderr.is_empty());
    assert_eq!(std::fs::read_to_string(config_path)?, sample_config);

    Ok(())
}

#[cfg(unix)]
#[test]
fn refuses_to_overwrite_existing_configuration() -> anyhow::Result<()> {
    // GIVEN
    let fx = Fixture::new();
    let temp_dir = tempfile::tempdir()?;
    let config_root = temp_dir.path().join("xdg");
    let config_dir = config_root.join("jiracc");
    let config_path = config_dir.join("jiracc.toml");
    let original_contents = b"existing configuration\n";
    std::fs::create_dir_all(config_dir)?;
    std::fs::write(&config_path, original_contents)?;
    let mut cmd = fx.cmd(["config", "init"]);
    cmd.env("XDG_CONFIG_HOME", &config_root);

    // WHEN
    let output = cmd.output()?;

    // THEN
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr)?,
        format!("Error: configuration already exists at {config_path:?}\n")
    );
    assert_eq!(std::fs::read(config_path)?, original_contents);

    Ok(())
}
