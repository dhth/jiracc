mod config;
mod sync;

pub use config::SampleConfigError;
pub use sync::IssueFetcher;
pub use sync::sync;

pub enum Command {
    Config(ConfigCommand),
}

pub enum ConfigCommand {
    Sample,
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error(transparent)]
    SampleConfig(#[from] SampleConfigError),
}

pub async fn run(command: Command) -> Result<(), ApplicationError> {
    match command {
        Command::Config(ConfigCommand::Sample) => config::sample()?,
    }

    Ok(())
}
