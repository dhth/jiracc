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
            let report = anyhow::Error::new(error);
            eprintln!("Error: {report:?}");
            ExitCode::FAILURE
        }
    }
}
