mod cli;

use clap::Parser;
use jiracc::application;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args = cli::Args::parse();
    let result = application::run(args.into()).await;

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let presentation = error.presentation();
            let report = anyhow::Error::new(error);
            eprintln!("Error: {report:?}");
            if let Some(follow_up) = presentation.follow_up {
                eprintln!(
                    "
{follow_up}"
                );
            }
            if presentation.unexpected {
                eprintln!(
                    "
---

This error is unexpected. Please check if there's an open issue for this on https://github.com/dhth/jiracc/issues. Create one if it doesn't exist."
                );
            }
            ExitCode::FAILURE
        }
    }
}
