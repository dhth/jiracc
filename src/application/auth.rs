use crate::config;
use crate::jira::{JiraClient, JiraClientError};
use crate::paths;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CheckAuthError {
    #[error(transparent)]
    Paths(#[from] paths::PathsError),

    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error("couldn't check Jira authentication")]
    Jira(#[from] JiraClientError),

    #[error("couldn't write result to stdout")]
    WriteOutput(#[from] std::io::Error),
}

pub async fn check(config_path: Option<PathBuf>) -> Result<(), CheckAuthError> {
    let config_path = match config_path {
        Some(path) => path,
        None => paths::get()?.config,
    };
    let loaded = config::load(&config_path)?;
    let jira = loaded.config.jira;
    let client = JiraClient::new(&jira.url, &jira.token)?;
    let user = client.get_current_user().await?;

    writeln!(
        std::io::stdout().lock(),
        "Authenticated to {} as {} ({}).",
        jira.url.as_str(),
        user.display_name,
        user.username
    )?;

    Ok(())
}
