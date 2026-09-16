mod common;

use anyhow::{Context, Result, bail, ensure};
use common::Fixture;
use insta_cmd::assert_cmd_snapshot;
use serde_json::json;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use wiremock::matchers::{bearer_token, body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";
const JQL: &str = "project = TEST
ORDER BY updated DESC
";
const TWO_ISSUE_SEARCH_RESPONSE: &str = include_str!("testdata/jira/search-response-2-issues.json");
const EMPTY_SEARCH_RESPONSE: &str = include_str!("testdata/jira/search-response-empty.json");

struct TestContext {
    fixture: Fixture,
    server: MockServer,
    _temp_dir: tempfile::TempDir,
    config_path: PathBuf,
    data_home: PathBuf,
}

impl TestContext {
    async fn new() -> Result<Self> {
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
            .env("XDG_DATA_HOME", &self.data_home);
        command
    }

    async fn mount_search(&self, response: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path("/rest/api/2/search"))
            .and(bearer_token(TOKEN))
            .and(body_partial_json(json!({ "jql": JQL })))
            .respond_with(response)
            .expect(1)
            .mount(&self.server)
            .await;
    }

    async fn seed_snapshot(&self) -> Result<()> {
        self.mount_search(search_response(TWO_ISSUE_SEARCH_RESPONSE))
            .await;

        let mut command = self.sync_command();
        let output = tokio::task::spawn_blocking(move || command.output()).await??;
        ensure!(
            output.status.success(),
            "couldn't seed snapshot: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        self.server.verify().await;
        self.server.reset().await;

        Ok(())
    }
}

#[test]
fn shows_help() {
    // GIVEN
    let fixture = Fixture::new();
    let mut cmd = fixture.cmd(["sync", "--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
    Synchronize issues from Jira

    Usage: jiracc sync [OPTIONS]

    Options:
          --config-path <PATH>  Path to the configuration file
      -h, --help                Print help

    ----- stderr -----
    ");
}

#[cfg(unix)]
#[tokio::test]
async fn synchronizes_issues_to_a_snapshot() -> Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context
        .mount_search(search_response(TWO_ISSUE_SEARCH_RESPONSE))
        .await;
    let mut cmd = context.sync_command();

    // WHEN
    // THEN
    tokio::task::spawn_blocking(move || {
        assert_cmd_snapshot!(cmd, @"
        success: true
        exit_code: 0
        ----- stdout -----
        Synchronized 2 issues.

        ----- stderr -----
        ");
    })
    .await?;

    assert_data_directory_snapshot("successful_sync_data_directory", &context.data_home)?;

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn an_empty_result_replaces_the_existing_snapshot() -> Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot().await?;
    context
        .mount_search(search_response(EMPTY_SEARCH_RESPONSE))
        .await;
    let mut cmd = context.sync_command();

    // WHEN
    // THEN
    tokio::task::spawn_blocking(move || {
        assert_cmd_snapshot!(cmd, @"
        success: true
        exit_code: 0
        ----- stdout -----
        No issues found.

        ----- stderr -----
        ");
    })
    .await?;

    assert_data_directory_snapshot("empty_sync_data_directory", &context.data_home)?;

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn a_jira_failure_preserves_the_existing_snapshot() -> Result<()> {
    // GIVEN
    let context = TestContext::new().await?;
    context.seed_snapshot().await?;
    let data_directory_before = render_directory_tree(&context.data_home)?;
    context.mount_search(ResponseTemplate::new(500)).await;
    let mut cmd = context.sync_command();

    // WHEN
    // THEN
    tokio::task::spawn_blocking(move || {
        assert_cmd_snapshot!(cmd, @"
        success: false
        exit_code: 1
        ----- stdout -----

        ----- stderr -----
        Error: couldn't fetch issues

        Caused by:
            Jira search request failed with status 500 Internal Server Error
        ");
    })
    .await?;

    let data_directory_after = render_directory_tree(&context.data_home)?;
    assert_eq!(data_directory_after, data_directory_before);

    Ok(())
}

fn search_response(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body, "application/json")
}

fn config_contents(jira_url: &str) -> String {
    format!(
        r#"
[jira]
url = {jira_url:?}
token = {TOKEN:?}
jql = """
{JQL}"""
"#
    )
}

fn assert_data_directory_snapshot(name: &str, data_home: &Path) -> Result<()> {
    let tree = render_directory_tree(data_home)?;
    insta::with_settings!({
        filters => vec![
            (r"[0-9a-f]{64}", "[namespace]"),
            (
                r#"("fetched_at": ")\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z(")"#,
                "$1[timestamp]$2"
            ),
            (r#"http://127\.0\.0\.1:\d+"#, "[mock-server-url]"),
        ]
    }, {
        insta::assert_snapshot!(name, tree);
    });

    Ok(())
}

fn render_directory_tree(root: &Path) -> Result<String> {
    let mut tree = String::new();
    let mut files = Vec::new();
    render_directory(root, root, 0, &mut tree, &mut files)?;

    for (relative_path, contents) in files {
        writeln!(tree, "\n--- {relative_path} ---")?;
        tree.push_str(&contents);
        if !contents.ends_with('\n') {
            tree.push('\n');
        }
    }

    Ok(tree)
}

fn render_directory(
    root: &Path,
    directory: &Path,
    depth: usize,
    tree: &mut String,
    files: &mut Vec<(String, String)>,
) -> Result<()> {
    let mut entries = std::fs::read_dir(directory)
        .with_context(|| format!("couldn't read directory at {directory:?}"))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("couldn't determine file type at {path:?}"))?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .with_context(|| format!("path is not valid UTF-8: {path:?}"))?;

        if file_type.is_dir() {
            writeln!(tree, "{}{name}/", "  ".repeat(depth))?;
            render_directory(root, &path, depth + 1, tree, files)?;
        } else if file_type.is_file() {
            writeln!(tree, "{}{name}", "  ".repeat(depth))?;
            let relative_path = path
                .strip_prefix(root)
                .with_context(|| format!("path {path:?} is not beneath root {root:?}"))?;
            let relative_path = relative_path
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let contents = std::fs::read_to_string(&path)
                .with_context(|| format!("couldn't read file at {path:?}"))?;
            files.push((relative_path, contents));
        } else {
            bail!("unsupported filesystem entry at {path:?}");
        }
    }

    Ok(())
}
