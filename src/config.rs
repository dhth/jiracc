use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Config {
    pub jira: JiraConfig,
}

#[derive(Debug)]
pub struct JiraConfig {
    pub url: JiraUrl,
    pub token: JiraToken,
    pub jql: JiraJql,
}

#[derive(Debug)]
pub struct JiraUrl(url::Url);

impl JiraUrl {
    pub fn as_url(&self) -> &url::Url {
        &self.0
    }
}

pub struct JiraToken(String);

impl JiraToken {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for JiraToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("JiraToken([REDACTED])")
    }
}

#[derive(Debug)]
pub struct JiraJql(String);

impl JiraJql {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read configuration from {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse configuration")]
    Parse(#[source] toml::de::Error),

    #[error("environment variable {variable} referenced by {field} is unavailable")]
    EnvironmentVariable {
        field: String,
        variable: String,
        #[source]
        source: std::env::VarError,
    },

    #[error("jira.url is not a valid URL")]
    InvalidJiraUrl(#[source] url::ParseError),

    #[error("jira.url must use http or https, not {scheme}")]
    UnsupportedJiraUrlScheme { scheme: String },

    #[error("jira.url must not contain a query or fragment")]
    JiraUrlContainsQueryOrFragment,

    #[error("jira.token must not be empty")]
    EmptyJiraToken,

    #[error("jira.jql must not be empty")]
    EmptyJiraJql,
}

#[derive(Deserialize)]
struct ConfigFile {
    jira: JiraConfigFile,
}

#[derive(Deserialize)]
struct JiraConfigFile {
    url: String,
    token: String,
    jql: String,
}

pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    parse(&contents, |variable| std::env::var(variable).map(Some))
}

fn parse(
    contents: &str,
    mut environment: impl FnMut(&str) -> Result<Option<String>, std::env::VarError>,
) -> Result<Config, ConfigError> {
    let config_file = toml::from_str::<ConfigFile>(contents).map_err(ConfigError::Parse)?;
    let ConfigFile { jira } = config_file;
    let expanded = ConfigFile {
        jira: JiraConfigFile {
            url: expand_environment_variables(jira.url, "jira.url", &mut environment)?,
            token: expand_environment_variables(jira.token, "jira.token", &mut environment)?,
            jql: expand_environment_variables(jira.jql, "jira.jql", &mut environment)?,
        },
    };

    Config::try_from(expanded)
}

impl TryFrom<ConfigFile> for Config {
    type Error = ConfigError;

    fn try_from(config: ConfigFile) -> Result<Self, Self::Error> {
        let ConfigFile { jira } = config;

        Ok(Self {
            jira: JiraConfig {
                url: JiraUrl::try_from(jira.url)?,
                token: JiraToken::try_from(jira.token)?,
                jql: JiraJql::try_from(jira.jql)?,
            },
        })
    }
}

impl TryFrom<String> for JiraUrl {
    type Error = ConfigError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut url = url::Url::parse(&value).map_err(ConfigError::InvalidJiraUrl)?;

        if !matches!(url.scheme(), "http" | "https") {
            return Err(ConfigError::UnsupportedJiraUrlScheme {
                scheme: url.scheme().to_owned(),
            });
        }
        if url.query().is_some() || url.fragment().is_some() {
            return Err(ConfigError::JiraUrlContainsQueryOrFragment);
        }

        let path = url.path().trim_end_matches('/');
        let canonical_path = if path.is_empty() {
            "/".to_owned()
        } else {
            format!("{path}/")
        };
        url.set_path(&canonical_path);

        Ok(Self(url))
    }
}

impl TryFrom<String> for JiraToken {
    type Error = ConfigError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(ConfigError::EmptyJiraToken);
        }

        Ok(Self(value))
    }
}

impl TryFrom<String> for JiraJql {
    type Error = ConfigError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(ConfigError::EmptyJiraJql);
        }

        Ok(Self(value))
    }
}

fn expand_environment_variables(
    value: String,
    field: &str,
    environment: &mut impl FnMut(&str) -> Result<Option<String>, std::env::VarError>,
) -> Result<String, ConfigError> {
    shellexpand::env_with_context(&value, environment)
        .map(|expanded| expanded.into_owned())
        .map_err(|error| ConfigError::EnvironmentVariable {
            field: field.to_owned(),
            variable: error.var_name,
            source: error.cause,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    //-------------//
    //  SUCCESSES  //
    //-------------//

    #[test]
    fn parsing_correct_config_works() -> anyhow::Result<()> {
        // GIVEN
        let config_str = r#"
[jira]
url = "https://jira.example.com/jira"
token = "secret-token"
jql = "project = TEST"
"#;

        // WHEN
        let result = parse(config_str, test_environment)?;

        // THEN
        assert_eq!(
            result.jira.url.as_url().as_str(),
            "https://jira.example.com/jira/"
        );
        assert_eq!(result.jira.token.as_str(), "secret-token");
        assert_eq!(result.jira.jql.as_str(), "project = TEST");

        Ok(())
    }

    #[test]
    fn parsing_config_with_environment_variables_works() -> anyhow::Result<()> {
        // GIVEN
        let config_str = r#"
[jira]
url = "https://${HOST}/jira"
token = "${TOKEN}"
jql = "project = ${PROJECT}"
"#;

        // WHEN
        let result = parse(config_str, test_environment)?;

        // THEN
        assert_eq!(
            result.jira.url.as_url().as_str(),
            "https://jira.example.com/jira/"
        );
        assert_eq!(result.jira.token.as_str(), "secret-token");
        assert_eq!(result.jira.jql.as_str(), "project = TEST");

        Ok(())
    }

    #[test]
    fn expanding_supported_environment_variable_syntax_works() -> anyhow::Result<()> {
        // GIVEN
        let test_cases = [
            ("no variables", "literal", "literal"),
            ("unbraced variable", "$HOST", "jira.example.com"),
            ("braced variable", "${HOST}", "jira.example.com"),
            (
                "embedded variables",
                "https://${HOST}/${PROJECT}",
                "https://jira.example.com/TEST",
            ),
            ("default value", "${MISSING:-fallback}", "fallback"),
        ];

        for (name, input, expected) in test_cases {
            let mut environment = test_environment;

            // WHEN
            let result =
                expand_environment_variables(input.to_owned(), "jira.url", &mut environment)?;

            // THEN
            assert_eq!(result, expected, "test case: {name}");
        }

        Ok(())
    }

    //------------//
    //  FAILURES  //
    //------------//

    #[test]
    fn parsing_invalid_toml_fails() {
        // GIVEN
        let config_str = r#"
[jira]
url   "https://jira.example.com"
"#;

        // WHEN
        let result = parse(config_str, test_environment);

        // THEN
        assert!(matches!(result, Err(ConfigError::Parse(_))));
    }

    #[test]
    fn parsing_config_with_missing_fields_fails() {
        // GIVEN
        let config_str = r#"
[jira]
url = "https://jira.example.com"
token = "secret-token"
"#;

        // WHEN
        let result = parse(config_str, test_environment);

        // THEN
        assert!(matches!(result, Err(ConfigError::Parse(_))));
    }

    #[test]
    fn parsing_config_with_invalid_jira_url_fails() {
        // GIVEN
        let test_cases = [
            (
                "malformed URL",
                r#"
[jira]
url = "not a URL"
token = "secret-token"
jql = "project = TEST"
"#,
                "jira.url is not a valid URL",
            ),
            (
                "unsupported scheme",
                r#"
[jira]
url = "ftp://jira.example.com"
token = "secret-token"
jql = "project = TEST"
"#,
                "jira.url must use http or https, not ftp",
            ),
            (
                "query",
                r#"
[jira]
url = "https://jira.example.com?view=all"
token = "secret-token"
jql = "project = TEST"
"#,
                "jira.url must not contain a query or fragment",
            ),
            (
                "fragment",
                r#"
[jira]
url = "https://jira.example.com#issues"
token = "secret-token"
jql = "project = TEST"
"#,
                "jira.url must not contain a query or fragment",
            ),
        ];

        for (name, config_str, expected) in test_cases {
            // WHEN
            let result = parse(config_str, test_environment);

            // THEN
            assert_eq!(
                result.err().map(|error| error.to_string()).as_deref(),
                Some(expected),
                "test case: {name}"
            );
        }
    }

    #[test]
    fn parsing_config_with_empty_values_fails() {
        // GIVEN
        let test_cases = [
            (
                "empty token",
                r#"
[jira]
url = "https://jira.example.com"
token = ""
jql = "project = TEST"
"#,
                "jira.token must not be empty",
            ),
            (
                "whitespace-only token",
                r#"
[jira]
url = "https://jira.example.com"
token = "   "
jql = "project = TEST"
"#,
                "jira.token must not be empty",
            ),
            (
                "empty JQL",
                r#"
[jira]
url = "https://jira.example.com"
token = "secret-token"
jql = ""
"#,
                "jira.jql must not be empty",
            ),
            (
                "whitespace-only JQL",
                r#"
[jira]
url = "https://jira.example.com"
token = "secret-token"
jql = "   "
"#,
                "jira.jql must not be empty",
            ),
        ];

        for (name, config_str, expected) in test_cases {
            // WHEN
            let result = parse(config_str, test_environment);

            // THEN
            assert_eq!(
                result.err().map(|error| error.to_string()).as_deref(),
                Some(expected),
                "test case: {name}"
            );
        }
    }

    #[test]
    fn expanding_a_missing_environment_variable_fails() {
        // GIVEN
        let mut environment = test_environment;

        // WHEN
        let result =
            expand_environment_variables("${MISSING}".to_owned(), "jira.jql", &mut environment);

        // THEN
        assert!(matches!(
            result,
            Err(ConfigError::EnvironmentVariable {
                field,
                variable,
                ..
            }) if field == "jira.jql" && variable == "MISSING"
        ));
    }

    fn test_environment(variable: &str) -> Result<Option<String>, std::env::VarError> {
        match variable {
            "HOST" => Ok(Some("jira.example.com".to_owned())),
            "TOKEN" => Ok(Some("secret-token".to_owned())),
            "PROJECT" => Ok(Some("TEST".to_owned())),
            _ => Err(std::env::VarError::NotPresent),
        }
    }
}
