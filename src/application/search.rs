use crate::application::{IssueFilter, IssueFilterError, SnapshotStore};
use crate::config;
use crate::domain::Issue;
use crate::paths;
use crate::persistence::{FileSnapshotStore, FileSnapshotStoreError};
use std::error::Error;
use std::io::Write;
use std::path::PathBuf;
use tabwriter::TabWriter;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error(transparent)]
    InvalidFilter(#[from] IssueFilterError),

    #[error(transparent)]
    Paths(#[from] paths::PathsError),

    #[error(transparent)]
    ConfigPath(#[from] config::ConfigError),

    #[error(transparent)]
    Operation(#[from] SearchOperationError<FileSnapshotStoreError>),
}

#[derive(Debug, thiserror::Error)]
pub enum SearchOperationError<StoreError>
where
    StoreError: Error + 'static,
{
    #[error(transparent)]
    LoadSnapshot(StoreError),

    #[error("couldn't write search results to stdout")]
    WriteOutput(#[source] std::io::Error),
}

pub fn search(
    query: Option<String>,
    assignees: Vec<String>,
    statuses: Vec<String>,
    issue_types: Vec<String>,
    config_path: Option<PathBuf>,
) -> Result<(), SearchError> {
    let filter = IssueFilter::new(query, assignees, statuses, issue_types)?;
    let paths = paths::get()?;
    let config_path = config_path.unwrap_or(paths.config);
    let config_path = config::resolve_path(&config_path)?;
    let store = FileSnapshotStore::new(&paths.data, &config_path);
    let mut stdout = std::io::stdout().lock();

    search_with(&store, &mut stdout, &filter)?;

    Ok(())
}

fn search_with<S, W>(
    store: &S,
    output: &mut W,
    filter: &IssueFilter,
) -> Result<(), SearchOperationError<S::Error>>
where
    S: SnapshotStore,
    W: Write,
{
    let snapshot = store
        .get_snapshot()
        .map_err(SearchOperationError::LoadSnapshot)?;

    let issues = {
        let mut issues = snapshot.issues;

        if !filter.is_unconstrained() {
            issues.retain(|issue| filter.matches(issue));
        }

        issues
    };

    if issues.is_empty() {
        return Ok(());
    }

    write_search_results(output, &issues).map_err(SearchOperationError::WriteOutput)?;

    Ok(())
}

fn write_search_results<'a, W, I>(output: W, issues: I) -> std::io::Result<()>
where
    W: Write,
    I: IntoIterator<Item = &'a Issue>,
{
    let mut table = TabWriter::new(output).padding(2);
    writeln!(table, "KEY\tTYPE\tSTATUS\tASSIGNEE\tSUMMARY")?;

    for issue in issues {
        let assignee = issue.assignee.as_ref().map_or_else(
            || "Unassigned".to_owned(),
            |assignee| format!("@{}", normalize(&assignee.username)),
        );
        writeln!(
            table,
            "{}\t{}\t{}\t{}\t{}",
            normalize(&issue.key),
            normalize(&issue.issue_type),
            normalize(&issue.status),
            assignee,
            normalize(&issue.summary),
        )?;
    }

    table.flush()
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Assignee;
    use insta::assert_snapshot;
    use std::io::ErrorKind;

    //-------------//
    //  SUCCESSES  //
    //-------------//

    #[test]
    fn writes_compact_results_in_input_order() -> anyhow::Result<()> {
        // GIVEN
        let issues = [
            issue(
                "TEST-2",
                "Technical Story",
                "In Progress",
                Some(("alice", "Alice Example")),
                "Investigate intermittent timeout",
            ),
            issue("TEST-1", "Bug", "Done", None, "Document recovery steps"),
        ];
        let mut output = Vec::new();

        // WHEN
        write_search_results(&mut output, &issues)?;

        // THEN
        assert_snapshot!(String::from_utf8(output)?, @"
        KEY     TYPE             STATUS       ASSIGNEE    SUMMARY
        TEST-2  Technical Story  In Progress  @alice      Investigate intermittent timeout
        TEST-1  Bug              Done         Unassigned  Document recovery steps
        ");
        Ok(())
    }

    #[test]
    fn normalizes_whitespace_before_writing_cells() -> anyhow::Result<()> {
        // GIVEN
        let issues = [
            issue(
                " TEST\t-\n3 ",
                " User\n\tStory ",
                " In\r\nProgress ",
                Some((" alice\t smith\n", "Ignored\nDisplay Name")),
                "  Preserve\nexactly\tone logical\r\nline  ",
            ),
            issue(
                "TEST-\n4",
                "Technical\tStory",
                "To\nDo",
                Some(("bob\tjones", "Ignored Display Name")),
                "Another\nissue\tsummary",
            ),
        ];
        let mut output = Vec::new();

        // WHEN
        write_search_results(&mut output, &issues)?;

        // THEN
        assert_snapshot!(String::from_utf8(output)?, @"
        KEY       TYPE             STATUS       ASSIGNEE      SUMMARY
        TEST - 3  User Story       In Progress  @alice smith  Preserve exactly one logical line
        TEST- 4   Technical Story  To Do        @bob jones    Another issue summary
        ");
        Ok(())
    }

    #[test]
    fn aligns_unicode_fields_without_padding_the_summary() -> anyhow::Result<()> {
        // GIVEN
        let issues = [
            issue("É-1", "改善", "Open", None, "短い"),
            issue("A-20", "Bug", "À faire", Some(("zoë", "Zoë")), "x"),
        ];
        let mut output = Vec::new();

        // WHEN
        write_search_results(&mut output, &issues)?;

        // THEN
        assert_snapshot!(String::from_utf8(output)?, @"
        KEY   TYPE  STATUS   ASSIGNEE    SUMMARY
        É-1   改善  Open     Unassigned  短い
        A-20  Bug   À faire  @zoë        x
        ");
        Ok(())
    }

    //------------//
    //  FAILURES  //
    //------------//

    #[test]
    fn reports_a_failure_while_rendering_the_buffered_table() {
        // GIVEN
        let issues = [issue("TEST-1", "Bug", "Open", None, "A summary")];

        // WHEN
        let error = write_search_results(FailingWriter, &issues)
            .expect_err("the underlying writer should fail");

        // THEN
        assert_eq!(error.kind(), ErrorKind::BrokenPipe);
    }

    fn issue(
        key: &str,
        issue_type: &str,
        status: &str,
        assignee: Option<(&str, &str)>,
        summary: &str,
    ) -> Issue {
        Issue {
            id: "10001".to_owned(),
            key: key.to_owned(),
            summary: summary.to_owned(),
            status: status.to_owned(),
            issue_type: issue_type.to_owned(),
            assignee: assignee.map(|(username, display_name)| Assignee {
                username: username.to_owned(),
                display_name: display_name.to_owned(),
            }),
            description: None,
            updated_at: "2026-09-18T10:00:00.000+0000".to_owned(),
            parent: None,
            subtasks: Vec::new(),
        }
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(ErrorKind::BrokenPipe, "write failed"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
