use crate::config::JiraJql;
use crate::domain::Issue;
use std::error::Error;
use std::future::Future;

pub trait IssueFetcher {
    type Error: Error + Send + Sync + 'static;

    fn fetch_issues(
        &self,
        jql: &JiraJql,
    ) -> impl Future<Output = Result<Vec<Issue>, Self::Error>> + Send;
}

pub async fn sync<F>(fetcher: &F, jql: &JiraJql) -> Result<(), F::Error>
where
    F: IssueFetcher,
{
    let issues = fetcher.fetch_issues(jql).await?;
    println!("Fetched {} issues", issues.len());

    Ok(())
}
