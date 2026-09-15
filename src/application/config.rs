use std::io::{self, Write};

const SAMPLE_CONFIG: &str = include_str!("assets/sample-config.toml");

#[derive(Debug, thiserror::Error)]
pub enum SampleConfigError {
    #[error("couldn't write sample configuration to stdout")]
    Write(#[source] io::Error),
}

pub fn sample() -> Result<(), SampleConfigError> {
    io::stdout()
        .lock()
        .write_all(SAMPLE_CONFIG.as_bytes())
        .map_err(SampleConfigError::Write)
}
