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
    #[error("'{property}' must not be empty")]
    EmptyValue { property: &'static str },
}

impl IssueFilter {
    pub fn new(
        query: Option<String>,
        assignee_usernames: Vec<String>,
        statuses: Vec<String>,
        issue_types: Vec<String>,
    ) -> Result<Self, IssueFilterError> {
        let query = match query {
            Some(query) => {
                let query = normalize_filter_value(&query);
                if query.is_empty() {
                    return Err(IssueFilterError::EmptyValue { property: "query" });
                }
                Some(query)
            }
            None => None,
        };

        Ok(Self {
            query,
            assignee_usernames: normalize_values(assignee_usernames, "assignee")?,
            statuses: normalize_values(statuses, "status")?,
            issue_types: normalize_values(issue_types, "issue type")?,
        })
    }

    pub fn matches(&self, issue: &Issue) -> bool {
        self.matches_query(issue)
            && self.matches_assignee(issue)
            && matches_any(&self.statuses, &issue.status)
            && matches_any(&self.issue_types, &issue.issue_type)
    }

    pub fn is_unconstrained(&self) -> bool {
        self.query.is_none()
            && self.assignee_usernames.is_empty()
            && self.statuses.is_empty()
            && self.issue_types.is_empty()
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

fn normalize_values(
    values: Vec<String>,
    property: &'static str,
) -> Result<Vec<String>, IssueFilterError> {
    let mut values = values
        .into_iter()
        .map(|value| normalize_filter_value(&value))
        .collect::<Vec<_>>();

    if values.iter().any(String::is_empty) {
        return Err(IssueFilterError::EmptyValue { property });
    }

    values.sort_unstable();
    values.dedup();
    Ok(values)
}

fn normalize_filter_value(value: &str) -> String {
    value.trim().to_lowercase()
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
        .any(|allowed_value| candidate.contains(allowed_value))
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
            ("summary", "Investigate connection timeout"),
            ("description", "Failures occur after a VPN reconnect."),
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
    fn supplied_search_values_are_trimmed_and_matched_case_insensitively() -> anyhow::Result<()> {
        // GIVEN
        let test_cases = [
            (
                "query",
                IssueFilter::new(
                    Some("  CONNECTION TIMEOUT  ".to_owned()),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )?,
            ),
            (
                "assignee",
                IssueFilter::new(None, strings(&["  ALICE  "]), Vec::new(), Vec::new())?,
            ),
            (
                "status",
                IssueFilter::new(None, Vec::new(), strings(&["  PROGRESS  "]), Vec::new())?,
            ),
            (
                "issue type",
                IssueFilter::new(None, Vec::new(), Vec::new(), strings(&["  STORY  "]))?,
            ),
        ];
        let issue = test_issue();

        for (name, filter) in test_cases {
            // WHEN
            let result = filter.matches(&issue);

            // THEN
            assert!(result, "test case: {name}");
        }

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
                    strings(&["Assignment", "Story"]),
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
    fn search_values_match_substrings() -> anyhow::Result<()> {
        // GIVEN
        let test_cases = [
            (
                "query",
                IssueFilter::new(
                    Some("connection timeout".to_owned()),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )?,
            ),
            (
                "assignee",
                IssueFilter::new(None, strings(&["alice"]), Vec::new(), Vec::new())?,
            ),
            (
                "status",
                IssueFilter::new(None, Vec::new(), strings(&["Progress"]), Vec::new())?,
            ),
            (
                "issue type",
                IssueFilter::new(None, Vec::new(), Vec::new(), strings(&["Story"]))?,
            ),
        ];
        let issue = test_issue();

        for (name, filter) in test_cases {
            // WHEN
            let result = filter.matches(&issue);

            // THEN
            assert!(result, "test case: {name}");
        }

        Ok(())
    }

    #[test]
    fn search_criteria_are_combined_with_and() -> anyhow::Result<()> {
        // GIVEN
        let test_cases = [
            (
                "all categories match",
                IssueFilter::new(
                    Some("Investigate connection timeout".to_owned()),
                    strings(&["alice.smith"]),
                    strings(&["In Progress"]),
                    strings(&["Technical Story"]),
                )?,
                true,
            ),
            (
                "query doesn't match",
                IssueFilter::new(
                    Some("database".to_owned()),
                    strings(&["alice.smith"]),
                    strings(&["In Progress"]),
                    strings(&["Technical Story"]),
                )?,
                false,
            ),
            (
                "assignee doesn't match",
                IssueFilter::new(
                    Some("Investigate connection timeout".to_owned()),
                    strings(&["bob"]),
                    strings(&["In Progress"]),
                    strings(&["Technical Story"]),
                )?,
                false,
            ),
            (
                "status doesn't match",
                IssueFilter::new(
                    Some("Investigate connection timeout".to_owned()),
                    strings(&["alice.smith"]),
                    strings(&["Done"]),
                    strings(&["Technical Story"]),
                )?,
                false,
            ),
            (
                "issue type doesn't match",
                IssueFilter::new(
                    Some("Investigate connection timeout".to_owned()),
                    strings(&["alice.smith"]),
                    strings(&["In Progress"]),
                    strings(&["Bug"]),
                )?,
                false,
            ),
        ];
        let issue = test_issue();

        for (name, filter, expected) in test_cases {
            // WHEN
            let result = filter.matches(&issue);

            // THEN
            assert_eq!(result, expected, "test case: {name}");
        }

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

    #[test]
    fn filter_without_criteria_matches_an_issue() -> anyhow::Result<()> {
        // GIVEN
        let filter = IssueFilter::new(None, Vec::new(), Vec::new(), Vec::new())?;
        let issue = test_issue();

        // WHEN
        let result = filter.matches(&issue);

        // THEN
        assert!(result);

        Ok(())
    }

    //------------//
    //  FAILURES  //
    //------------//

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
                matches!(
                    result,
                    Err(IssueFilterError::EmptyValue { property: "query" })
                ),
                "test case: {name}"
            );
        }
    }

    #[test]
    fn empty_categorical_value_fails() {
        // GIVEN
        let test_cases = [
            (
                "empty assignee",
                "assignee",
                strings(&["alice", ""]),
                Vec::new(),
                Vec::new(),
            ),
            (
                "whitespace-only assignee",
                "assignee",
                strings(&["alice", "   "]),
                Vec::new(),
                Vec::new(),
            ),
            (
                "empty status",
                "status",
                Vec::new(),
                strings(&["Open", ""]),
                Vec::new(),
            ),
            (
                "whitespace-only status",
                "status",
                Vec::new(),
                strings(&["Open", "   "]),
                Vec::new(),
            ),
            (
                "empty issue type",
                "issue type",
                Vec::new(),
                Vec::new(),
                strings(&["Bug", ""]),
            ),
            (
                "whitespace-only issue type",
                "issue type",
                Vec::new(),
                Vec::new(),
                strings(&["Bug", "   "]),
            ),
        ];

        for (name, expected_property, assignees, statuses, issue_types) in test_cases {
            // WHEN
            let result = IssueFilter::new(None, assignees, statuses, issue_types);

            // THEN
            assert!(
                matches!(
                    result,
                    Err(IssueFilterError::EmptyValue { property })
                        if property == expected_property
                ),
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
            issue_type: "Technical Story".to_owned(),
            assignee: Some(Assignee {
                username: "alice.smith".to_owned(),
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
