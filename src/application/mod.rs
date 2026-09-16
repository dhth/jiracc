mod config;
mod sync;

use std::path::PathBuf;

pub use config::{InitConfigError, SampleConfigError, ValidateConfigError};
pub use sync::IssueFetcher;
pub use sync::sync;

pub enum Command {
    Config(ConfigCommand),
}

pub enum ConfigCommand {
    Init,
    Sample,
    Validate { config_path: Option<PathBuf> },
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error(transparent)]
    InitConfig(#[from] InitConfigError),

    #[error(transparent)]
    SampleConfig(#[from] SampleConfigError),

    #[error(transparent)]
    ValidateConfig(#[from] ValidateConfigError),
}

pub async fn run(command: Command) -> Result<(), ApplicationError> {
    match command {
        Command::Config(ConfigCommand::Init) => config::init()?,
        Command::Config(ConfigCommand::Sample) => config::sample()?,
        Command::Config(ConfigCommand::Validate { config_path }) => config::validate(config_path)?,
    }

    Ok(())
}
