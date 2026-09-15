use etcetera::{BaseStrategy, choose_base_strategy};
use std::path::PathBuf;

pub struct Paths {
    pub config: PathBuf,
    pub data: PathBuf,
}

#[derive(Debug, thiserror::Error)]
#[error("couldn't determine jiracc's paths")]
pub struct PathsError(#[source] etcetera::HomeDirError);

pub fn get() -> Result<Paths, PathsError> {
    let strategy = choose_base_strategy().map_err(PathsError)?;

    Ok(Paths {
        config: strategy.config_dir().join("jiracc").join("jiracc.toml"),
        data: strategy.data_dir().join("jiracc"),
    })
}
