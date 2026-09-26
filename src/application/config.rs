use crate::paths;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

const SAMPLE_CONFIG: &str = include_str!("assets/sample-config.toml");
const VALID_CONFIG_MESSAGE: &str = "Configuration is valid.\n";

#[derive(Debug, thiserror::Error)]
pub enum InitConfigError {
    #[error(transparent)]
    Paths(#[from] paths::PathsError),

    #[error("couldn't create configuration directory at {path:?}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("configuration already exists at {path:?}")]
    AlreadyExists { path: PathBuf },

    #[error("couldn't create configuration at {path:?}")]
    Create {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("couldn't write configuration at {path:?}")]
    WriteConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("couldn't write configuration path to stdout")]
    WriteOutput(#[source] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum SampleConfigError {
    #[error("couldn't write sample configuration to stdout")]
    Write(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ValidateConfigError {
    #[error(transparent)]
    Paths(#[from] paths::PathsError),

    #[error("configuration is invalid")]
    Invalid(#[from] crate::config::ConfigError),

    #[error("couldn't write configuration validation result to stdout")]
    Write(#[from] std::io::Error),
}

pub fn init() -> Result<(), InitConfigError> {
    let config_path = paths::get()?.config;

    if let Some(config_dir) = config_path.parent() {
        std::fs::create_dir_all(config_dir).map_err(|source| InitConfigError::CreateDirectory {
            path: config_dir.to_path_buf(),
            source,
        })?;
    }

    let mut config_file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config_path)
    {
        Ok(file) => file,
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(InitConfigError::AlreadyExists { path: config_path });
        }
        Err(source) => {
            return Err(InitConfigError::Create {
                path: config_path,
                source,
            });
        }
    };

    config_file
        .write_all(SAMPLE_CONFIG.as_bytes())
        .map_err(|source| InitConfigError::WriteConfig {
            path: config_path.clone(),
            source,
        })?;

    match writeln!(
        std::io::stdout().lock(),
        "Created sample configuration at {}.

Edit it to match your Jira setup.",
        config_path.display()
    ) {
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(InitConfigError::WriteOutput),
    }
}

pub fn sample() -> Result<(), SampleConfigError> {
    match std::io::stdout().lock().write_all(SAMPLE_CONFIG.as_bytes()) {
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(SampleConfigError::Write),
    }
}

pub fn validate(config_path: Option<PathBuf>) -> Result<(), ValidateConfigError> {
    let config_path = match config_path {
        Some(path) => path,
        None => paths::get()?.config,
    };

    crate::config::load(&config_path)?;

    match std::io::stdout()
        .lock()
        .write_all(VALID_CONFIG_MESSAGE.as_bytes())
    {
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(ValidateConfigError::Write),
    }
}
