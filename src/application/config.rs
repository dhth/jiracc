use crate::{config as configuration, paths};
use std::io::{self, Write};
use std::path::PathBuf;

const SAMPLE_CONFIG: &str = include_str!("assets/sample-config.toml");
const VALID_CONFIG_MESSAGE: &str = "Configuration is valid.\n";

#[derive(Debug, thiserror::Error)]
pub enum SampleConfigError {
    #[error("couldn't write sample configuration to stdout")]
    Write(#[source] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ValidateConfigError {
    #[error(transparent)]
    Paths(#[from] paths::PathsError),

    #[error("configuration is invalid")]
    Invalid(#[source] configuration::ConfigError),

    #[error("couldn't write configuration validation result to stdout")]
    Write(#[source] io::Error),
}

pub fn sample() -> Result<(), SampleConfigError> {
    io::stdout()
        .lock()
        .write_all(SAMPLE_CONFIG.as_bytes())
        .map_err(SampleConfigError::Write)
}

pub fn validate(config_path: Option<PathBuf>) -> Result<(), ValidateConfigError> {
    let config_path = match config_path {
        Some(path) => path,
        None => paths::get()?.config,
    };

    configuration::load(&config_path).map_err(ValidateConfigError::Invalid)?;

    io::stdout()
        .lock()
        .write_all(VALID_CONFIG_MESSAGE.as_bytes())
        .map_err(ValidateConfigError::Write)
}
