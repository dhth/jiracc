use crate::domain::Issue;

#[derive(Debug)]
pub struct IssueFilter {
    query: Option<String>,
    assignee_usernames: Vec<String>,
    statuses: Vec<String>,
    issue_types: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum IssueFilterError {
    #[error("search requires a query or at least one filter")]
    NoCriteria,

    #[error("search query must not be empty")]
    EmptyQuery,
}

impl IssueFilter {
    pub fn new(
        query: Option<String>,
        assignee_usernames: Vec<String>,
        statuses: Vec<String>,
        issue_types: Vec<String>,
    ) -> Result<Self, IssueFilterError> {
        if query.is_none()
            && assignee_usernames.is_empty()
            && statuses.is_empty()
            && issue_types.is_empty()
        {
            return Err(IssueFilterError::NoCriteria);
        }

        let query = match query {
            Some(query) => {
                let query = query.trim();
                if query.is_empty() {
                    return Err(IssueFilterError::EmptyQuery);
                }
                Some(query.to_lowercase())
            }
            None => None,
        };

        Ok(Self {
            query,
            assignee_usernames: normalize(assignee_usernames),
            statuses: normalize(statuses),
            issue_types: normalize(issue_types),
        })
    }

    pub fn matches(&self, issue: &Issue) -> bool {
        self.matches_query(issue)
            && self.matches_assignee(issue)
            && matches_any(&self.statuses, &issue.status)
            && matches_any(&self.issue_types, &issue.issue_type)
    }

    fn matches_query(&self, issue: &Issue) -> bool {
        let Some(query) = &self.query else {
            return true;
        };

        contains(&issue.key, query)
            || contains(&issue.summary, query)
            || issue
                .description
                .as_deref()
                .is_some_and(|description| contains(description, query))
    }

    fn matches_assignee(&self, issue: &Issue) -> bool {
        if self.assignee_usernames.is_empty() {
            return true;
        }

        issue
            .assignee
            .as_ref()
            .is_some_and(|assignee| matches_any(&self.assignee_usernames, &assignee.username))
    }
}

fn normalize(values: Vec<String>) -> Vec<String> {
    let mut values = values
        .into_iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values
}

fn contains(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(needle)
}

fn matches_any(allowed_values: &[String], candidate: &str) -> bool {
    if allowed_values.is_empty() {
        return true;
    }

    let candidate = candidate.to_lowercase();
    allowed_values
        .iter()
        .any(|allowed_value| allowed_value == &candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Assignee;

    //-------------//
    //  SUCCESSES  //
    //-------------//

    #[test]
    fn query_matches_each_searchable_field() -> anyhow::Result<()> {
        // GIVEN
        let issue = test_issue();
        let test_cases = [
            ("key", "PROJ-42"),
            ("summary", "connection timeout"),
            ("description", "VPN reconnect"),
        ];

        for (name, query) in test_cases {
            let filter =
                IssueFilter::new(Some(query.to_owned()), Vec::new(), Vec::new(), Vec::new())?;

            // WHEN
            let result = filter.matches(&issue);

            // THEN
            assert!(result, "test case: {name}");
        }

        Ok(())
    }

    #[test]
    fn query_is_trimmed_and_matched_case_insensitively() -> anyhow::Result<()> {
        // GIVEN
        let filter = IssueFilter::new(
            Some("  ÜBERPRÜFEN  ".to_owned()),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )?;
        let mut issue = test_issue();
        issue.summary = "Verbindung überprüfen".to_owned();

        // WHEN
        let result = filter.matches(&issue);

        // THEN
        assert!(result);

        Ok(())
    }

    #[test]
    fn query_treats_special_characters_literally() -> anyhow::Result<()> {
        // GIVEN
        let filter =
            IssueFilter::new(Some("[v2]+".to_owned()), Vec::new(), Vec::new(), Vec::new())?;
        let test_cases = [
            ("literal match", "Support parser [v2]+", true),
            ("regex-like match", "Support parser v222", false),
        ];

        for (name, summary, expected) in test_cases {
            let mut issue = test_issue();
            issue.summary = summary.to_owned();

            // WHEN
            let result = filter.matches(&issue);

            // THEN
            assert_eq!(result, expected, "test case: {name}");
        }

        Ok(())
    }

    #[test]
    fn values_within_each_category_are_combined_with_or() -> anyhow::Result<()> {
        // GIVEN
        let issue = test_issue();
        let test_cases = [
            (
                "assignee",
                IssueFilter::new(None, strings(&["aaron", "alice"]), Vec::new(), Vec::new())?,
            ),
            (
                "status",
                IssueFilter::new(
                    None,
                    Vec::new(),
                    strings(&["Closed", "In Progress"]),
                    Vec::new(),
                )?,
            ),
            (
                "issue type",
                IssueFilter::new(
                    None,
                    Vec::new(),
                    Vec::new(),
                    strings(&["Assignment", "Bug"]),
                )?,
            ),
        ];

        for (name, filter) in test_cases {
            // WHEN
            let result = filter.matches(&issue);

            // THEN
            assert!(result, "test case: {name}");
        }

        Ok(())
    }

    #[test]
    fn all_filter_categories_match_together() -> anyhow::Result<()> {
        // GIVEN
        let filter = IssueFilter::new(
            Some("timeout".to_owned()),
            strings(&["ALICE"]),
            strings(&["IN PROGRESS"]),
            strings(&["BUG"]),
        )?;
        let issue = test_issue();

        // WHEN
        let result = filter.matches(&issue);

        // THEN
        assert!(result);

        Ok(())
    }

    #[test]
    fn categorical_values_are_normalized_and_deduplicated() -> anyhow::Result<()> {
        // GIVEN
        let assignee_usernames = strings(&["bob", "ALICE", "alice"]);

        // WHEN
        let result = IssueFilter::new(None, assignee_usernames, Vec::new(), Vec::new())?;

        // THEN
        assert_eq!(result.assignee_usernames, strings(&["alice", "bob"]));

        Ok(())
    }

    #[test]
    fn query_split_across_fields_does_not_match() -> anyhow::Result<()> {
        // GIVEN
        let filter = IssueFilter::new(
            Some("connection timeout".to_owned()),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )?;
        let mut issue = test_issue();
        issue.summary = "Connection failures".to_owned();
        issue.description = Some("Investigate the timeout".to_owned());

        // WHEN
        let result = filter.matches(&issue);

        // THEN
        assert!(!result);

        Ok(())
    }

    #[test]
    fn query_does_not_match_an_issue_without_a_description() -> anyhow::Result<()> {
        // GIVEN
        let filter = IssueFilter::new(
            Some("release notes".to_owned()),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )?;
        let mut issue = test_issue();
        issue.description = None;

        // WHEN
        let result = filter.matches(&issue);

        // THEN
        assert!(!result);

        Ok(())
    }

    #[test]
    fn categorical_filter_requires_an_exact_match() -> anyhow::Result<()> {
        // GIVEN
        let filter = IssueFilter::new(None, Vec::new(), strings(&["Progress"]), Vec::new())?;
        let issue = test_issue();

        // WHEN
        let result = filter.matches(&issue);

        // THEN
        assert!(!result);

        Ok(())
    }

    #[test]
    fn different_filter_categories_are_combined_with_and() -> anyhow::Result<()> {
        // GIVEN
        let filter = IssueFilter::new(
            Some("timeout".to_owned()),
            strings(&["alice"]),
            strings(&["Open"]),
            strings(&["Bug"]),
        )?;
        let issue = test_issue();

        // WHEN
        let result = filter.matches(&issue);

        // THEN
        assert!(!result);

        Ok(())
    }

    #[test]
    fn assignee_filter_does_not_match_the_display_name() -> anyhow::Result<()> {
        // GIVEN
        let filter = IssueFilter::new(None, strings(&["Alice Example"]), Vec::new(), Vec::new())?;
        let issue = test_issue();

        // WHEN
        let result = filter.matches(&issue);

        // THEN
        assert!(!result);

        Ok(())
    }

    #[test]
    fn assignee_filter_does_not_match_an_unassigned_issue() -> anyhow::Result<()> {
        // GIVEN
        let filter = IssueFilter::new(None, strings(&["alice"]), Vec::new(), Vec::new())?;
        let mut issue = test_issue();
        issue.assignee = None;

        // WHEN
        let result = filter.matches(&issue);

        // THEN
        assert!(!result);

        Ok(())
    }

    //------------//
    //  FAILURES  //
    //------------//

    #[test]
    fn filter_without_criteria_cannot_be_created() {
        // GIVEN
        // WHEN
        let result = IssueFilter::new(None, Vec::new(), Vec::new(), Vec::new());

        // THEN
        assert!(matches!(result, Err(IssueFilterError::NoCriteria)));
    }

    #[test]
    fn empty_query_fails() {
        // GIVEN
        let test_cases = [("empty", ""), ("whitespace only", "   ")];

        for (name, query) in test_cases {
            // WHEN
            let result =
                IssueFilter::new(Some(query.to_owned()), Vec::new(), Vec::new(), Vec::new());

            // THEN
            assert!(
                matches!(result, Err(IssueFilterError::EmptyQuery)),
                "test case: {name}"
            );
        }
    }

    fn test_issue() -> Issue {
        Issue {
            id: "10042".to_owned(),
            key: "PROJ-42".to_owned(),
            summary: "Investigate connection timeout".to_owned(),
            status: "In Progress".to_owned(),
            issue_type: "Bug".to_owned(),
            assignee: Some(Assignee {
                username: "alice".to_owned(),
                display_name: "Alice Example".to_owned(),
            }),
            description: Some("Failures occur after a VPN reconnect.".to_owned()),
            updated_at: "2026-09-15T10:00:00.000+0000".to_owned(),
            parent: None,
            subtasks: Vec::new(),
        }
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }
}
