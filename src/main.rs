use jiracc::application;
use jiracc::config;
use jiracc::jira::JiraClient;
use std::error::Error;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = config::load(Path::new("jiracc.toml"))?;
    let jira_client = JiraClient::new(&config.jira.url, &config.jira.token);

    application::sync(&jira_client, &config.jira.jql).await?;

    Ok(())
}
